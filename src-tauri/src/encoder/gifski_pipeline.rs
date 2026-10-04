use std::io::Write;
use std::path::Path;
use thiserror::Error;

/// Options for GIF export via Gifski.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GifExportOptions {
    pub fps: u32,
    pub quality: u8,
    pub loop_count: u16,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl Default for GifExportOptions {
    fn default() -> Self {
        Self {
            fps: 30,
            quality: 80,
            loop_count: 0,
            width: None,
            height: None,
        }
    }
}

impl GifExportOptions {
    pub fn new(fps: u32, quality: u8, loop_count: u16) -> Self {
        Self {
            fps,
            quality,
            loop_count,
            width: None,
            height: None,
        }
    }

    pub fn with_dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }
}

/// Errors occurring during GIF encoding.
#[derive(Error, Debug)]
pub enum EncoderError {
    #[error("cannot encode gif from empty frames iterator")]
    EmptyFrames,

    #[error("failed to initialize gifski: {0}")]
    GifskiInit(String),

    #[error("failed to create or write output file: {0}")]
    OutputFile(#[source] std::io::Error),

    #[error("failed to encode frame: {0}")]
    FrameEncode(String),

    #[error("failed during gifski write: {0}")]
    GifskiWrite(String),

    #[error("encoder worker thread panicked: {0}")]
    ThreadError(String),
}

/// Encode a sequence of timestamped RGBA frames into a GIF using Gifski.
///
/// Frames are expected as an iterator of `(presentation_timestamp_seconds, RgbaImage)`.
pub fn encode_gif<I>(
    frames: I,
    opts: GifExportOptions,
    output_path: &Path,
) -> Result<(), EncoderError>
where
    I: IntoIterator<Item = (f64, image::RgbaImage)>,
{
    let mut iter = frames.into_iter().peekable();
    if iter.peek().is_none() {
        return Err(EncoderError::EmptyFrames);
    }

    let quality = opts.quality.clamp(1, 100);
    let repeat = if opts.loop_count == 0 {
        gifski::Repeat::Infinite
    } else {
        gifski::Repeat::Finite(opts.loop_count)
    };

    let settings = gifski::Settings {
        width: opts.width,
        height: opts.height,
        quality,
        fast: false,
        repeat,
    };

    let (collector, writer) =
        gifski::new(settings).map_err(|e| EncoderError::GifskiInit(e.to_string()))?;

    let file = std::fs::File::create(output_path).map_err(EncoderError::OutputFile)?;
    let mut writer_buf = std::io::BufWriter::new(file);

    let run_res: Result<(), EncoderError> = std::thread::scope(|s| {
        let writer_handle = s.spawn(move || {
            let mut reporter = gifski::progress::NoProgress {};
            writer
                .write(&mut writer_buf, &mut reporter)
                .map_err(|e| EncoderError::GifskiWrite(e.to_string()))?;
            writer_buf.flush().map_err(EncoderError::OutputFile)?;
            Ok(())
        });

        let mut frame_err = None;
        for (frame_idx, (pts, frame)) in iter.enumerate() {
            let width = frame.width() as usize;
            let height = frame.height() as usize;
            let raw = frame.into_raw();
            let pixels: Vec<rgb::RGBA8> = raw
                .as_chunks::<4>()
                .0
                .iter()
                .map(|chunk| rgb::RGBA8::new(chunk[0], chunk[1], chunk[2], chunk[3]))
                .collect();
            let img_vec = imgref::ImgVec::new(pixels, width, height);

            if let Err(e) = collector.add_frame_rgba(frame_idx, img_vec, pts) {
                frame_err = Some(EncoderError::FrameEncode(e.to_string()));
                break;
            }
        }

        drop(collector);

        let writer_res = writer_handle
            .join()
            .map_err(|_| EncoderError::ThreadError("writer thread panicked".into()))?;

        if let Some(err) = frame_err {
            if let Err(w_err) = writer_res {
                return Err(w_err);
            }
            return Err(err);
        }

        writer_res
    });

    if run_res.is_err() {
        let _ = std::fs::remove_file(output_path);
    }

    run_res
}
