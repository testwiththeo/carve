use image::{Rgba, RgbaImage};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::encoder::{encode_gif, EncoderError, GifExportOptions};
use crate::model::{TimedVectorLedger, VectorElement, VectorToolKind};
use crate::storage::{DiskFrameCache, RawFrame, StorageError};

pub mod commands;
pub use commands::{
    encode_raw_frames, export_gif, render_frame_with_vectors, update_vector_ledger,
    RawCompositedFrame, RenderedFrameResponse,
};

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("No frames available in cache")]
    EmptyCache,

    #[error("No frames found within trim range [{start}ms, {end}ms]")]
    NoFramesInRange { start: u64, end: u64 },

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Encoder error: {0}")]
    Encoder(#[from] EncoderError),

    #[error("Lock error: {0}")]
    Lock(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Managed session state holding active frame cache and vector annotations.
#[derive(Default, Clone)]
pub struct SessionState {
    pub cache: Arc<Mutex<Option<DiskFrameCache>>>,
    pub ledger: Arc<Mutex<TimedVectorLedger>>,
}

impl SessionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_cache(cache: DiskFrameCache) -> Self {
        Self {
            cache: Arc::new(Mutex::new(Some(cache))),
            ledger: Arc::new(Mutex::new(TimedVectorLedger::new())),
        }
    }
}

/// Parses a CSS/hex color string like "#EF4444" or "rgba(...)" into image::Rgba<u8>.
pub fn parse_color_hex(hex: &str) -> Rgba<u8> {
    let clean = hex.trim().trim_start_matches('#');
    match clean.len() {
        6 => {
            let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(239);
            let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(68);
            let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(68);
            Rgba([r, g, b, 255])
        }
        8 => {
            let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(239);
            let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(68);
            let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(68);
            let a = u8::from_str_radix(&clean[6..8], 16).unwrap_or(255);
            Rgba([r, g, b, a])
        }
        _ => Rgba([239, 68, 68, 255]),
    }
}

/// Composites an uncompressed BGRA RawFrame with active vector annotations at its timestamp,
/// producing an RGBA image suitable for Gifski encoding or preview rendering.
pub fn composite_frame_with_vectors(
    raw_frame: &RawFrame,
    active_elements: &[&VectorElement],
) -> RgbaImage {
    let width = raw_frame.width;
    let height = raw_frame.height;
    let mut img = RgbaImage::new(width, height);

    // 1. Copy BGRA raw frame into RGBA image
    for (i, chunk) in raw_frame.bgra_data.as_chunks::<4>().0.iter().enumerate() {
        let x = (i as u32) % width;
        let y = (i as u32) / width;
        if x < width && y < height {
            let b = chunk[0];
            let g = chunk[1];
            let r = chunk[2];
            let a = chunk[3];
            img.put_pixel(x, y, Rgba([r, g, b, a]));
        }
    }

    // 2. Rasterize vector elements over image
    for elem in active_elements {
        apply_vector_to_image(&mut img, elem);
    }

    img
}

/// Rasterizes an individual vector annotation over an RGBA image buffer.
fn apply_vector_to_image(img: &mut RgbaImage, elem: &VectorElement) {
    let width = img.width() as i32;
    let height = img.height() as i32;

    let x = elem.data.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let y = elem.data.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let w = elem.data.get("width").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let h = elem
        .data
        .get("height")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;
    let stroke_str = elem
        .data
        .get("stroke")
        .and_then(|v| v.as_str())
        .unwrap_or("#EF4444");
    let stroke_color = parse_color_hex(stroke_str);
    let stroke_width = elem
        .data
        .get("strokeWidth")
        .and_then(|v| v.as_i64())
        .unwrap_or(3) as i32;

    match elem.tool {
        VectorToolKind::Rectangle => {
            let fill_opt = elem.data.get("fill").and_then(|v| v.as_str());
            if let Some(fill_str) = fill_opt {
                if fill_str != "transparent" && !fill_str.is_empty() {
                    let fill_col = parse_color_hex(fill_str);
                    for py in y.max(0)..(y + h).min(height) {
                        for px in x.max(0)..(x + w).min(width) {
                            img.put_pixel(px as u32, py as u32, fill_col);
                        }
                    }
                }
            }
            // Draw rectangle border
            for step in 0..stroke_width {
                let rx1 = (x + step).clamp(0, width - 1) as u32;
                let rx2 = (x + w - 1 - step).clamp(0, width - 1) as u32;
                let ry1 = (y + step).clamp(0, height - 1) as u32;
                let ry2 = (y + h - 1 - step).clamp(0, height - 1) as u32;

                for px in rx1..=rx2 {
                    if ry1 < height as u32 {
                        img.put_pixel(px, ry1, stroke_color);
                    }
                    if ry2 < height as u32 {
                        img.put_pixel(px, ry2, stroke_color);
                    }
                }
                for py in ry1..=ry2 {
                    if rx1 < width as u32 {
                        img.put_pixel(rx1, py, stroke_color);
                    }
                    if rx2 < width as u32 {
                        img.put_pixel(rx2, py, stroke_color);
                    }
                }
            }
        }
        VectorToolKind::Ellipse => {
            let cx = x + w / 2;
            let cy = y + h / 2;
            let rx = (w / 2).max(1) as f64;
            let ry = (h / 2).max(1) as f64;

            let fill_opt = elem.data.get("fill").and_then(|v| v.as_str());
            if let Some(fill_str) = fill_opt {
                if fill_str != "transparent" && !fill_str.is_empty() {
                    let fill_col = parse_color_hex(fill_str);
                    for py in (cy - ry as i32).max(0)..(cy + ry as i32).min(height) {
                        for px in (cx - rx as i32).max(0)..(cx + rx as i32).min(width) {
                            let dx = (px - cx) as f64 / rx;
                            let dy = (py - cy) as f64 / ry;
                            if dx * dx + dy * dy <= 1.0 {
                                img.put_pixel(px as u32, py as u32, fill_col);
                            }
                        }
                    }
                }
            }

            // Draw ellipse border
            let perimeter_steps = ((rx + ry) * std::f64::consts::PI).max(10.0) as usize * 2;
            for i in 0..perimeter_steps {
                let theta = (i as f64 / perimeter_steps as f64) * 2.0 * std::f64::consts::PI;
                let px = (cx as f64 + rx * theta.cos()).round() as i32;
                let py = (cy as f64 + ry * theta.sin()).round() as i32;
                draw_circle_brush(img, px, py, stroke_width, stroke_color);
            }
        }
        VectorToolKind::Arrow => {
            if let Some(pts) = elem.data.get("points").and_then(|v| v.as_array()) {
                if pts.len() >= 4 {
                    let x1 = pts[0].as_i64().unwrap_or(0) as i32;
                    let y1 = pts[1].as_i64().unwrap_or(0) as i32;
                    let x2 = pts[2].as_i64().unwrap_or(0) as i32;
                    let y2 = pts[3].as_i64().unwrap_or(0) as i32;

                    draw_line(img, x1, y1, x2, y2, stroke_width, stroke_color);

                    // Arrowhead
                    let angle = ((y2 - y1) as f64).atan2((x2 - x1) as f64);
                    let head_len = 16.0f64.max(stroke_width as f64 * 3.0);
                    let arrow_angle = std::f64::consts::PI / 6.0;

                    let hx1 = (x2 as f64 - head_len * (angle - arrow_angle).cos()).round() as i32;
                    let hy1 = (y2 as f64 - head_len * (angle - arrow_angle).sin()).round() as i32;
                    let hx2 = (x2 as f64 - head_len * (angle + arrow_angle).cos()).round() as i32;
                    let hy2 = (y2 as f64 - head_len * (angle + arrow_angle).sin()).round() as i32;

                    draw_line(img, x2, y2, hx1, hy1, stroke_width, stroke_color);
                    draw_line(img, x2, y2, hx2, hy2, stroke_width, stroke_color);
                }
            }
        }
        VectorToolKind::Pencil => {
            if let Some(pts) = elem.data.get("points").and_then(|v| v.as_array()) {
                for pair in pts.chunks(2) {
                    if pair.len() == 2 {
                        let px = pair[0].as_i64().unwrap_or(0) as i32;
                        let py = pair[1].as_i64().unwrap_or(0) as i32;
                        draw_circle_brush(img, px, py, stroke_width, stroke_color);
                    }
                }
            }
        }
        VectorToolKind::CounterBadge => {
            let radius = 14;
            let badge_bg = stroke_color;
            let badge_fg = Rgba([255, 255, 255, 255]);

            // Draw outer filled badge circle
            for py in (y - radius).max(0)..(y + radius).min(height) {
                for px in (x - radius).max(0)..(x + radius).min(width) {
                    let d2 = (px - x) * (px - x) + (py - y) * (py - y);
                    if d2 <= radius * radius {
                        img.put_pixel(px as u32, py as u32, badge_bg);
                    }
                }
            }
            // Draw inner center dot
            for py in (y - 3).max(0)..(y + 3).min(height) {
                for px in (x - 3).max(0)..(x + 3).min(width) {
                    img.put_pixel(px as u32, py as u32, badge_fg);
                }
            }
        }
        VectorToolKind::Text => {
            // Draw a high-contrast text pill marker
            let tw = 60.max(w);
            let th = 24.max(h);
            let bg_color = Rgba([30, 41, 59, 220]);
            for py in y.max(0)..(y + th).min(height) {
                for px in x.max(0)..(x + tw).min(width) {
                    img.put_pixel(px as u32, py as u32, bg_color);
                }
            }
            // Inner line indicator
            let text_color = stroke_color;
            for px in (x + 6).max(0)..(x + tw - 6).min(width) {
                if y + 12 < height && y + 12 >= 0 {
                    img.put_pixel(px as u32, (y + 12) as u32, text_color);
                }
            }
        }
        VectorToolKind::Blur => {
            // Box/block pixelation effect on the specified subregion
            let block_size = 8;
            let rx1 = x.max(0);
            let ry1 = y.max(0);
            let rx2 = (x + w).min(width);
            let ry2 = (y + h).min(height);

            for by in (ry1..ry2).step_by(block_size) {
                for bx in (rx1..rx2).step_by(block_size) {
                    let mut r_sum = 0u32;
                    let mut g_sum = 0u32;
                    let mut b_sum = 0u32;
                    let mut count = 0u32;

                    let block_end_y = (by + block_size as i32).min(ry2);
                    let block_end_x = (bx + block_size as i32).min(rx2);

                    for py in by..block_end_y {
                        for px in bx..block_end_x {
                            let p = img.get_pixel(px as u32, py as u32);
                            r_sum += p.0[0] as u32;
                            g_sum += p.0[1] as u32;
                            b_sum += p.0[2] as u32;
                            count += 1;
                        }
                    }

                    let avg = |sum: u32| sum.checked_div(count).map(|v| v as u8);
                    if let (Some(r), Some(g), Some(b)) = (avg(r_sum), avg(g_sum), avg(b_sum)) {
                        let avg_color = Rgba([r, g, b, 255]);
                        for py in by..block_end_y {
                            for px in bx..block_end_x {
                                img.put_pixel(px as u32, py as u32, avg_color);
                            }
                        }
                    }
                }
            }
        }
        VectorToolKind::Highlight => {
            // Semi-transparent yellow highlight blending
            let rx1 = x.max(0);
            let ry1 = y.max(0);
            let rx2 = (x + w).min(width);
            let ry2 = (y + h).min(height);

            let hl_r = 255f64;
            let hl_g = 235f64;
            let hl_b = 59f64;
            let alpha = 0.45f64;

            for py in ry1..ry2 {
                for px in rx1..rx2 {
                    let orig = img.get_pixel(px as u32, py as u32);
                    let r = ((1.0 - alpha) * orig.0[0] as f64 + alpha * hl_r).round() as u8;
                    let g = ((1.0 - alpha) * orig.0[1] as f64 + alpha * hl_g).round() as u8;
                    let b = ((1.0 - alpha) * orig.0[2] as f64 + alpha * hl_b).round() as u8;
                    img.put_pixel(px as u32, py as u32, Rgba([r, g, b, orig.0[3]]));
                }
            }
        }
    }
}

/// Helper to draw a filled circle brush footprint
fn draw_circle_brush(img: &mut RgbaImage, cx: i32, cy: i32, radius: i32, color: Rgba<u8>) {
    let width = img.width() as i32;
    let height = img.height() as i32;
    let r = radius.max(1);
    for py in (cy - r).max(0)..(cy + r).min(height) {
        for px in (cx - r).max(0)..(cx + r).min(width) {
            if (px - cx) * (px - cx) + (py - cy) * (py - cy) <= r * r {
                img.put_pixel(px as u32, py as u32, color);
            }
        }
    }
}

/// Bresenham-style line drawing with brush stroke thickness
fn draw_line(
    img: &mut RgbaImage,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    thickness: i32,
    color: Rgba<u8>,
) {
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;

    let mut curr_x = x0;
    let mut curr_y = y0;
    let radius = (thickness / 2).max(1);

    loop {
        draw_circle_brush(img, curr_x, curr_y, radius, color);
        if curr_x == x1 && curr_y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            curr_x += sx;
        }
        if e2 < dx {
            err += dx;
            curr_y += sy;
        }
    }
}

/// Lazy streaming iterator reading raw frames from DiskFrameCache one frame at a time,
/// compositing active vector annotations on the fly, and yielding (pts, RgbaImage).
/// This keeps memory consumption strictly bounded to a single frame (~2-8 MB).
pub struct SessionFrameIterator<'a> {
    cache: &'a DiskFrameCache,
    ledger: &'a TimedVectorLedger,
    indices: std::vec::IntoIter<usize>,
    min_trim: u64,
    prev_pts: Option<f64>,
    is_single_frame: bool,
    single_duration: f64,
    yielded_first: bool,
    first_frame_clone: Option<(f64, RgbaImage)>,
}

impl<'a> Iterator for SessionFrameIterator<'a> {
    type Item = (f64, RgbaImage);

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_single_frame && self.yielded_first {
            self.yielded_first = false;
            return self.first_frame_clone.take();
        }

        let idx = self.indices.next()?;
        let raw = match self.cache.read_frame(idx) {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!(index = idx, error = %e, "dropping unreadable frame from export");
                return None;
            }
        };

        let mut pts = (raw.timestamp_ms.saturating_sub(self.min_trim)) as f64 / 1000.0;
        if let Some(prev) = self.prev_pts {
            if pts <= prev {
                pts = prev + 0.001;
            }
        }
        self.prev_pts = Some(pts);

        let active = self.ledger.active_at(raw.timestamp_ms);
        let img = composite_frame_with_vectors(&raw, &active);

        if self.is_single_frame {
            self.yielded_first = true;
            self.first_frame_clone = Some((pts + self.single_duration, img.clone()));
        }

        Some((pts, img))
    }
}

/// Export a recording session by lazily iterating through cached frames in `[trim_start_ms, trim_end_ms]`,
/// compositing active vector elements on the fly, and streaming into `encode_gif`.
/// Guarantees bounded RAM consumption of a single frame (~2-8 MB) regardless of capture length.
#[tracing::instrument(skip(cache, ledger, options))]
pub fn export_session_to_gif(
    cache: &DiskFrameCache,
    ledger: &TimedVectorLedger,
    output_path: &Path,
    options: GifExportOptions,
    trim_start_ms: u64,
    trim_end_ms: u64,
) -> Result<(), ExportError> {
    let total_frames = cache.total_frames();
    if total_frames == 0 {
        return Err(ExportError::EmptyCache);
    }

    let min_trim = trim_start_ms.min(trim_end_ms);
    let max_trim = trim_start_ms.max(trim_end_ms);

    let matching_indices: Vec<usize> = cache
        .index_entries()
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.timestamp_ms >= min_trim && entry.timestamp_ms <= max_trim)
        .map(|(i, _)| i)
        .collect();

    if matching_indices.is_empty() {
        return Err(ExportError::NoFramesInRange {
            start: min_trim,
            end: max_trim,
        });
    }

    let is_single_frame = matching_indices.len() == 1;
    let single_duration = 1.0 / (options.fps.max(1) as f64);

    let frame_stream = SessionFrameIterator {
        cache,
        ledger,
        indices: matching_indices.into_iter(),
        min_trim,
        prev_pts: None,
        is_single_frame,
        single_duration,
        yielded_first: false,
        first_frame_clone: None,
    };

    encode_gif(frame_stream, options, output_path)?;
    Ok(())
}

/// End-to-end mock export pipeline helper connecting synthetic frames from DiskFrameCache,
/// TimedVectorLedger, and encode_gif.
pub fn run_mock_export_pipeline(output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    let mut cache = DiskFrameCache::new(tmp.path())?;

    let width = 160u32;
    let height = 120u32;
    for i in 0..10 {
        let mut bgra = vec![0u8; (width * height * 4) as usize];
        let color_offset = (i * 10) as u8;
        for chunk in bgra.as_chunks_mut::<4>().0 {
            chunk[0] = 50 + color_offset; // B
            chunk[1] = 80 + color_offset; // G
            chunk[2] = 100 + color_offset; // R
            chunk[3] = 255; // A
        }
        let frame = RawFrame::new(i * 100, width, height, bgra);
        cache.append_frame(&frame)?;
    }

    let mut ledger = TimedVectorLedger::new();
    ledger.add_element(VectorElement::new(
        "rect_mock",
        VectorToolKind::Rectangle,
        0,
        500,
        serde_json::json!({
            "x": 20,
            "y": 20,
            "width": 60,
            "height": 40,
            "stroke": "#EF4444",
            "strokeWidth": 2
        }),
    ));
    ledger.add_element(VectorElement::new(
        "arrow_mock",
        VectorToolKind::Arrow,
        200,
        800,
        serde_json::json!({
            "points": [10, 10, 100, 100],
            "stroke": "#3B82F6",
            "strokeWidth": 3
        }),
    ));

    let options = GifExportOptions {
        fps: 10,
        quality: 80,
        loop_count: 0,
        width: None,
        height: None,
    };

    export_session_to_gif(&cache, &ledger, output_path, options, 0, 900)?;
    Ok(())
}
