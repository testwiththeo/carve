use carve_lib::encoder::GifExportOptions;
use carve_lib::export::{
    composite_frame_with_vectors, encode_raw_frames, export_session_to_gif,
    run_mock_export_pipeline, RawCompositedFrame, SessionState,
};
use carve_lib::model::{TimedVectorLedger, VectorElement, VectorToolKind};
use carve_lib::storage::{DiskFrameCache, RawFrame};

#[test]
fn test_end_to_end_export_pipeline() {
    let tmp = tempfile::tempdir().unwrap();
    let gif_path = tmp.path().join("screencast.gif");
    let result = run_mock_export_pipeline(&gif_path);
    assert!(result.is_ok());
    assert!(gif_path.exists());
    assert!(std::fs::metadata(&gif_path).unwrap().len() > 1000);
}

#[test]
fn test_export_pipeline_zero_frames() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = DiskFrameCache::new(tmp.path().join("cache")).unwrap();
    let ledger = TimedVectorLedger::new();
    let gif_path = tmp.path().join("empty.gif");
    let opts = GifExportOptions::default();

    let result = export_session_to_gif(&cache, &ledger, &gif_path, opts, 0, 1000);
    assert!(result.is_err());
    assert!(!gif_path.exists());
}

#[test]
fn test_export_pipeline_custom_fps_and_quality() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cache = DiskFrameCache::new(tmp.path().join("cache")).unwrap();
    for i in 0..6 {
        let frame = RawFrame::new(i * 100, 64, 64, vec![((i * 40) % 255) as u8; 64 * 64 * 4]);
        cache.append_frame(&frame).unwrap();
    }
    let ledger = TimedVectorLedger::new();
    let gif_path = tmp.path().join("custom.gif");
    let opts = GifExportOptions::new(15, 50, 2);

    let result = export_session_to_gif(&cache, &ledger, &gif_path, opts, 0, 500);
    assert!(result.is_ok());
    assert!(gif_path.exists());
    assert!(std::fs::metadata(&gif_path).unwrap().len() > 0);
}

#[test]
fn test_export_pipeline_with_vector_annotations() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cache = DiskFrameCache::new(tmp.path().join("cache")).unwrap();
    for i in 0..8 {
        let frame = RawFrame::new(i * 100, 80, 80, vec![200; 80 * 80 * 4]);
        cache.append_frame(&frame).unwrap();
    }

    let mut ledger = TimedVectorLedger::new();
    ledger.add_element(VectorElement::new(
        "rect_1",
        VectorToolKind::Rectangle,
        0,
        400,
        serde_json::json!({
            "x": 10,
            "y": 10,
            "width": 30,
            "height": 30,
            "stroke": "#EF4444",
            "fill": "#3B82F6"
        }),
    ));
    ledger.add_element(VectorElement::new(
        "arrow_1",
        VectorToolKind::Arrow,
        200,
        700,
        serde_json::json!({
            "points": [10, 10, 60, 60],
            "stroke": "#10B981"
        }),
    ));
    ledger.add_element(VectorElement::new(
        "blur_1",
        VectorToolKind::Blur,
        300,
        800,
        serde_json::json!({
            "x": 20,
            "y": 20,
            "width": 40,
            "height": 40
        }),
    ));

    let gif_path = tmp.path().join("annotated.gif");
    let opts = GifExportOptions::new(10, 80, 0);
    let result = export_session_to_gif(&cache, &ledger, &gif_path, opts, 0, 700);
    assert!(result.is_ok());
    assert!(gif_path.exists());
    assert!(std::fs::metadata(&gif_path).unwrap().len() > 1000);
}

#[test]
fn test_export_pipeline_trim_boundaries() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cache = DiskFrameCache::new(tmp.path().join("cache")).unwrap();
    for i in 0..10 {
        let frame = RawFrame::new(i * 100, 64, 64, vec![(i * 25) as u8; 64 * 64 * 4]);
        cache.append_frame(&frame).unwrap();
    }
    let ledger = TimedVectorLedger::new();
    let gif_path = tmp.path().join("trimmed.gif");
    let opts = GifExportOptions::new(10, 70, 0);

    let result = export_session_to_gif(&cache, &ledger, &gif_path, opts, 250, 650);
    assert!(result.is_ok());
    assert!(gif_path.exists());
    assert!(std::fs::metadata(&gif_path).unwrap().len() > 500);

    let inverted_path = tmp.path().join("inverted.gif");
    let inv_result = export_session_to_gif(&cache, &ledger, &inverted_path, opts, 650, 250);
    assert!(inv_result.is_ok());
    assert!(inverted_path.exists());
}

#[test]
fn test_composite_frame_with_vectors_direct() {
    let frame = RawFrame::new(100, 50, 50, vec![255; 50 * 50 * 4]);
    let elem = VectorElement::new(
        "badge_1",
        VectorToolKind::CounterBadge,
        50,
        150,
        serde_json::json!({ "x": 25, "y": 25, "count": 1, "stroke": "#EF4444" }),
    );
    let active = vec![&elem];
    let img = composite_frame_with_vectors(&frame, &active);
    assert_eq!(img.width(), 50);
    assert_eq!(img.height(), 50);
    // Ensure the badge background modified pixels from pure white
    let badge_pixel = img.get_pixel(25 + 8, 25);
    assert_eq!(badge_pixel.0, [239, 68, 68, 255]);
    assert!(img.pixels().any(|p| p.0 != [255, 255, 255, 255]));
}

#[test]
fn test_encode_raw_frames_success() {
    let tmp = tempfile::tempdir().unwrap();
    let gif_path = tmp.path().join("raw_export.gif");
    let opts = GifExportOptions::new(10, 80, 0);

    let mut frames = Vec::new();
    for i in 0..4 {
        let rgba_data = vec![((i * 60) % 255) as u8; 32 * 32 * 4];
        frames.push(RawCompositedFrame {
            timestamp_ms: i * 100,
            width: 32,
            height: 32,
            rgba_data,
        });
    }

    let result = encode_raw_frames(gif_path.to_str().unwrap().to_string(), opts, frames);
    assert!(result.is_ok());
    assert!(gif_path.exists());
    assert!(std::fs::metadata(&gif_path).unwrap().len() > 100);
}

#[test]
fn test_encode_raw_frames_single_frame() {
    let tmp = tempfile::tempdir().unwrap();
    let gif_path = tmp.path().join("single_raw.gif");
    let opts = GifExportOptions::new(10, 80, 0);

    let frames = vec![RawCompositedFrame {
        timestamp_ms: 0,
        width: 32,
        height: 32,
        rgba_data: vec![200; 32 * 32 * 4],
    }];

    let result = encode_raw_frames(gif_path.to_str().unwrap().to_string(), opts, frames);
    assert!(result.is_ok());
    assert!(gif_path.exists());
    assert!(std::fs::metadata(&gif_path).unwrap().len() > 50);
}

#[test]
fn test_encode_raw_frames_empty_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let gif_path = tmp.path().join("empty_raw.gif");
    let opts = GifExportOptions::new(10, 80, 0);

    let result = encode_raw_frames(gif_path.to_str().unwrap().to_string(), opts, vec![]);
    assert!(result.is_err());
    assert!(!gif_path.exists());
}

#[test]
fn test_encode_raw_frames_invalid_dimensions() {
    let tmp = tempfile::tempdir().unwrap();
    let gif_path = tmp.path().join("invalid_raw.gif");
    let opts = GifExportOptions::new(10, 80, 0);

    let frames = vec![RawCompositedFrame {
        timestamp_ms: 0,
        width: 32,
        height: 32,
        rgba_data: vec![200; 10], // Corrupt size: expected 32*32*4 = 4096 bytes
    }];

    let result = encode_raw_frames(gif_path.to_str().unwrap().to_string(), opts, frames);
    assert!(result.is_err());
    assert!(!gif_path.exists());
}

#[test]
fn test_streaming_export_with_session_state() {
    let tmp = tempfile::tempdir().unwrap();
    let cache_dir = tmp.path().join("session_cache");
    let mut cache = DiskFrameCache::new(&cache_dir).unwrap();

    // Generate 12 frames to verify lazy streaming
    for i in 0..12 {
        let frame = RawFrame::new(i * 50, 64, 64, vec![((i * 20) % 255) as u8; 64 * 64 * 4]);
        cache.append_frame(&frame).unwrap();
    }

    let session = SessionState::with_cache(cache);
    let gif_path = tmp.path().join("session_streamed.gif");
    let opts = GifExportOptions::new(20, 80, 0);

    let cache_guard = session.cache.lock().unwrap();
    let ledger_guard = session.ledger.lock().unwrap();

    let res = export_session_to_gif(
        cache_guard.as_ref().unwrap(),
        &ledger_guard,
        &gif_path,
        opts,
        0,
        600,
    );

    assert!(res.is_ok());
    assert!(gif_path.exists());
    assert!(std::fs::metadata(&gif_path).unwrap().len() > 100);
}
