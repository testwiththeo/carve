use serde::{Deserialize, Serialize};

/// Represents an uncompressed frame buffer in memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawFrame {
    /// Timestamp in milliseconds from recording start.
    pub timestamp_ms: u64,
    /// Width of the frame in pixels.
    pub width: u32,
    /// Height of the frame in pixels.
    pub height: u32,
    /// Raw frame pixel data in 32-bit BGRA format.
    pub bgra_data: Vec<u8>,
}

impl RawFrame {
    /// Create a new RawFrame.
    pub fn new(timestamp_ms: u64, width: u32, height: u32, bgra_data: Vec<u8>) -> Self {
        Self {
            timestamp_ms,
            width,
            height,
            bgra_data,
        }
    }

    /// Calculate the expected byte size for a 32-bit BGRA buffer (4 bytes per pixel).
    pub fn expected_byte_len(&self) -> usize {
        (self.width as usize) * (self.height as usize) * 4
    }

    /// Validate whether the buffer size matches the dimensions.
    pub fn is_valid_size(&self) -> bool {
        self.bgra_data.len() == self.expected_byte_len()
    }
}
