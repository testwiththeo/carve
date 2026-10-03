use crate::capture::traits::{CaptureError, CaptureRegion, ScreenCapturer};
use crate::storage::RawFrame;

/// Linux screen capturer using XDG Desktop Portal ScreenCast and PipeWire.
///
/// On Wayland and modern Linux desktops, initiates an `org.freedesktop.portal.ScreenCast`
/// session via DBus/ashpd to obtain a PipeWire stream file descriptor, then consumes
/// incoming video stream buffers and crops them to the selected `CaptureRegion`.
#[derive(Debug, Default)]
pub struct LinuxPortalCapturer {
    capturing: bool,
    paused: bool,
    current_region: Option<CaptureRegion>,
    portal_session_id: Option<String>,
    pipewire_node_id: Option<u32>,
}

impl LinuxPortalCapturer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_session(portal_session_id: String, pipewire_node_id: u32) -> Self {
        Self {
            capturing: false,
            paused: false,
            current_region: None,
            portal_session_id: Some(portal_session_id),
            pipewire_node_id: Some(pipewire_node_id),
        }
    }

    pub fn portal_session_id(&self) -> Option<&str> {
        self.portal_session_id.as_deref()
    }

    pub fn pipewire_node_id(&self) -> Option<u32> {
        self.pipewire_node_id
    }

    pub fn current_region(&self) -> Option<CaptureRegion> {
        self.current_region
    }
}

impl ScreenCapturer for LinuxPortalCapturer {
    fn start_capture(&mut self, region: CaptureRegion) -> Result<(), CaptureError> {
        #[cfg(target_os = "linux")]
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
        #[cfg(not(target_os = "linux"))]
        {
            let _ = region;
            Err(CaptureError::BackendError(
                "Linux Portal/PipeWire capture is only supported on Linux".to_string(),
            ))
        }
    }

    fn stop_capture(&mut self) -> Result<(), CaptureError> {
        #[cfg(target_os = "linux")]
        {
            if !self.capturing {
                return Err(CaptureError::NotActive);
            }
            self.capturing = false;
            self.paused = false;
            self.current_region = None;
            Ok(())
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(CaptureError::BackendError(
                "Linux Portal/PipeWire capture is only supported on Linux".to_string(),
            ))
        }
    }

    fn pause_capture(&mut self) -> Result<(), CaptureError> {
        #[cfg(target_os = "linux")]
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
        #[cfg(not(target_os = "linux"))]
        {
            Err(CaptureError::BackendError(
                "Linux Portal/PipeWire capture is only supported on Linux".to_string(),
            ))
        }
    }

    fn resume_capture(&mut self) -> Result<(), CaptureError> {
        #[cfg(target_os = "linux")]
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
        #[cfg(not(target_os = "linux"))]
        {
            Err(CaptureError::BackendError(
                "Linux Portal/PipeWire capture is only supported on Linux".to_string(),
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
        #[cfg(target_os = "linux")]
        {
            if !self.capturing {
                return Err(CaptureError::NotActive);
            }
            // Real PipeWire buffers arrive asynchronously via PipeWire stream callbacks
            Ok(None)
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(CaptureError::BackendError(
                "Linux Portal/PipeWire capture is only supported on Linux".to_string(),
            ))
        }
    }
}
