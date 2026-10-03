pub mod cropper;
pub mod linux;
pub mod macos;
pub mod mock;
pub mod traits;
pub mod windows;

pub use cropper::{crop_bgra, crop_bgra_into};
pub use linux::LinuxPortalCapturer;
pub use macos::MacosCapturer;
pub use mock::MockCapturer;
pub use traits::{CaptureError, CaptureRegion, ScreenCapturer};
pub use windows::WindowsCapturer;

/// Factory function to create the default platform capturer.
pub fn create_platform_capturer() -> Box<dyn ScreenCapturer> {
    #[cfg(target_os = "linux")]
    {
        Box::new(LinuxPortalCapturer::new())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(MacosCapturer::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(WindowsCapturer::new())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Box::new(MockCapturer::new())
    }
}
