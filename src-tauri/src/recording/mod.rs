//! Recording lifecycle domain: state machine, capture acquisition, and
//! overlay window control. Tauri command glue lives in `crate::window_manager`.

pub mod acquisition;
pub mod controller;
pub mod overlay;

pub use acquisition::{
    start_recording_session, CaptureAcquisition, CaptureBackend, PortalAcquisition, StartedCapture,
};
pub use controller::{RecordingError, RecordingState};
pub use overlay::{hide_overlay, set_overlay_clickthrough, show_overlay};
