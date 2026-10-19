//! Thin Tauri glue for the recording lifecycle: command handlers, the active
//! capture loop, and window plumbing. Domain logic lives in [`crate::recording`].

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Manager, Runtime, State};

use crate::capture::{CaptureRegion, MockCapturer, ScreenCapturer};
use crate::export::SessionState;
use crate::storage::{DiskFrameCache, RawFrame};

// `window_manager` remains the public façade for the recording domain:
// integration tests and `lib.rs` import these paths. A `pub use` also brings
// the names into scope for the command glue below.
pub use crate::recording::{
    hide_overlay, set_overlay_clickthrough, show_overlay, start_recording_session,
    CaptureAcquisition, CaptureBackend, PortalAcquisition, RecordingError, RecordingState,
    StartedCapture,
};

struct ActiveCapture {
    stop_signal: Arc<AtomicBool>,
    paused_signal: Arc<AtomicBool>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

static ACTIVE_CAPTURE: Mutex<Option<ActiveCapture>> = Mutex::new(None);

// -----------------------------------------------------------------------------
// Tauri commands
// -----------------------------------------------------------------------------

pub mod commands {
    use super::*;

    /// Starts recording for the specified screen region, shows the overlay, and hides the selector.
    ///
    /// On Linux the portal prompt runs asynchronously: the shared state reports
    /// `RequestingPortal` while the user picks a screen, and only transitions to
    /// `Recording` once the PipeWire stream is live. A denied prompt surfaces a
    /// clean cancelled error; a missing portal degrades to mock frames.
    #[tauri::command]
    pub async fn start_recording<R: Runtime>(
        app: AppHandle<R>,
        state: State<'_, Arc<Mutex<RecordingState>>>,
        session: State<'_, SessionState>,
        region: CaptureRegion,
    ) -> Result<(), RecordingError> {
        let shared_state = state.inner().clone();

        let has_display =
            std::env::var("WAYLAND_DISPLAY").is_ok() || std::env::var("DISPLAY").is_ok();

        let acquisition = if !has_display {
            CaptureAcquisition::Headless
        } else {
            #[cfg(target_os = "linux")]
            {
                CaptureAcquisition::Portal(PortalAcquisition::platform())
            }
            #[cfg(not(target_os = "linux"))]
            {
                CaptureAcquisition::PlatformNative
            }
        };

        // The portal prompt blocks for as long as the user stares at it; keep it
        // off the async runtime while the state machine stays observable.
        let started = tauri::async_runtime::spawn_blocking(move || {
            start_recording_session(&shared_state, region, acquisition)
        })
        .await
        .map_err(|e| RecordingError::Internal(format!("Capture startup task failed: {e}")))??;

        // 1. Initialize SessionState.cache with a DiskFrameCache in a temporary directory
        let session_dir = std::env::temp_dir().join("carve_session");
        let _ = std::fs::create_dir_all(&session_dir);
        let mut disk_cache = DiskFrameCache::new(&session_dir)
            .map_err(|e| RecordingError::Internal(format!("Failed to init frame cache: {e}")))?;
        disk_cache.set_delete_on_drop(false);

        {
            let mut cache_guard = session.cache.lock().map_err(|_| {
                RecordingError::Internal("Failed to lock session cache".to_string())
            })?;
            *cache_guard = Some(disk_cache);
        }

        // 2. Spin up frame capture loop with the live portal-backed capturer
        let stop_signal = Arc::new(AtomicBool::new(false));
        let paused_signal = Arc::new(AtomicBool::new(false));
        let stop_thread = stop_signal.clone();
        let paused_thread = paused_signal.clone();
        let session_cache = session.cache.clone();

        let mut capturer = started.capturer;
        let mut fallback_mock = MockCapturer::new();
        if let Err(e) = fallback_mock.start_capture(region) {
            tracing::warn!(error = %e, "mock fallback capturer failed to start");
        }
        let start_time = Instant::now();

        let thread_handle = std::thread::spawn(move || {
            while !stop_thread.load(Ordering::Relaxed) {
                let loop_start = Instant::now();

                if paused_thread.load(Ordering::Relaxed) {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    continue;
                }

                let frame_opt = capturer.capture_frame().ok().flatten();
                let frame = match frame_opt {
                    Some(f) => f,
                    None => fallback_mock
                        .capture_frame()
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| {
                            let ts = start_time.elapsed().as_millis() as u64;
                            let bgra = vec![128u8; region.expected_byte_len()];
                            RawFrame::new(ts, region.width, region.height, bgra)
                        }),
                };

                if let Ok(mut guard) = session_cache.lock() {
                    if let Some(ref mut c) = *guard {
                        if let Err(e) = c.append_frame(&frame) {
                            tracing::warn!(error = %e, "failed to append captured frame to cache");
                        }
                    }
                }

                let elapsed = loop_start.elapsed();
                let target_interval = std::time::Duration::from_millis(33); // ~30 fps
                if elapsed < target_interval {
                    std::thread::sleep(target_interval - elapsed);
                }
            }

            let _ = capturer.stop_capture();
        });

        if let Ok(mut active_guard) = ACTIVE_CAPTURE.lock() {
            *active_guard = Some(ActiveCapture {
                stop_signal,
                paused_signal,
                thread_handle: Some(thread_handle),
            });
        }

        // Hide selector window if open
        if let Some(selector) = app.get_webview_window("selector") {
            let _ = selector.hide();
        }

        // Show live overlay over the selected region
        show_overlay(&app, region)?;
        tracing::info!(?region, backend = ?started.backend, "recording started");
        Ok(())
    }

    /// Stops recording and hides the overlay. Returns the captured region.
    #[tauri::command]
    pub fn stop_recording<R: Runtime>(
        app: AppHandle<R>,
        state: State<'_, Arc<Mutex<RecordingState>>>,
        _session: State<'_, SessionState>,
    ) -> Result<CaptureRegion, RecordingError> {
        let mut lock = state
            .lock()
            .map_err(|_| RecordingError::Internal("Failed to lock recording state".to_string()))?;
        let region = lock.stop()?;

        // Stop the capturer loop and wait for completion
        if let Ok(mut active_guard) = ACTIVE_CAPTURE.lock() {
            if let Some(mut active) = active_guard.take() {
                active.stop_signal.store(true, Ordering::Relaxed);
                if let Some(handle) = active.thread_handle.take() {
                    let _ = handle.join();
                }
            }
        }

        let _ = hide_overlay(&app);
        tracing::info!(?region, "recording stopped");
        Ok(region)
    }

    /// Pauses active recording.
    #[tauri::command]
    pub fn pause_recording(
        state: State<'_, Arc<Mutex<RecordingState>>>,
    ) -> Result<(), RecordingError> {
        let mut lock = state
            .lock()
            .map_err(|_| RecordingError::Internal("Failed to lock recording state".to_string()))?;
        lock.pause()?;

        if let Ok(active_guard) = ACTIVE_CAPTURE.lock() {
            if let Some(ref active) = *active_guard {
                active.paused_signal.store(true, Ordering::Relaxed);
            }
        }

        Ok(())
    }

    /// Resumes paused recording.
    #[tauri::command]
    pub fn resume_recording(
        state: State<'_, Arc<Mutex<RecordingState>>>,
    ) -> Result<(), RecordingError> {
        let mut lock = state
            .lock()
            .map_err(|_| RecordingError::Internal("Failed to lock recording state".to_string()))?;
        lock.resume()?;

        if let Ok(active_guard) = ACTIVE_CAPTURE.lock() {
            if let Some(ref active) = *active_guard {
                active.paused_signal.store(false, Ordering::Relaxed);
            }
        }

        Ok(())
    }

    /// Configures pointer event clickthrough for the live overlay window.
    ///
    /// JS command name stays `set_overlay_clickthrough` (frontend contract);
    /// the Rust fn is named for the command glue it is.
    #[tauri::command(rename = "set_overlay_clickthrough")]
    pub fn overlay_clickthrough<R: Runtime>(
        app: AppHandle<R>,
        enabled: bool,
    ) -> Result<(), RecordingError> {
        super::set_overlay_clickthrough(&app, enabled)
    }
}

pub use commands::{pause_recording, resume_recording, start_recording, stop_recording};
