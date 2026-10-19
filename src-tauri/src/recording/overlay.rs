//! Live overlay window control: show, hide, and pointer clickthrough.

use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, Position, Runtime, Size};

use crate::capture::CaptureRegion;

use super::controller::RecordingError;

/// Positions, sizes, protects from screen capture, and shows the live overlay window.
///
/// Toggles window exclusion:
/// - Windows: SetWindowDisplayAffinity (WDA_EXCLUDEFROMCAPTURE via `set_content_protected(true)`)
/// - macOS: NSWindowSharingNone / SCContentFilter exclusion via `set_content_protected(true)`
pub fn show_overlay<R: Runtime>(
    app: &AppHandle<R>,
    region: CaptureRegion,
) -> Result<(), RecordingError> {
    let overlay = app
        .get_webview_window("overlay")
        .ok_or_else(|| RecordingError::WindowError("Overlay window not found".to_string()))?;

    // HiDPI Scaling: Use PhysicalPosition and PhysicalSize to guarantee 1:1 alignment with
    // physical screen pixels on Retina / 125%+ HiDPI displays without fractional DPI distortion.
    let _scale_factor = overlay.scale_factor().unwrap_or(1.0);
    let phys_pos = PhysicalPosition::new(region.x as i32, region.y as i32);
    let phys_size = PhysicalSize::new(region.width, region.height);

    overlay
        .set_position(Position::Physical(phys_pos))
        .map_err(|e| RecordingError::WindowError(e.to_string()))?;

    overlay
        .set_size(Size::Physical(phys_size))
        .map_err(|e| RecordingError::WindowError(e.to_string()))?;

    // Window exclusion handler for macOS & Windows
    let _ = overlay.set_content_protected(true);

    overlay
        .show()
        .map_err(|e| RecordingError::WindowError(e.to_string()))?;

    overlay
        .set_focus()
        .map_err(|e| RecordingError::WindowError(e.to_string()))?;

    Ok(())
}

/// Hides the live overlay window.
pub fn hide_overlay<R: Runtime>(app: &AppHandle<R>) -> Result<(), RecordingError> {
    let overlay = app
        .get_webview_window("overlay")
        .ok_or_else(|| RecordingError::WindowError("Overlay window not found".to_string()))?;

    overlay
        .hide()
        .map_err(|e| RecordingError::WindowError(e.to_string()))?;

    Ok(())
}

/// Enables or disables pointer clickthrough on the live overlay window.
pub fn set_overlay_clickthrough<R: Runtime>(
    app: &AppHandle<R>,
    enabled: bool,
) -> Result<(), RecordingError> {
    let overlay = app
        .get_webview_window("overlay")
        .ok_or_else(|| RecordingError::WindowError("Overlay window not found".to_string()))?;

    // On Linux GTK, setting cursor ignore on an unshown or hidden window panics in tao
    if !overlay.is_visible().unwrap_or(false) {
        return Ok(());
    }

    overlay
        .set_ignore_cursor_events(enabled)
        .map_err(|e| RecordingError::WindowError(e.to_string()))?;

    Ok(())
}
