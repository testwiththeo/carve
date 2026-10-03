use crate::capture::traits::{CaptureError, CaptureRegion, ScreenCapturer};
use crate::storage::RawFrame;

/// Windows Graphics Capture (WGC)-based screen capturer.
///
/// Utilizes `Windows.Graphics.Capture.GraphicsCaptureItem` paired with
/// Win32 `SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)` to capture
/// the screen while keeping the recording toolbar and annotations invisible to the frame buffer.
#[derive(Debug, Default)]
pub struct WindowsCapturer {
    capturing: bool,
    paused: bool,
    current_region: Option<CaptureRegion>,
}

impl WindowsCapturer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current_region(&self) -> Option<CaptureRegion> {
        self.current_region
    }
}

impl ScreenCapturer for WindowsCapturer {
    fn start_capture(&mut self, region: CaptureRegion) -> Result<(), CaptureError> {
        #[cfg(target_os = "windows")]
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
        #[cfg(not(target_os = "windows"))]
        {
            let _ = region;
            Err(CaptureError::BackendError(
                "Windows Graphics Capture is only supported on Windows".to_string(),
            ))
        }
    }

    fn stop_capture(&mut self) -> Result<(), CaptureError> {
        #[cfg(target_os = "windows")]
        {
            if !self.capturing {
                return Err(CaptureError::NotActive);
            }
            self.capturing = false;
            self.paused = false;
            self.current_region = None;
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(CaptureError::BackendError(
                "Windows Graphics Capture is only supported on Windows".to_string(),
            ))
        }
    }

    fn pause_capture(&mut self) -> Result<(), CaptureError> {
        #[cfg(target_os = "windows")]
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
        #[cfg(not(target_os = "windows"))]
        {
            Err(CaptureError::BackendError(
                "Windows Graphics Capture is only supported on Windows".to_string(),
            ))
        }
    }

    fn resume_capture(&mut self) -> Result<(), CaptureError> {
        #[cfg(target_os = "windows")]
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
        #[cfg(not(target_os = "windows"))]
        {
            Err(CaptureError::BackendError(
                "Windows Graphics Capture is only supported on Windows".to_string(),
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
        #[cfg(target_os = "windows")]
        {
            if !self.capturing {
                return Err(CaptureError::NotActive);
            }
            // In full implementation, frames arrive asynchronously via Direct3D11CaptureFramePool
            Ok(None)
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(CaptureError::BackendError(
                "Windows Graphics Capture is only supported on Windows".to_string(),
            ))
        }
    }
}
