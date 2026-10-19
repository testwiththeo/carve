//! Recording state machine and its typed error.

use std::time::Instant;

use crate::capture::{CaptureError, CaptureRegion};

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
