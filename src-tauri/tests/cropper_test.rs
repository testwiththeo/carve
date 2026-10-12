use carve_lib::capture::{
    create_platform_capturer, crop_bgra, crop_bgra_into, CaptureError, CaptureRegion,
    LinuxPortalCapturer, MacosCapturer, MockCapturer, ScreenCapturer, WindowsCapturer,
};

#[test]
fn test_crop_bgra_subregion() {
    let src_w = 4;
    let src_h = 4;
    let mut buffer = Vec::new();
    for y in 0..src_h {
        for x in 0..src_w {
            buffer.extend_from_slice(&[x as u8, y as u8, 0, 255]);
        }
    }
    let region = CaptureRegion {
        x: 1,
        y: 1,
        width: 2,
        height: 2,
    };
    let cropped = crop_bgra(&buffer, src_w, src_h, &region).unwrap();
    assert_eq!(cropped.len(), 2 * 2 * 4);
    // (1,1) pixel
    assert_eq!(&cropped[0..4], &[1, 1, 0, 255]);
    // (2,1) pixel
    assert_eq!(&cropped[4..8], &[2, 1, 0, 255]);
    // (1,2) pixel
    assert_eq!(&cropped[8..12], &[1, 2, 0, 255]);
    // (2,2) pixel
    assert_eq!(&cropped[12..16], &[2, 2, 0, 255]);
}

#[test]
fn test_crop_bgra_full_frame() {
    let src_w = 3;
    let src_h = 3;
    let mut buffer = Vec::new();
    for i in 0..(src_w * src_h * 4) {
        buffer.push((i % 256) as u8);
    }
    let region = CaptureRegion {
        x: 0,
        y: 0,
        width: src_w,
        height: src_h,
    };
    let cropped = crop_bgra(&buffer, src_w, src_h, &region).unwrap();
    assert_eq!(cropped, buffer);
}

#[test]
fn test_crop_bgra_out_of_bounds() {
    let src_w = 10;
    let src_h = 10;
    let buffer = vec![0u8; (src_w * src_h * 4) as usize];

    // X out of bounds
    let region_oob_x = CaptureRegion {
        x: 8,
        y: 0,
        width: 3,
        height: 5,
    };
    let res_x = crop_bgra(&buffer, src_w, src_h, &region_oob_x);
    assert!(matches!(res_x, Err(CaptureError::OutOfBounds { .. })));

    // Y out of bounds
    let region_oob_y = CaptureRegion {
        x: 0,
        y: 8,
        width: 5,
        height: 3,
    };
    let res_y = crop_bgra(&buffer, src_w, src_h, &region_oob_y);
    assert!(matches!(res_y, Err(CaptureError::OutOfBounds { .. })));
}

#[test]
fn test_crop_bgra_zero_sized_region() {
    let src_w = 10;
    let src_h = 10;
    let buffer = vec![0u8; (src_w * src_h * 4) as usize];

    // Zero width
    let region_zero_w = CaptureRegion {
        x: 0,
        y: 0,
        width: 0,
        height: 5,
    };
    let res_w = crop_bgra(&buffer, src_w, src_h, &region_zero_w);
    assert!(matches!(res_w, Err(CaptureError::ZeroSizedRegion { .. })));

    // Zero height
    let region_zero_h = CaptureRegion {
        x: 0,
        y: 0,
        width: 5,
        height: 0,
    };
    let res_h = crop_bgra(&buffer, src_w, src_h, &region_zero_h);
    assert!(matches!(res_h, Err(CaptureError::ZeroSizedRegion { .. })));
}

#[test]
fn test_crop_bgra_into_reuse_buffer() {
    let src_w = 4;
    let src_h = 4;
    let mut buffer = Vec::new();
    for y in 0..src_h {
        for x in 0..src_w {
            buffer.extend_from_slice(&[x as u8, y as u8, 0, 255]);
        }
    }
    let region = CaptureRegion {
        x: 2,
        y: 2,
        width: 2,
        height: 2,
    };
    let mut dest = vec![0u8; 2 * 2 * 4];
    crop_bgra_into(&buffer, src_w, src_h, &region, &mut dest).unwrap();
    // (2,2)
    assert_eq!(&dest[0..4], &[2, 2, 0, 255]);
    // (3,2)
    assert_eq!(&dest[4..8], &[3, 2, 0, 255]);
    // (2,3)
    assert_eq!(&dest[8..12], &[2, 3, 0, 255]);
    // (3,3)
    assert_eq!(&dest[12..16], &[3, 3, 0, 255]);

    // Destination size mismatch
    let mut bad_dest = vec![0u8; 10];
    let res = crop_bgra_into(&buffer, src_w, src_h, &region, &mut bad_dest);
    assert!(matches!(
        res,
        Err(CaptureError::DestinationBufferMismatch { .. })
    ));
}

#[test]
fn test_crop_bgra_invalid_buffer_size() {
    let src_w = 4;
    let src_h = 4;
    let truncated_buffer = vec![0u8; 20]; // expected 64 bytes
    let region = CaptureRegion {
        x: 0,
        y: 0,
        width: 2,
        height: 2,
    };
    let res = crop_bgra(&truncated_buffer, src_w, src_h, &region);
    assert!(matches!(
        res,
        Err(CaptureError::InvalidBufferSize {
            expected: 64,
            actual: 20
        })
    ));
}

#[test]
fn test_mock_capturer_lifecycle() {
    let mut capturer = MockCapturer::new();
    assert!(!capturer.is_capturing());
    assert!(!capturer.is_paused());

    let region = CaptureRegion {
        x: 10,
        y: 20,
        width: 100,
        height: 80,
    };
    capturer.start_capture(region).unwrap();
    assert!(capturer.is_capturing());
    assert!(!capturer.is_paused());

    // Already capturing should error
    assert!(matches!(
        capturer.start_capture(region),
        Err(CaptureError::AlreadyActive)
    ));

    // Pause and resume
    capturer.pause_capture().unwrap();
    assert!(capturer.is_paused());
    capturer.resume_capture().unwrap();
    assert!(!capturer.is_paused());

    // Stop
    capturer.stop_capture().unwrap();
    assert!(!capturer.is_capturing());

    // Stop again should error
    assert!(matches!(
        capturer.stop_capture(),
        Err(CaptureError::NotActive)
    ));
}

#[test]
fn test_mock_capturer_frame_generation() {
    let mut capturer = MockCapturer::new();
    let region = CaptureRegion::new(0, 0, 10, 10);
    capturer.start_capture(region).unwrap();

    let frame1 = capturer.capture_frame().unwrap().expect("frame 1");
    assert_eq!(frame1.timestamp_ms, 0);
    assert_eq!(frame1.width, 10);
    assert_eq!(frame1.height, 10);
    assert_eq!(frame1.bgra_data.len(), 10 * 10 * 4);

    let frame2 = capturer.capture_frame().unwrap().expect("frame 2");
    assert_eq!(frame2.timestamp_ms, 33);
    assert_eq!(capturer.frame_count(), 2);

    capturer.pause_capture().unwrap();
    assert_eq!(capturer.capture_frame().unwrap(), None);

    capturer.resume_capture().unwrap();
    let frame3 = capturer.capture_frame().unwrap().expect("frame 3");
    assert_eq!(frame3.timestamp_ms, 66);
}

#[test]
fn test_platform_capturers_and_factory() {
    let mut default_capturer = create_platform_capturer();
    assert!(!default_capturer.is_capturing());
    assert!(!default_capturer.is_paused());
    #[cfg(target_os = "linux")]
    {
        let res = default_capturer.start_capture(CaptureRegion::new(0, 0, 50, 50));
        if res.is_ok() {
            assert!(default_capturer.is_capturing());
            default_capturer.stop_capture().unwrap();
            assert!(!default_capturer.is_capturing());
        }
    }

    // Linux adapter structure
    let mut linux = LinuxPortalCapturer::with_session("test-portal-session".into(), 42);
    assert_eq!(linux.portal_session_id(), Some("test-portal-session"));
    assert_eq!(linux.pipewire_node_id(), Some(42));
    #[cfg(target_os = "linux")]
    {
        linux
            .start_capture(CaptureRegion::new(0, 0, 100, 100))
            .unwrap();
        assert!(linux.is_capturing());
        linux.stop_capture().unwrap();
        assert!(!linux.is_capturing());
    }

    // macOS adapter structure
    let mut macos = MacosCapturer::new();
    #[cfg(target_os = "macos")]
    {
        macos
            .start_capture(CaptureRegion::new(0, 0, 100, 100))
            .unwrap();
        assert!(macos.is_capturing());
        macos.stop_capture().unwrap();
    }
    #[cfg(not(target_os = "macos"))]
    {
        let res = macos.start_capture(CaptureRegion::new(0, 0, 100, 100));
        assert!(matches!(res, Err(CaptureError::BackendError(_))));
    }

    // Windows adapter structure
    let mut windows = WindowsCapturer::new();
    #[cfg(target_os = "windows")]
    {
        windows
            .start_capture(CaptureRegion::new(0, 0, 100, 100))
            .unwrap();
        assert!(windows.is_capturing());
        windows.stop_capture().unwrap();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let res = windows.start_capture(CaptureRegion::new(0, 0, 100, 100));
        assert!(matches!(res, Err(CaptureError::BackendError(_))));
    }
}

