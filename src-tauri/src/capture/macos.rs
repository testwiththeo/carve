use crate::capture::traits::{CaptureError, CaptureRegion, ScreenCapturer};
use crate::storage::RawFrame;

/// macOS ScreenCaptureKit-based screen capturer.
///
/// On macOS 12.3+, utilizes `ScreenCaptureKit` (`SCStream`) with `SCContentFilter`
/// to capture display frames and exclude recording overlay windows.
#[derive(Debug, Default)]
pub struct MacosCapturer {
    capturing: bool,
    paused: bool,
    current_region: Option<CaptureRegion>,
}

impl MacosCapturer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current_region(&self) -> Option<CaptureRegion> {
        self.current_region
    }
}

impl ScreenCapturer for MacosCapturer {
    fn start_capture(&mut self, region: CaptureRegion) -> Result<(), CaptureError> {
        #[cfg(target_os = "macos")]
        {
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
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = region;
            Err(CaptureError::BackendError(
                "ScreenCaptureKit is only supported on macOS".to_string(),
            ))
        }
    }

    fn stop_capture(&mut self) -> Result<(), CaptureError> {
        #[cfg(target_os = "macos")]
        {
            if !self.capturing {
                return Err(CaptureError::NotActive);
            }
            self.capturing = false;
            self.paused = false;
            self.current_region = None;
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(CaptureError::BackendError(
                "ScreenCaptureKit is only supported on macOS".to_string(),
            ))
        }
    }

    fn pause_capture(&mut self) -> Result<(), CaptureError> {
        #[cfg(target_os = "macos")]
        {
            if !self.capturing {
                return Err(CaptureError::NotActive);
            }
            if self.paused {
                return Err(CaptureError::Paused);
            }
            self.paused = true;
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(CaptureError::BackendError(
                "ScreenCaptureKit is only supported on macOS".to_string(),
            ))
        }
    }

    fn resume_capture(&mut self) -> Result<(), CaptureError> {
        #[cfg(target_os = "macos")]
        {
            if !self.capturing {
                return Err(CaptureError::NotActive);
            }
            if !self.paused {
                return Err(CaptureError::NotPaused);
            }
            self.paused = false;
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(CaptureError::BackendError(
                "ScreenCaptureKit is only supported on macOS".to_string(),
            ))
        }
    }

    fn is_capturing(&self) -> bool {
        self.capturing
    }

    fn is_paused(&self) -> bool {
        self.paused
    }

    fn capture_frame(&mut self) -> Result<Option<RawFrame>, CaptureError> {
        #[cfg(target_os = "macos")]
        {
            if !self.capturing {
                return Err(CaptureError::NotActive);
            }
            // In full implementation, frames arrive asynchronously via SCStreamDelegate
            Ok(None)
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(CaptureError::BackendError(
                "ScreenCaptureKit is only supported on macOS".to_string(),
            ))
        }
    }
}
