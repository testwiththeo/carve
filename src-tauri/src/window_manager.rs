use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, Position, Runtime, Size, State};

#[cfg(not(target_os = "linux"))]
use crate::capture::create_platform_capturer;
use crate::capture::{CaptureError, CaptureRegion, MockCapturer, ScreenCapturer};
use crate::export::SessionState;
use crate::storage::{DiskFrameCache, RawFrame};

#[cfg(target_os = "linux")]
use crate::capture::portal_session::PortalStreamInfo;

struct ActiveCapture {
    stop_signal: Arc<AtomicBool>,
    paused_signal: Arc<AtomicBool>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

static ACTIVE_CAPTURE: Mutex<Option<ActiveCapture>> = Mutex::new(None);

/// Errors that can occur during recording session management and overlay manipulation.
///
/// Serialized across the Tauri boundary as a flat, stable payload so the
/// frontend can branch on the machine tag while still showing the message:
/// `{ "kind": "<machine tag>", "message": "<human message>" }`.
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
    #[error("No portal request is currently pending")]
    NotRequestingPortal,
    #[error("Window error: {0}")]
    WindowError(String),
    /// Frame-capture backend failure during session startup or teardown.
    #[error(transparent)]
    Capture(#[from] CaptureError),
    /// Unrecoverable internal failure (poisoned lock, failed runtime task, cache init I/O).
    #[error("Internal error: {0}")]
    Internal(String),
}

impl RecordingError {
    /// Stable machine-readable tag exposed to the frontend as `error.kind`.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::InvalidRegion => "invalid_region",
            Self::AlreadyRecording => "already_recording",
            Self::NotRecording => "not_recording",
            Self::NotActiveRecording => "not_active_recording",
            Self::NotPaused => "not_paused",
            Self::NotRequestingPortal => "not_requesting_portal",
            Self::WindowError(_) => "window_error",
            Self::Capture(_) => "capture",
            Self::Internal(_) => "internal",
        }
    }
}

impl serde::Serialize for RecordingError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("RecordingError", 2)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

/// Represents the active capture lifecycle state machine.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RecordingState {
    #[default]
    Idle,
    RequestingPortal {
        region: CaptureRegion,
    },
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

    /// Transitions to `RequestingPortal`: the user-facing portal prompt is pending
    /// and frames are not flowing yet.
    pub fn begin_portal_request(&mut self, region: CaptureRegion) -> Result<(), RecordingError> {
        if region.width == 0 || region.height == 0 {
            return Err(RecordingError::InvalidRegion);
        }
        if !matches!(self, RecordingState::Idle) {
            return Err(RecordingError::AlreadyRecording);
        }
        *self = RecordingState::RequestingPortal { region };
        Ok(())
    }

    /// Completes a pending portal request and starts recording frames.
    pub fn portal_acquired(&mut self) -> Result<(), RecordingError> {
        match std::mem::replace(self, RecordingState::Idle) {
            RecordingState::RequestingPortal { region } => {
                *self = RecordingState::Recording {
                    region,
                    start_time: Instant::now(),
                };
                Ok(())
            }
            other => {
                *self = other;
                Err(RecordingError::NotRequestingPortal)
            }
        }
    }

    /// Rolls back a pending portal request (user denied or cancelled the prompt).
    pub fn cancel_portal_request(&mut self) -> Result<(), RecordingError> {
        match std::mem::replace(self, RecordingState::Idle) {
            RecordingState::RequestingPortal { .. } => Ok(()),
            other => {
                *self = other;
                Err(RecordingError::NotRequestingPortal)
            }
        }
    }

    /// Stops the current recording and returns the captured region.
    pub fn stop(&mut self) -> Result<CaptureRegion, RecordingError> {
        match *self {
            RecordingState::Idle => Err(RecordingError::NotRecording),
            RecordingState::RequestingPortal { region }
            | RecordingState::Recording { region, .. }
            | RecordingState::Paused { region, .. } => {
                *self = RecordingState::Idle;
                Ok(region)
            }
        }
    }

    /// Pauses an active recording session.
    pub fn pause(&mut self) -> Result<(), RecordingError> {
        match *self {
            RecordingState::Idle => Err(RecordingError::NotRecording),
            RecordingState::RequestingPortal { .. } => Err(RecordingError::NotActiveRecording),
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
            RecordingState::RequestingPortal { .. } => Err(RecordingError::NotPaused),
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

    /// Returns true if the portal prompt is pending (between start and first frames).
    pub fn is_requesting_portal(&self) -> bool {
        matches!(self, RecordingState::RequestingPortal { .. })
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
            RecordingState::RequestingPortal { region } => Some(region),
            RecordingState::Recording { region, .. } => Some(region),
            RecordingState::Paused { region, .. } => Some(region),
        }
    }
}

// -----------------------------------------------------------------------------
// Capture acquisition (portal-aware recording startup)
// -----------------------------------------------------------------------------

/// Which backend a started recording session is capturing frames from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureBackend {
    /// XDG Desktop Portal ScreenCast stream consumed over PipeWire.
    #[cfg(target_os = "linux")]
    PortalPipeWire,
    /// Platform-native capturer (macOS ScreenCaptureKit / Windows Graphics Capture).
    #[cfg(not(target_os = "linux"))]
    Native,
    /// Synthetic mock frames (headless environment or portal unavailable).
    Mock,
}

/// A successfully started recording session. The capturer has already entered
/// the capturing state, so frames are ready to flow.
pub struct StartedCapture {
    pub capturer: Box<dyn ScreenCapturer>,
    pub backend: CaptureBackend,
}

impl std::fmt::Debug for StartedCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StartedCapture")
            .field("backend", &self.backend)
            .field("capturing", &self.capturer.is_capturing())
            .finish()
    }
}

/// How a recording session acquires its capture backend.
pub enum CaptureAcquisition {
    /// Negotiate an XDG Portal ScreenCast session before frames flow (Linux).
    #[cfg(target_os = "linux")]
    Portal(PortalAcquisition),
    /// Instantiate the platform capturer directly (macOS / Windows).
    #[cfg(not(target_os = "linux"))]
    PlatformNative,
    /// No interactive display session is available: record synthetic mock frames.
    Headless,
}

/// Injectable portal negotiation seam: request the ScreenCast session, then
/// connect a stream consumer to the negotiated PipeWire node.
#[cfg(target_os = "linux")]
pub struct PortalAcquisition {
    request: Box<dyn FnOnce() -> Result<PortalStreamInfo, CaptureError> + Send>,
    connect: ConnectFn,
}

#[cfg(target_os = "linux")]
type ConnectFn =
    Box<dyn FnOnce(PortalStreamInfo) -> Result<Box<dyn ScreenCapturer>, CaptureError> + Send>;

#[cfg(target_os = "linux")]
impl PortalAcquisition {
    pub fn new<R, C>(request: R, connect: C) -> Self
    where
        R: FnOnce() -> Result<PortalStreamInfo, CaptureError> + Send + 'static,
        C: FnOnce(PortalStreamInfo) -> Result<Box<dyn ScreenCapturer>, CaptureError>
            + Send
            + 'static,
    {
        Self {
            request: Box::new(request),
            connect: Box::new(connect),
        }
    }

    /// Production wiring: an ashpd portal request (blocking wrapper, safe under
    /// an existing Tokio runtime) feeding a PipeWire stream consumer.
    pub fn platform() -> Self {
        use ashpd::desktop::screencast::CursorMode;
        Self::new(
            || {
                crate::capture::portal_session::request_screencast_session_blocking(
                    CursorMode::Embedded,
                    false,
                )
            },
            |info| {
                let capturer = crate::capture::LinuxPortalCapturer::with_stream_info(info)?;
                Ok(Box::new(capturer) as Box<dyn ScreenCapturer>)
            },
        )
    }
}

fn lock_state(
    state: &Mutex<RecordingState>,
) -> Result<std::sync::MutexGuard<'_, RecordingState>, CaptureError> {
    state
        .lock()
        .map_err(|_| CaptureError::BackendError("Recording state mutex poisoned".to_string()))
}

fn recording_err(err: RecordingError) -> CaptureError {
    CaptureError::BackendError(err.to_string())
}

fn start_mock_capturer(region: CaptureRegion) -> Result<Box<dyn ScreenCapturer>, CaptureError> {
    let mut mock = MockCapturer::new();
    mock.start_capture(region)?;
    Ok(Box::new(mock))
}

/// Rolls the state machine back out of `RequestingPortal` after a failed
/// acquisition. Never panics; a poisoned lock leaves the state untouched.
fn rollback_portal_request(state: &Mutex<RecordingState>) {
    if let Ok(mut guard) = state.lock() {
        let _ = guard.cancel_portal_request();
    }
}

/// Runs the portal-aware recording startup against the shared state machine:
/// `Idle -> RequestingPortal -> Recording`.
///
/// - The `RequestingPortal` phase is observable while the user-facing portal
///   prompt is pending: the state lock is NOT held during acquisition.
/// - A denied or cancelled prompt rolls back to `Idle` and returns a clean
///   `CaptureError::Cancelled` — never a panic, never a wrapped backend error.
/// - A merely unavailable portal (missing service, headless session) degrades
///   gracefully to `MockCapturer` frames.
#[tracing::instrument(skip(state, acquisition))]
pub fn start_recording_session(
    state: &Mutex<RecordingState>,
    region: CaptureRegion,
    acquisition: CaptureAcquisition,
) -> Result<StartedCapture, CaptureError> {
    lock_state(state)?
        .begin_portal_request(region)
        .map_err(recording_err)?;

    let acquired = match acquisition {
        #[cfg(target_os = "linux")]
        CaptureAcquisition::Portal(portal) => acquire_via_portal(portal, region),
        #[cfg(not(target_os = "linux"))]
        CaptureAcquisition::PlatformNative => acquire_native(region),
        CaptureAcquisition::Headless => {
            start_mock_capturer(region).map(|c| (c, CaptureBackend::Mock))
        }
    };

    match acquired {
        Ok((mut capturer, backend)) => {
            let mut guard = lock_state(state)?;
            match guard.portal_acquired() {
                Ok(()) => Ok(StartedCapture { capturer, backend }),
                Err(err) => {
                    // The session was stopped while the prompt was pending.
                    tracing::warn!(error = %err, "session stopped during portal prompt; stopping capturer");
                    let _ = capturer.stop_capture();
                    Err(recording_err(err))
                }
            }
        }
        Err(err) => {
            tracing::warn!(error = %err, "capture acquisition failed; rolling back portal request");
            rollback_portal_request(state);
            Err(err)
        }
    }
}

#[cfg(target_os = "linux")]
fn acquire_via_portal(
    portal: PortalAcquisition,
    region: CaptureRegion,
) -> Result<(Box<dyn ScreenCapturer>, CaptureBackend), CaptureError> {
    let PortalAcquisition { request, connect } = portal;
    let info = match request() {
        Ok(info) => info,
        // The user denied or dismissed the prompt: surface a clean cancellation
        // instead of silently recording synthetic frames.
        Err(CaptureError::Cancelled) => return Err(CaptureError::Cancelled),
        // No usable portal (missing service, DBus failure, ...): degrade to mock.
        Err(_) => {
            return start_mock_capturer(region).map(|c| (c, CaptureBackend::Mock));
        }
    };
    let mut capturer = connect(info)?;
    capturer.start_capture(region)?;
    Ok((capturer, CaptureBackend::PortalPipeWire))
}

#[cfg(not(target_os = "linux"))]
fn acquire_native(
    region: CaptureRegion,
) -> Result<(Box<dyn ScreenCapturer>, CaptureBackend), CaptureError> {
    let mut capturer = create_platform_capturer();
    if capturer.start_capture(region).is_ok() {
        Ok((capturer, CaptureBackend::Native))
    } else {
        start_mock_capturer(region).map(|c| (c, CaptureBackend::Mock))
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

    // On Linux GTK, setting cursor ignore on an unshown or hidden window panics in tao
    if !overlay.is_visible().unwrap_or(false) {
        return Ok(());
    }

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
    #[tauri::command]
    pub fn set_overlay_clickthrough<R: Runtime>(
        app: AppHandle<R>,
        enabled: bool,
    ) -> Result<(), RecordingError> {
        super::set_overlay_clickthrough(&app, enabled)
    }
}

pub use commands::set_overlay_clickthrough as set_overlay_clickthrough_cmd;
pub use commands::{pause_recording, resume_recording, start_recording, stop_recording};
