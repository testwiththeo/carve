use crate::capture::traits::{CaptureError, CaptureRegion, ScreenCapturer};
use crate::storage::RawFrame;

#[cfg(target_os = "linux")]
use crate::capture::pipewire_stream::PipeWireStreamConsumer;

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
    #[cfg(target_os = "linux")]
    stream_consumer: Option<PipeWireStreamConsumer>,
    #[cfg(target_os = "linux")]
    is_mock: bool,
    #[cfg(target_os = "linux")]
    crop_buffer: Vec<u8>,
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
            #[cfg(target_os = "linux")]
            stream_consumer: None,
            #[cfg(target_os = "linux")]
            is_mock: true,
            #[cfg(target_os = "linux")]
            crop_buffer: Vec::new(),
        }
    }

    #[cfg(target_os = "linux")]
    pub fn with_stream_info(
        info: crate::capture::portal_session::PortalStreamInfo,
    ) -> Result<Self, CaptureError> {
        let node_id = info.node_id;
        let consumer = PipeWireStreamConsumer::connect(info.fd, node_id)?;
        Ok(Self {
            capturing: false,
            paused: false,
            current_region: None,
            portal_session_id: None,
            pipewire_node_id: Some(node_id),
            stream_consumer: Some(consumer),
            is_mock: false,
            crop_buffer: Vec::new(),
        })
    }

    #[cfg(target_os = "linux")]
    pub fn with_consumer(consumer: PipeWireStreamConsumer) -> Self {
        Self {
            capturing: false,
            paused: false,
            current_region: None,
            portal_session_id: None,
            pipewire_node_id: None,
            stream_consumer: Some(consumer),
            is_mock: false,
            crop_buffer: Vec::new(),
        }
    }

    #[cfg(target_os = "linux")]
    pub fn new_mock() -> (Self, crate::capture::pipewire_stream::FrameSender) {
        let (consumer, sender) = PipeWireStreamConsumer::new_mock();
        let capturer = Self {
            capturing: false,
            paused: false,
            current_region: None,
            portal_session_id: None,
            pipewire_node_id: None,
            stream_consumer: Some(consumer),
            is_mock: true,
            crop_buffer: Vec::new(),
        };
        (capturer, sender)
    }

    #[cfg(target_os = "linux")]
    pub fn set_mock(&mut self, is_mock: bool) {
        self.is_mock = is_mock;
    }

    #[cfg(target_os = "linux")]
    pub fn is_mock(&self) -> bool {
        self.is_mock
    }

    #[cfg(target_os = "linux")]
    pub fn set_consumer(&mut self, consumer: PipeWireStreamConsumer) {
        self.stream_consumer = Some(consumer);
    }

    #[cfg(target_os = "linux")]
    pub fn inject_frame(&mut self, frame: RawFrame) -> Result<(), CaptureError> {
        if let Some(consumer) = self.stream_consumer.as_ref() {
            consumer.send_synthetic_frame(frame).map_err(|e| {
                CaptureError::BackendError(format!("Failed to inject frame: {e:?}"))
            })
        } else {
            Err(CaptureError::NotActive)
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

            if self.stream_consumer.is_none() {
                if self.is_mock {
                    let (consumer, _) = PipeWireStreamConsumer::new_mock();
                    self.stream_consumer = Some(consumer);
                } else {
                    let stream_info = crate::capture::portal_session::request_screencast_session_blocking(
                        ashpd::desktop::screencast::CursorMode::Embedded,
                        false,
                    )?;
                    let node_id = stream_info.node_id;
                    let consumer = PipeWireStreamConsumer::connect(stream_info.fd, node_id)?;
                    self.pipewire_node_id = Some(node_id);
                    self.stream_consumer = Some(consumer);
                }
            }

            self.capturing = true;
            self.paused = false;
            self.current_region = Some(region);
            let dest_len = (region.width as usize) * (region.height as usize) * 4;
            self.crop_buffer.resize(dest_len, 0);
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
            self.crop_buffer.clear();

            if let Some(mut consumer) = self.stream_consumer.take() {
                consumer.stop();
            }

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
            if let Some(consumer) = self.stream_consumer.as_ref() {
                consumer.flush();
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
            if self.paused {
                return Ok(None);
            }

            let Some(consumer) = self.stream_consumer.as_mut() else {
                return Ok(None);
            };

            let Some(raw_frame) = consumer.recv_frame() else {
                return Ok(None);
            };

            if let Some(region) = self.current_region {
                if region.x == 0
                    && region.y == 0
                    && region.width == raw_frame.width
                    && region.height == raw_frame.height
                {
                    Ok(Some(raw_frame))
                } else {
                    let dest_len = (region.width as usize) * (region.height as usize) * 4;
                    if self.crop_buffer.len() != dest_len {
                        self.crop_buffer.resize(dest_len, 0);
                    }
                    crate::capture::crop_bgra_into(
                        &raw_frame.bgra_data,
                        raw_frame.width,
                        raw_frame.height,
                        &region,
                        &mut self.crop_buffer,
                    )?;
                    let mut cropped = std::mem::replace(&mut self.crop_buffer, raw_frame.bgra_data);
                    cropped.truncate(dest_len);
                    self.crop_buffer.resize(dest_len, 0);
                    Ok(Some(RawFrame::new(
                        raw_frame.timestamp_ms,
                        region.width,
                        region.height,
                        cropped,
                    )))
                }
            } else {
                Ok(Some(raw_frame))
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(CaptureError::BackendError(
                "Linux Portal/PipeWire capture is only supported on Linux".to_string(),
            ))
        }
    }
}
