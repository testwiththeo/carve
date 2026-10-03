use crate::capture::traits::{CaptureError, CaptureRegion, ScreenCapturer};
use crate::storage::RawFrame;

/// In-memory mock capturer for testing recording pipelines without physical display hardware.
#[derive(Debug, Clone)]
pub struct MockCapturer {
    capturing: bool,
    paused: bool,
    current_region: Option<CaptureRegion>,
    frame_count: u64,
}

impl MockCapturer {
    pub fn new() -> Self {
        Self {
            capturing: false,
            paused: false,
            current_region: None,
            frame_count: 0,
        }
    }

    pub fn region(&self) -> Option<CaptureRegion> {
        self.current_region
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}

impl Default for MockCapturer {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenCapturer for MockCapturer {
    fn start_capture(&mut self, region: CaptureRegion) -> Result<(), CaptureError> {
        if self.capturing {
            return Err(CaptureError::AlreadyActive);
        }
        if region.is_empty() {
            return Err(CaptureError::ZeroSizedRegion {
                width: region.width,
                height: region.height,
            });
        }
        self.capturing = true;
        self.paused = false;
        self.current_region = Some(region);
        self.frame_count = 0;
        Ok(())
    }

    fn stop_capture(&mut self) -> Result<(), CaptureError> {
        if !self.capturing {
            return Err(CaptureError::NotActive);
        }
        self.capturing = false;
        self.paused = false;
        self.current_region = None;
        Ok(())
    }

    fn pause_capture(&mut self) -> Result<(), CaptureError> {
        if !self.capturing {
            return Err(CaptureError::NotActive);
        }
        if self.paused {
            return Err(CaptureError::Paused);
        }
        self.paused = true;
        Ok(())
    }

    fn resume_capture(&mut self) -> Result<(), CaptureError> {
        if !self.capturing {
            return Err(CaptureError::NotActive);
        }
        if !self.paused {
            return Err(CaptureError::NotPaused);
        }
        self.paused = false;
        Ok(())
    }

    fn is_capturing(&self) -> bool {
        self.capturing
    }

    fn is_paused(&self) -> bool {
        self.paused
    }

    fn capture_frame(&mut self) -> Result<Option<RawFrame>, CaptureError> {
        if !self.capturing {
            return Err(CaptureError::NotActive);
        }
        if self.paused {
            return Ok(None);
        }
        let region = self.current_region.ok_or(CaptureError::NotActive)?;
        let byte_len = region.expected_byte_len();
        // Generate synthetic patterned BGRA frame
        let timestamp_ms = self.frame_count * 33; // ~30 fps
        let mut bgra = vec![0u8; byte_len];
        for (i, byte) in bgra.iter_mut().enumerate() {
            *byte = ((self.frame_count as usize * 17 + i) % 256) as u8;
        }
        self.frame_count += 1;
        Ok(Some(RawFrame::new(
            timestamp_ms,
            region.width,
            region.height,
            bgra,
        )))
    }
}
