use carve_lib::encoder::{encode_gif, EncoderError, GifExportOptions};
use std::path::Path;

#[test]
fn test_encode_gif_creates_valid_file() {
    let tmp = tempfile::tempdir().unwrap();
    let out_file = tmp.path().join("output.gif");
    let opts = GifExportOptions {
        fps: 10,
        quality: 75,
        loop_count: 0,
        width: None,
        height: None,
    };
    let mut img = image::RgbaImage::new(64, 64);
    for p in img.pixels_mut() {
        *p = image::Rgba([255, 0, 0, 255]);
    }
    let frames = vec![(0.0, img.clone()), (0.1, img)];

    encode_gif(frames, opts, &out_file).unwrap();
    assert!(out_file.exists());
    let size = std::fs::metadata(&out_file).unwrap().len();
    assert!(size > 0);

    let content = std::fs::read(&out_file).unwrap();
    assert!(
        content.starts_with(b"GIF89a") || content.starts_with(b"GIF87a"),
        "File should start with GIF magic bytes"
    );
}

#[test]
fn test_encode_gif_empty_frames_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let out_file = tmp.path().join("empty.gif");
    let opts = GifExportOptions {
        fps: 10,
        quality: 75,
        loop_count: 0,
        width: None,
        height: None,
    };
    let frames: Vec<(f64, image::RgbaImage)> = vec![];

    let result = encode_gif(frames, opts, &out_file);
    assert!(matches!(result, Err(EncoderError::EmptyFrames)));
    assert!(!out_file.exists());
}

#[test]
fn test_encode_gif_quality_clamping_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let out_file = tmp.path().join("quality_zero.gif");
    let opts = GifExportOptions {
        fps: 10,
        quality: 0, // Clamped to 1
        loop_count: 0,
        width: None,
        height: None,
    };
    let img = image::RgbaImage::new(32, 32);
    let frames = vec![(0.0, img.clone()), (0.1, img)];

    let result = encode_gif(frames, opts, &out_file);
    assert!(
        result.is_ok(),
        "Quality 0 should be clamped to valid quality (1) and succeed"
    );
    assert!(out_file.exists());
    assert!(std::fs::metadata(&out_file).unwrap().len() > 0);
}

#[test]
fn test_encode_gif_quality_clamping_high() {
    let tmp = tempfile::tempdir().unwrap();
    let out_file = tmp.path().join("quality_high.gif");
    let opts = GifExportOptions {
        fps: 10,
        quality: 200, // Clamped to 100
        loop_count: 0,
        width: None,
        height: None,
    };
    let img = image::RgbaImage::new(32, 32);
    let frames = vec![(0.0, img.clone()), (0.1, img)];

    let result = encode_gif(frames, opts, &out_file);
    assert!(
        result.is_ok(),
        "Quality > 100 should be clamped to valid quality (100) and succeed"
    );
    assert!(out_file.exists());
    assert!(std::fs::metadata(&out_file).unwrap().len() > 0);
}

#[test]
fn test_encode_gif_with_resize_dimensions() {
    let tmp = tempfile::tempdir().unwrap();
    let out_file = tmp.path().join("resized.gif");
    let opts = GifExportOptions {
        fps: 10,
        quality: 75,
        loop_count: 0,
        width: Some(32),
        height: Some(32),
    };
    let img = image::RgbaImage::new(64, 64);
    let frames = vec![(0.0, img.clone()), (0.1, img)];

    let result = encode_gif(frames, opts, &out_file);
    assert!(result.is_ok());
    assert!(out_file.exists());
    assert!(std::fs::metadata(&out_file).unwrap().len() > 0);
}

#[test]
fn test_encode_gif_invalid_output_path() {
    let out_file = Path::new("/nonexistent_path_xyz_123/output.gif");
    let opts = GifExportOptions {
        fps: 10,
        quality: 75,
        loop_count: 0,
        width: None,
        height: None,
    };
    let img = image::RgbaImage::new(16, 16);
    let frames = vec![(0.0, img.clone()), (0.1, img)];

    let result = encode_gif(frames, opts, out_file);
    assert!(matches!(result, Err(EncoderError::OutputFile(_))));
}
