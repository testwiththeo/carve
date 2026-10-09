use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, Position, Runtime, Size, State};

use crate::capture::{create_platform_capturer, CaptureRegion, MockCapturer, ScreenCapturer};
use crate::export::SessionState;
use crate::storage::{DiskFrameCache, RawFrame};

struct ActiveCapture {
    stop_signal: Arc<AtomicBool>,
    paused_signal: Arc<AtomicBool>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

static ACTIVE_CAPTURE: Mutex<Option<ActiveCapture>> = Mutex::new(None);

/// Errors that can occur during recording session management and overlay manipulation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RecordingError {
    #[error("Capture region is invalid: width and height must be greater than 0")]
    InvalidRegion,
    #[error("Recording already in progress")]
    AlreadyRecording,
    #[error("No recording is currently in progress")]
    NotRecording,
    #[error("Cannot pause: recording is not active")]
    NotActiveRecording,
    #[error("Cannot resume: recording is not paused")]
    NotPaused,
    #[error("Window error: {0}")]
    WindowError(String),
}

/// Represents the active capture lifecycle state machine.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RecordingState {
    #[default]
    Idle,
    Recording {
        region: CaptureRegion,
        start_time: Instant,
    },
    Paused {
        region: CaptureRegion,
        start_time: Instant,
        paused_at: Instant,
    },
}

impl RecordingState {
    pub fn new() -> Self {
        Self::Idle
    }

    /// Transitions to `Recording` state with the specified capture region.
    pub fn start(&mut self, region: CaptureRegion) -> Result<(), RecordingError> {
        if region.width == 0 || region.height == 0 {
            return Err(RecordingError::InvalidRegion);
        }
        if !matches!(self, RecordingState::Idle) {
            return Err(RecordingError::AlreadyRecording);
        }
        *self = RecordingState::Recording {
            region,
            start_time: Instant::now(),
        };
        Ok(())
    }

    /// Stops the current recording and returns the captured region.
    pub fn stop(&mut self) -> Result<CaptureRegion, RecordingError> {
        match *self {
            RecordingState::Idle => Err(RecordingError::NotRecording),
            RecordingState::Recording { region, .. } | RecordingState::Paused { region, .. } => {
                *self = RecordingState::Idle;
                Ok(region)
            }
        }
    }

    /// Pauses an active recording session.
    pub fn pause(&mut self) -> Result<(), RecordingError> {
        match *self {
            RecordingState::Idle => Err(RecordingError::NotRecording),
            RecordingState::Paused { .. } => Err(RecordingError::NotActiveRecording),
            RecordingState::Recording { region, start_time } => {
                *self = RecordingState::Paused {
                    region,
                    start_time,
                    paused_at: Instant::now(),
                };
                Ok(())
            }
        }
    }

    /// Resumes a paused recording session.
    pub fn resume(&mut self) -> Result<(), RecordingError> {
        match *self {
            RecordingState::Idle => Err(RecordingError::NotRecording),
            RecordingState::Recording { .. } => Err(RecordingError::NotPaused),
            RecordingState::Paused {
                region, start_time, ..
            } => {
                *self = RecordingState::Recording { region, start_time };
                Ok(())
            }
        }
    }

    /// Returns true if a recording session is actively in progress (not paused and not idle).
    pub fn is_recording(&self) -> bool {
        matches!(self, RecordingState::Recording { .. })
    }

    /// Returns true if the recording session is currently paused.
    pub fn is_paused(&self) -> bool {
        matches!(self, RecordingState::Paused { .. })
    }

    /// Returns true if idle (no active recording session).
    pub fn is_idle(&self) -> bool {
        matches!(self, RecordingState::Idle)
    }

    /// Returns a reference to the active `CaptureRegion`, if any.
    pub fn region(&self) -> Option<&CaptureRegion> {
        match self {
            RecordingState::Idle => None,
            RecordingState::Recording { region, .. } => Some(region),
            RecordingState::Paused { region, .. } => Some(region),
        }
    }
}

// -----------------------------------------------------------------------------
// Window control helpers
// -----------------------------------------------------------------------------

/// Positions, sizes, protects from screen capture, and shows the live overlay window.
///
/// Toggles window exclusion:
/// - Windows: SetWindowDisplayAffinity (WDA_EXCLUDEFROMCAPTURE via `set_content_protected(true)`)
/// - macOS: NSWindowSharingNone / SCContentFilter exclusion via `set_content_protected(true)`
pub fn show_overlay<R: Runtime>(
    app: &AppHandle<R>,
    region: CaptureRegion,
) -> Result<(), RecordingError> {
    let overlay = app
        .get_webview_window("overlay")
        .ok_or_else(|| RecordingError::WindowError("Overlay window not found".to_string()))?;

    // HiDPI Scaling: Use PhysicalPosition and PhysicalSize to guarantee 1:1 alignment with
    // physical screen pixels on Retina / 125%+ HiDPI displays without fractional DPI distortion.
    let _scale_factor = overlay.scale_factor().unwrap_or(1.0);
    let phys_pos = PhysicalPosition::new(region.x as i32, region.y as i32);
    let phys_size = PhysicalSize::new(region.width, region.height);

    overlay
        .set_position(Position::Physical(phys_pos))
        .map_err(|e| RecordingError::WindowError(e.to_string()))?;

    overlay
        .set_size(Size::Physical(phys_size))
        .map_err(|e| RecordingError::WindowError(e.to_string()))?;

    // Window exclusion handler for macOS & Windows
    let _ = overlay.set_content_protected(true);

    overlay
        .show()
        .map_err(|e| RecordingError::WindowError(e.to_string()))?;

    overlay
        .set_focus()
        .map_err(|e| RecordingError::WindowError(e.to_string()))?;

    Ok(())
}

/// Hides the live overlay window.
pub fn hide_overlay<R: Runtime>(app: &AppHandle<R>) -> Result<(), RecordingError> {
    let overlay = app
        .get_webview_window("overlay")
        .ok_or_else(|| RecordingError::WindowError("Overlay window not found".to_string()))?;

    overlay
        .hide()
        .map_err(|e| RecordingError::WindowError(e.to_string()))?;

    Ok(())
}

/// Enables or disables pointer clickthrough on the live overlay window.
pub fn set_overlay_clickthrough<R: Runtime>(
    app: &AppHandle<R>,
    enabled: bool,
) -> Result<(), RecordingError> {
    let overlay = app
        .get_webview_window("overlay")
        .ok_or_else(|| RecordingError::WindowError("Overlay window not found".to_string()))?;

    overlay
        .set_ignore_cursor_events(enabled)
        .map_err(|e| RecordingError::WindowError(e.to_string()))?;

    Ok(())
}

// -----------------------------------------------------------------------------
// Tauri commands
// -----------------------------------------------------------------------------

pub mod commands {
    use super::*;

    /// Starts recording for the specified screen region, shows the overlay, and hides the selector.
    #[tauri::command]
    pub fn start_recording<R: Runtime>(
        app: AppHandle<R>,
        state: State<'_, Mutex<RecordingState>>,
        session: State<'_, SessionState>,
        region: CaptureRegion,
    ) -> Result<(), String> {
        let mut lock = state
            .lock()
            .map_err(|_| "Failed to lock recording state".to_string())?;
        lock.start(region).map_err(|e| e.to_string())?;

        // 1. Initialize SessionState.cache with a DiskFrameCache in a temporary directory
        let session_dir = std::env::temp_dir().join("gif_recorder_session");
        let _ = std::fs::create_dir_all(&session_dir);
        let mut disk_cache = DiskFrameCache::new(&session_dir).map_err(|e| e.to_string())?;
        disk_cache.set_delete_on_drop(false);

        {
            let mut cache_guard = session
                .cache
                .lock()
                .map_err(|_| "Failed to lock session cache".to_string())?;
            *cache_guard = Some(disk_cache);
        }

        // 2. Spin up frame capture loop
        let stop_signal = Arc::new(AtomicBool::new(false));
        let paused_signal = Arc::new(AtomicBool::new(false));
        let stop_thread = stop_signal.clone();
        let paused_thread = paused_signal.clone();
        let session_cache = session.cache.clone();

        let has_display =
            std::env::var("WAYLAND_DISPLAY").is_ok() || std::env::var("DISPLAY").is_ok();

        let thread_handle = std::thread::spawn(move || {
            let mut capturer: Box<dyn ScreenCapturer> = if has_display {
                let mut c = create_platform_capturer();
                if c.start_capture(region).is_err() {
                    Box::new(MockCapturer::new())
                } else {
                    c
                }
            } else {
                Box::new(MockCapturer::new())
            };

            let mut fallback_mock = MockCapturer::new();
            let _ = fallback_mock.start_capture(region);
            let start_time = Instant::now();

            while !stop_thread.load(Ordering::Relaxed) {
                let loop_start = Instant::now();

                if paused_thread.load(Ordering::Relaxed) {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    continue;
                }

                let frame_opt = capturer.capture_frame().ok().flatten();
                let frame = match frame_opt {
                    Some(f) => f,
                    None => fallback_mock.capture_frame().ok().flatten().unwrap_or_else(|| {
                        let ts = start_time.elapsed().as_millis() as u64;
                        let bgra = vec![128u8; region.expected_byte_len()];
                        RawFrame::new(ts, region.width, region.height, bgra)
                    }),
                };

                if let Ok(mut guard) = session_cache.lock() {
                    if let Some(ref mut c) = *guard {
                        let _ = c.append_frame(&frame);
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
        show_overlay(&app, region).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Stops recording and hides the overlay. Returns the captured region.
    #[tauri::command]
    pub fn stop_recording<R: Runtime>(
        app: AppHandle<R>,
        state: State<'_, Mutex<RecordingState>>,
        _session: State<'_, SessionState>,
    ) -> Result<CaptureRegion, String> {
        let mut lock = state
            .lock()
            .map_err(|_| "Failed to lock recording state".to_string())?;
        let region = lock.stop().map_err(|e| e.to_string())?;

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
        Ok(region)
    }

    /// Pauses active recording.
    #[tauri::command]
    pub fn pause_recording(state: State<'_, Mutex<RecordingState>>) -> Result<(), String> {
        let mut lock = state
            .lock()
            .map_err(|_| "Failed to lock recording state".to_string())?;
        lock.pause().map_err(|e| e.to_string())?;

        if let Ok(active_guard) = ACTIVE_CAPTURE.lock() {
            if let Some(ref active) = *active_guard {
                active.paused_signal.store(true, Ordering::Relaxed);
            }
        }

        Ok(())
    }

    /// Resumes paused recording.
    #[tauri::command]
    pub fn resume_recording(state: State<'_, Mutex<RecordingState>>) -> Result<(), String> {
        let mut lock = state
            .lock()
            .map_err(|_| "Failed to lock recording state".to_string())?;
        lock.resume().map_err(|e| e.to_string())?;

        if let Ok(active_guard) = ACTIVE_CAPTURE.lock() {
            if let Some(ref active) = *active_guard {
                active.paused_signal.store(false, Ordering::Relaxed);
            }
        }

        Ok(())
    }

    /// Configures pointer event clickthrough for the live overlay window.
    #[tauri::command]
    pub fn set_overlay_clickthrough<R: Runtime>(
        app: AppHandle<R>,
        enabled: bool,
    ) -> Result<(), String> {
        super::set_overlay_clickthrough(&app, enabled).map_err(|e| e.to_string())
    }
}

pub use commands::set_overlay_clickthrough as set_overlay_clickthrough_cmd;
pub use commands::{pause_recording, resume_recording, start_recording, stop_recording};
