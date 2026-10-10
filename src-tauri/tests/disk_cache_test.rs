use carve_lib::storage::{DiskFrameCache, RawFrame};

#[test]
fn test_disk_frame_cache_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cache = DiskFrameCache::new(tmp.path()).unwrap();
    let dummy_frame = RawFrame {
        timestamp_ms: 33,
        width: 100,
        height: 100,
        bgra_data: vec![255; 100 * 100 * 4],
    };
    cache.append_frame(&dummy_frame).unwrap();
    assert_eq!(cache.total_frames(), 1);

    let loaded = cache.read_frame(0).unwrap();
    assert_eq!(loaded.timestamp_ms, 33);
    assert_eq!(loaded.width, 100);
    assert_eq!(loaded.height, 100);
    assert_eq!(loaded.bgra_data, dummy_frame.bgra_data);
}

#[test]
fn test_disk_cache_multi_frame_indexing() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cache = DiskFrameCache::new(tmp.path()).unwrap();
    let mut frames = Vec::new();

    for i in 0..10 {
        let frame = RawFrame {
            timestamp_ms: i * 33,
            width: 10 + (i as u32),
            height: 10 + (i as u32),
            bgra_data: vec![(i * 20) as u8; ((10 + i) * (10 + i) * 4) as usize],
        };
        cache.append_frame(&frame).unwrap();
        frames.push(frame);
    }

    assert_eq!(cache.total_frames(), 10);

    // Read back in arbitrary non-sequential order
    for &idx in &[9, 0, 4, 7, 2, 8, 1, 3, 6, 5] {
        let loaded = cache.read_frame(idx).unwrap();
        assert_eq!(loaded.timestamp_ms, frames[idx].timestamp_ms);
        assert_eq!(loaded.width, frames[idx].width);
        assert_eq!(loaded.height, frames[idx].height);
        assert_eq!(loaded.bgra_data, frames[idx].bgra_data);
    }
}

#[test]
fn test_disk_cache_bounds_checking() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cache = DiskFrameCache::new(tmp.path()).unwrap();
    assert_eq!(cache.total_frames(), 0);
    assert!(cache.is_empty());

    // Read on empty cache
    assert!(cache.read_frame(0).is_err());

    let frame = RawFrame {
        timestamp_ms: 100,
        width: 10,
        height: 10,
        bgra_data: vec![0; 400],
    };
    cache.append_frame(&frame).unwrap();

    assert!(!cache.is_empty());
    assert_eq!(cache.total_frames(), 1);
    assert!(cache.read_frame(0).is_ok());
    assert!(cache.read_frame(1).is_err());
    assert!(cache.read_frame(999).is_err());
}

#[test]
fn test_disk_cache_cleanup_on_drop() {
    let tmp = tempfile::tempdir().unwrap();
    let cache_dir = tmp.path().to_path_buf();
    let frames_file = cache_dir.join("frames.bin");

    {
        let mut cache = DiskFrameCache::new(&cache_dir).unwrap();
        let frame = RawFrame {
            timestamp_ms: 0,
            width: 4,
            height: 4,
            bgra_data: vec![128; 4 * 4 * 4],
        };
        cache.append_frame(&frame).unwrap();
        assert!(frames_file.exists());
    }

    // After cache is dropped, frames.bin should be cleaned up
    assert!(!frames_file.exists());
}

#[test]
fn test_disk_cache_compression() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cache = DiskFrameCache::new(tmp.path()).unwrap();
    // 500x500 solid color image = 1,000,000 bytes uncompressed
    let raw_size = 500 * 500 * 4;
    let frame = RawFrame {
        timestamp_ms: 50,
        width: 500,
        height: 500,
        bgra_data: vec![42; raw_size],
    };
    cache.append_frame(&frame).unwrap();

    let loaded = cache.read_frame(0).unwrap();
    assert_eq!(loaded.bgra_data.len(), raw_size);
    assert_eq!(loaded.bgra_data, frame.bgra_data);

    // File size on disk should be much smaller than 1MB due to LZ4 compression
    let metadata = std::fs::metadata(tmp.path().join("frames.bin")).unwrap();
    assert!(metadata.len() < (raw_size as u64) / 10);
}

#[test]
fn test_disk_cache_open_existing_rebuild_index() {
    let tmp = tempfile::tempdir().unwrap();
    let bin_path = tmp.path().join("frames.bin");

    {
        let mut cache = DiskFrameCache::new(tmp.path()).unwrap();
        cache.set_delete_on_drop(false);

        for i in 0..5 {
            let frame = RawFrame {
                timestamp_ms: i * 50,
                width: 20,
                height: 20,
                bgra_data: vec![(i * 30) as u8; 20 * 20 * 4],
            };
            cache.append_frame(&frame).unwrap();
        }
        assert_eq!(cache.total_frames(), 5);
    }

    assert!(bin_path.exists());

    // Reopen existing frames.bin and verify all frames are indexed and readable
    let reopened = DiskFrameCache::open_existing(&bin_path).unwrap();
    assert_eq!(reopened.total_frames(), 5);

    for i in 0..5 {
        let frame = reopened.read_frame(i).unwrap();
        assert_eq!(frame.timestamp_ms, (i as u64) * 50);
        assert_eq!(frame.width, 20);
        assert_eq!(frame.height, 20);
        assert_eq!(frame.bgra_data, vec![(i * 30) as u8; 20 * 20 * 4]);
    }
}

#[test]
fn test_raw_frame_helpers() {
    let frame = RawFrame::new(1234, 64, 32, vec![0; 64 * 32 * 4]);
    assert_eq!(frame.timestamp_ms, 1234);
    assert_eq!(frame.width, 64);
    assert_eq!(frame.height, 32);
    assert_eq!(frame.expected_byte_len(), 64 * 32 * 4);
    assert!(frame.is_valid_size());

    let invalid_frame = RawFrame::new(0, 10, 10, vec![0; 50]);
    assert!(!invalid_frame.is_valid_size());
}
