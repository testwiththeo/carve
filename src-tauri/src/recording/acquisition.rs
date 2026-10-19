//! Capture acquisition for a recording session: portal negotiation (Linux),
//! platform-native backends, and headless mock fallback.

use std::sync::Mutex;

#[cfg(not(target_os = "linux"))]
use crate::capture::create_platform_capturer;
use crate::capture::{CaptureError, CaptureRegion, MockCapturer, ScreenCapturer};

#[cfg(target_os = "linux")]
use crate::capture::portal_session::PortalStreamInfo;

use super::controller::{RecordingError, RecordingState};

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
