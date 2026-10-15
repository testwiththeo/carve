use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::State;

use super::{composite_frame_with_vectors, export_session_to_gif, SessionState};
use crate::encoder::GifExportOptions;
use crate::model::{TimedVectorLedger, VectorElement};

/// Response structure returning the composited RGBA frame and active vector elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedFrameResponse {
    pub index: usize,
    pub timestamp_ms: u64,
    pub width: u32,
    pub height: u32,
    pub rgba_data: Vec<u8>,
    pub active_elements: Vec<VectorElement>,
}

/// Raw composited frame payload received from offscreen client canvas rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawCompositedFrame {
    #[serde(alias = "timestamp_ms")]
    pub timestamp_ms: u64,
    pub width: u32,
    pub height: u32,
    #[serde(alias = "rgba_data")]
    pub rgba_data: Vec<u8>,
}

/// Exports the active recording session to a GIF file using stored frames and vector ledger.
#[tauri::command]
pub fn export_gif(
    state: State<'_, SessionState>,
    output_path: String,
    options: GifExportOptions,
    trim_start_ms: u64,
    trim_end_ms: u64,
) -> Result<String, String> {
    let cache_guard = state
        .cache
        .lock()
        .map_err(|_| "Failed to lock frame cache".to_string())?;

    let cache = cache_guard
        .as_ref()
        .ok_or_else(|| "No recorded frames available in session cache".to_string())?;

    let ledger_guard = state
        .ledger
        .lock()
        .map_err(|_| "Failed to lock vector ledger".to_string())?;

    let out_path = Path::new(&output_path);
    export_session_to_gif(
        cache,
        &ledger_guard,
        out_path,
        options,
        trim_start_ms,
        trim_end_ms,
    )
    .map_err(|e| e.to_string())?;

    Ok(output_path)
}

/// Fetches a frame by index, composites any active vector elements for its timestamp,
/// and returns the RGBA pixel buffer and active elements metadata.
#[tauri::command]
pub fn render_frame_with_vectors(
    state: State<'_, SessionState>,
    frame_index: usize,
) -> Result<RenderedFrameResponse, String> {
    let cache_guard = state
        .cache
        .lock()
        .map_err(|_| "Failed to lock frame cache".to_string())?;

    let cache = cache_guard
        .as_ref()
        .ok_or_else(|| "No recorded frames available in session cache".to_string())?;

    let frame = cache.read_frame(frame_index).map_err(|e| e.to_string())?;

    let ledger_guard = state
        .ledger
        .lock()
        .map_err(|_| "Failed to lock vector ledger".to_string())?;

    let active_elements = ledger_guard.active_elements_at(frame.timestamp_ms);
    let active_refs: Vec<&VectorElement> = active_elements.iter().collect();
    let composited = composite_frame_with_vectors(&frame, &active_refs);

    Ok(RenderedFrameResponse {
        index: frame_index,
        timestamp_ms: frame.timestamp_ms,
        width: composited.width(),
        height: composited.height(),
        rgba_data: composited.into_raw(),
        active_elements,
    })
}

/// Synchronizes the frontend TimedVectorLedger into the backend session state.
#[tauri::command]
pub fn update_vector_ledger(
    state: State<'_, SessionState>,
    ledger: TimedVectorLedger,
) -> Result<(), String> {
    let mut ledger_guard = state
        .ledger
        .lock()
        .map_err(|_| "Failed to lock vector ledger".to_string())?;
    *ledger_guard = ledger;
    Ok(())
}

/// Encodes raw RGBA frames streamed directly from client offscreen canvas compositor.
#[tauri::command]
pub fn encode_raw_frames(
    output_path: String,
    options: GifExportOptions,
    frames: Vec<RawCompositedFrame>,
) -> Result<String, String> {
    if frames.is_empty() {
        return Err("Cannot encode empty frames sequence".into());
    }

    let first_ts = frames[0].timestamp_ms;
    let mut img_frames = Vec::with_capacity(frames.len() + 1);

    let mut prev_pts: Option<f64> = None;
    for f in frames {
        let mut pts = (f.timestamp_ms.saturating_sub(first_ts)) as f64 / 1000.0;
        if let Some(prev) = prev_pts {
            if pts <= prev {
                pts = prev + 0.001;
            }
        }
        prev_pts = Some(pts);

        let img = image::RgbaImage::from_raw(f.width, f.height, f.rgba_data).ok_or_else(|| {
            format!(
                "Invalid image dimensions {}x{} for raw buffer size",
                f.width, f.height
            )
        })?;
        img_frames.push((pts, img));
    }

    // Single frame needs a non-zero duration for gifski
    if img_frames.len() == 1 {
        let frame_duration = 1.0 / (options.fps.max(1) as f64);
        img_frames.push((frame_duration, img_frames[0].1.clone()));
    }

    let out_path = Path::new(&output_path);
    crate::encoder::encode_gif(img_frames, options, out_path).map_err(|e| e.to_string())?;

    Ok(output_path)
}
