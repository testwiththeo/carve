use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, Position, Runtime, Size, State};

use crate::capture::CaptureRegion;

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

    overlay
        .set_position(Position::Physical(PhysicalPosition::new(
            region.x as i32,
            region.y as i32,
        )))
        .map_err(|e| RecordingError::WindowError(e.to_string()))?;

    overlay
        .set_size(Size::Physical(PhysicalSize::new(
            region.width,
            region.height,
        )))
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
        region: CaptureRegion,
    ) -> Result<(), String> {
        let mut lock = state
            .lock()
            .map_err(|_| "Failed to lock recording state".to_string())?;
        lock.start(region).map_err(|e| e.to_string())?;

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
    ) -> Result<CaptureRegion, String> {
        let mut lock = state
            .lock()
            .map_err(|_| "Failed to lock recording state".to_string())?;
        let region = lock.stop().map_err(|e| e.to_string())?;
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
        Ok(())
    }

    /// Resumes paused recording.
    #[tauri::command]
    pub fn resume_recording(state: State<'_, Mutex<RecordingState>>) -> Result<(), String> {
        let mut lock = state
            .lock()
            .map_err(|_| "Failed to lock recording state".to_string())?;
        lock.resume().map_err(|e| e.to_string())?;
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
