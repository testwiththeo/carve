#[cfg(target_os = "linux")]
mod pipewire_capture_tests {
    use carve_lib::capture::pipewire_stream::{convert_format_to_bgra, PipeWireStreamConsumer};
    use carve_lib::capture::traits::{CaptureError, CaptureRegion, ScreenCapturer};
    use carve_lib::capture::LinuxPortalCapturer;
    use carve_lib::storage::RawFrame;
    use pipewire::spa::param::video::VideoFormat;

    fn generate_test_frame(timestamp_ms: u64, width: u32, height: u32) -> RawFrame {
        let mut bgra = vec![0u8; (width as usize) * (height as usize) * 4];
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                bgra[idx] = (x % 256) as u8; // B
                bgra[idx + 1] = (y % 256) as u8; // G
                bgra[idx + 2] = ((x + y) % 256) as u8; // R
                bgra[idx + 3] = 255; // A
            }
        }
        RawFrame::new(timestamp_ms, width, height, bgra)
    }

    #[test]
    fn test_linux_portal_capturer_initial_state() {
        let capturer = LinuxPortalCapturer::new();
        assert!(!capturer.is_capturing());
        assert!(!capturer.is_paused());
        assert_eq!(capturer.current_region(), None);
    }

    #[test]
    fn test_linux_portal_capturer_lifecycle_transitions() {
        let (mut capturer, _sender) = LinuxPortalCapturer::new_mock();
        assert!(!capturer.is_capturing());
        assert!(!capturer.is_paused());

        // Zero-sized region must fail
        let zero_region = CaptureRegion::new(0, 0, 0, 100);
        assert!(matches!(
            capturer.start_capture(zero_region),
            Err(CaptureError::ZeroSizedRegion { .. })
        ));

        // Start capture successfully
        let valid_region = CaptureRegion::new(0, 0, 640, 480);
        assert!(capturer.start_capture(valid_region).is_ok());
        assert!(capturer.is_capturing());
        assert!(!capturer.is_paused());
        assert_eq!(capturer.current_region(), Some(valid_region));

        // Double start must fail
        assert_eq!(
            capturer.start_capture(valid_region),
            Err(CaptureError::AlreadyActive)
        );

        // Pause capture
        assert!(capturer.pause_capture().is_ok());
        assert!(capturer.is_paused());
        assert!(capturer.is_capturing());

        // Double pause must fail
        assert_eq!(capturer.pause_capture(), Err(CaptureError::Paused));

        // Resume capture
        assert!(capturer.resume_capture().is_ok());
        assert!(!capturer.is_paused());
        assert!(capturer.is_capturing());

        // Double resume must fail
        assert_eq!(capturer.resume_capture(), Err(CaptureError::NotPaused));

        // Stop capture
        assert!(capturer.stop_capture().is_ok());
        assert!(!capturer.is_capturing());
        assert!(!capturer.is_paused());
        assert_eq!(capturer.current_region(), None);

        // Stop when already stopped must fail
        assert_eq!(capturer.stop_capture(), Err(CaptureError::NotActive));

        // Pause/resume when stopped must fail
        assert_eq!(capturer.pause_capture(), Err(CaptureError::NotActive));
        assert_eq!(capturer.resume_capture(), Err(CaptureError::NotActive));
    }

    #[test]
    fn test_linux_portal_capturer_frame_receiving_full_frame() {
        let (mut capturer, sender) = LinuxPortalCapturer::new_mock();
        let region = CaptureRegion::new(0, 0, 320, 240);
        capturer.start_capture(region).expect("start_capture failed");

        // Before sending any frames, capture_frame returns Ok(None)
        assert_eq!(capturer.capture_frame().unwrap(), None);

        // Send a frame matching region dimensions
        let frame = generate_test_frame(100, 320, 240);
        sender.try_send(frame.clone()).expect("try_send failed");

        // capture_frame should return the frame
        let received = capturer.capture_frame().unwrap().expect("expected a frame");
        assert_eq!(received.timestamp_ms, 100);
        assert_eq!(received.width, 320);
        assert_eq!(received.height, 240);
        assert_eq!(received.bgra_data, frame.bgra_data);

        // Next call should be empty
        assert_eq!(capturer.capture_frame().unwrap(), None);

        capturer.stop_capture().expect("stop_capture failed");
    }

    #[test]
    fn test_linux_portal_capturer_subregion_cropping() {
        let (mut capturer, sender) = LinuxPortalCapturer::new_mock();
        // Crop a 100x80 subregion at offset (50, 40)
        let region = CaptureRegion::new(50, 40, 100, 80);
        capturer.start_capture(region).expect("start_capture failed");

        // Source frame is 640x480
        let src_frame = generate_test_frame(250, 640, 480);
        sender.try_send(src_frame.clone()).expect("try_send failed");

        let cropped = capturer.capture_frame().unwrap().expect("expected cropped frame");
        assert_eq!(cropped.timestamp_ms, 250);
        assert_eq!(cropped.width, 100);
        assert_eq!(cropped.height, 80);
        assert_eq!(cropped.bgra_data.len(), 100 * 80 * 4);

        // Verify corner pixels match source at offset
        // Cropped pixel (0, 0) should equal source pixel (50, 40)
        let src_top_left_idx = ((40 * 640 + 50) * 4) as usize;
        assert_eq!(
            &cropped.bgra_data[0..4],
            &src_frame.bgra_data[src_top_left_idx..src_top_left_idx + 4]
        );

        // Cropped pixel (99, 79) should equal source pixel (149, 119)
        let src_bottom_right_idx = (((40 + 79) * 640 + (50 + 99)) * 4) as usize;
        let crop_bottom_right_idx = ((79 * 100 + 99) * 4) as usize;
        assert_eq!(
            &cropped.bgra_data[crop_bottom_right_idx..crop_bottom_right_idx + 4],
            &src_frame.bgra_data[src_bottom_right_idx..src_bottom_right_idx + 4]
        );

        capturer.stop_capture().expect("stop_capture failed");
    }

    #[test]
    fn test_linux_portal_capturer_crop_out_of_bounds() {
        let (mut capturer, sender) = LinuxPortalCapturer::new_mock();
        // Region exceeds incoming frame dimensions
        let region = CaptureRegion::new(100, 100, 200, 200);
        capturer.start_capture(region).expect("start_capture failed");

        // Incoming frame is only 150x150, so crop region (100+200=300 > 150) is out of bounds
        let small_frame = generate_test_frame(300, 150, 150);
        sender.try_send(small_frame).expect("try_send failed");

        let err = capturer.capture_frame().unwrap_err();
        assert!(matches!(err, CaptureError::OutOfBounds { .. }));

        capturer.stop_capture().expect("stop_capture failed");
    }

    #[test]
    fn test_linux_portal_capturer_paused_suppresses_frames() {
        let (mut capturer, sender) = LinuxPortalCapturer::new_mock();
        let region = CaptureRegion::new(0, 0, 100, 100);
        capturer.start_capture(region).expect("start_capture failed");

        capturer.pause_capture().expect("pause failed");
        assert!(capturer.is_paused());

        let frame = generate_test_frame(500, 100, 100);
        sender.try_send(frame).expect("try_send failed");

        // When paused, capture_frame returns Ok(None)
        assert_eq!(capturer.capture_frame().unwrap(), None);

        // Resume and stale frames are flushed
        capturer.resume_capture().expect("resume failed");
        assert!(!capturer.is_paused());
        assert_eq!(capturer.capture_frame().unwrap(), None);

        // Subsequent frame sent after resume is received
        let fresh_frame = generate_test_frame(600, 100, 100);
        sender.try_send(fresh_frame).expect("try_send failed");
        let received = capturer.capture_frame().unwrap().expect("expected fresh frame");
        assert_eq!(received.timestamp_ms, 600);

        capturer.stop_capture().expect("stop_capture failed");
    }

    #[test]
    fn test_linux_portal_capturer_flush_on_resume() {
        let (mut capturer, sender) = LinuxPortalCapturer::new_mock();
        let region = CaptureRegion::new(0, 0, 100, 100);
        capturer.start_capture(region).expect("start_capture failed");

        // Send a frame while running
        sender
            .try_send(generate_test_frame(100, 100, 100))
            .expect("try_send failed");

        // Pause capture
        capturer.pause_capture().expect("pause failed");

        // Send frame during pause
        sender
            .try_send(generate_test_frame(200, 100, 100))
            .expect("try_send failed");

        // While paused, capture_frame returns None
        assert_eq!(capturer.capture_frame().unwrap(), None);

        // Resume capture flushes the stale frame
        capturer.resume_capture().expect("resume failed");
        assert_eq!(capturer.capture_frame().unwrap(), None);

        // Send new frame after resume
        sender
            .try_send(generate_test_frame(300, 100, 100))
            .expect("try_send failed");
        let frame = capturer.capture_frame().unwrap().expect("expected frame");
        assert_eq!(frame.timestamp_ms, 300);

        capturer.stop_capture().expect("stop_capture failed");
    }

    #[test]
    fn test_pipewire_stream_consumer_freshest_frame_semantics() {
        let (consumer, sender) = PipeWireStreamConsumer::new_mock();

        let frame1 = generate_test_frame(10, 64, 64);
        let frame2 = generate_test_frame(20, 64, 64);
        let frame3 = generate_test_frame(30, 64, 64);

        // Sending frames successively overwrites older frames
        assert!(sender.try_send(frame1).is_ok());
        assert!(sender.try_send(frame2).is_ok());
        assert!(sender.try_send(frame3).is_ok());

        // Drain / receive freshest frame (timestamp 30)
        let latest = consumer.recv_frame().expect("expected frame from consumer");
        assert_eq!(latest.timestamp_ms, 30);

        // Queue is now empty
        assert!(consumer.recv_frame().is_none());

        // Test flush clears queued frame
        let frame4 = generate_test_frame(40, 64, 64);
        assert!(sender.try_send(frame4).is_ok());
        consumer.flush();
        assert!(consumer.recv_frame().is_none());
    }

    #[test]
    fn test_pipewire_format_conversion_rgbx_and_others() {
        // RGBx: [R, G, B, X] -> [B, G, R, 255]
        let mut rgbx_buf = vec![10, 20, 30, 0, 100, 150, 200, 50];
        convert_format_to_bgra(&mut rgbx_buf, VideoFormat::RGBx);
        assert_eq!(&rgbx_buf[0..4], &[30, 20, 10, 255]);
        assert_eq!(&rgbx_buf[4..8], &[200, 150, 100, 255]);

        // RGBA: [R, G, B, A] -> [B, G, R, A]
        let mut rgba_buf = vec![10, 20, 30, 128];
        convert_format_to_bgra(&mut rgba_buf, VideoFormat::RGBA);
        assert_eq!(&rgba_buf, &[30, 20, 10, 128]);

        // BGRx: [B, G, R, X] -> [B, G, R, 255]
        let mut bgrx_buf = vec![30, 20, 10, 0];
        convert_format_to_bgra(&mut bgrx_buf, VideoFormat::BGRx);
        assert_eq!(&bgrx_buf, &[30, 20, 10, 255]);

        // BGRA: [B, G, R, A] -> unchanged
        let mut bgra_buf = vec![30, 20, 10, 200];
        convert_format_to_bgra(&mut bgra_buf, VideoFormat::BGRA);
        assert_eq!(&bgra_buf, &[30, 20, 10, 200]);
    }

    #[test]
    fn test_linux_portal_capturer_propagates_portal_error() {
        let mut capturer = LinuxPortalCapturer::new(); // is_mock is false
        assert!(!capturer.is_mock());

        let res = capturer.start_capture(CaptureRegion::new(0, 0, 100, 100));
        // On a headless or live system without interactive user portal authorization,
        // start_capture must propagate the actual CaptureError rather than falling back to mock
        assert!(res.is_err());
        assert!(!capturer.is_capturing());
    }

    #[test]
    fn test_pipewire_stream_consumer_stop_signal() {
        let (mut consumer, _sender) = PipeWireStreamConsumer::new_mock();
        assert!(!consumer.is_stopped());
        consumer.stop();
        assert!(consumer.is_stopped());
    }
}
