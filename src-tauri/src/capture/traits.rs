use serde::{Deserialize, Serialize};

/// Represents a rectangular subregion of a display or buffer to capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl CaptureRegion {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn area(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    pub fn expected_byte_len(&self) -> usize {
        self.area() * 4
    }
}

/// Errors occurring during screen capture or frame cropping operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CaptureError {
    #[error("Invalid source buffer size: expected {expected} bytes, got {actual} bytes")]
    InvalidBufferSize { expected: usize, actual: usize },

    #[error("Region out of bounds: crop region ({x}, {y}, {width}x{height}) exceeds source dimensions ({src_width}x{src_height})")]
    OutOfBounds {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        src_width: u32,
        src_height: u32,
    },

    #[error("Zero-sized capture region: width={width}, height={height}")]
    ZeroSizedRegion { width: u32, height: u32 },

    #[error("Destination buffer size mismatch: expected {expected} bytes, got {actual} bytes")]
    DestinationBufferMismatch { expected: usize, actual: usize },

    #[error("Capture session already active")]
    AlreadyActive,

    #[error("Capture session not active")]
    NotActive,

    #[error("Capture session is paused")]
    Paused,

    #[error("Capture session is not paused")]
    NotPaused,

    #[error("Platform capture backend error: {0}")]
    BackendError(String),
}

/// Trait defining the lifecycle and control operations for screen capture backends.
pub trait ScreenCapturer: Send + Sync {
    /// Start capturing screen frames for the specified region.
    fn start_capture(&mut self, region: CaptureRegion) -> Result<(), CaptureError>;

    /// Stop capturing screen frames and clean up session resources.
    fn stop_capture(&mut self) -> Result<(), CaptureError>;

    /// Pause capture without destroying the underlying capture stream.
    fn pause_capture(&mut self) -> Result<(), CaptureError>;

    /// Resume capturing after being paused.
    fn resume_capture(&mut self) -> Result<(), CaptureError>;

    /// Whether capture is currently running.
    fn is_capturing(&self) -> bool;

    /// Whether capture is currently paused.
    fn is_paused(&self) -> bool;

    /// Optional pull-based frame retrieval for testing or synchronous captures.
    fn capture_frame(&mut self) -> Result<Option<crate::storage::RawFrame>, CaptureError> {
        Ok(None)
    }
}
