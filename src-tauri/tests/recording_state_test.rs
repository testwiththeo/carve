use carve_lib::capture::CaptureRegion;
use carve_lib::window_manager::{RecordingError, RecordingState};

#[test]
fn test_recording_state_transitions() {
    let mut state = RecordingState::Idle;
    state
        .start(CaptureRegion {
            x: 0,
            y: 0,
            width: 200,
            height: 200,
        })
        .unwrap();
    assert!(state.is_recording());
    state.stop().unwrap();
    assert!(!state.is_recording());
}

#[test]
fn test_recording_invalid_region() {
    let mut state = RecordingState::Idle;
    // Width 0
    let err = state
        .start(CaptureRegion {
            x: 0,
            y: 0,
            width: 0,
            height: 200,
        })
        .unwrap_err();
    assert_eq!(err, RecordingError::InvalidRegion);
    assert!(!state.is_recording());

    // Height 0
    let err2 = state
        .start(CaptureRegion {
            x: 10,
            y: 10,
            width: 100,
            height: 0,
        })
        .unwrap_err();
    assert_eq!(err2, RecordingError::InvalidRegion);
    assert!(!state.is_recording());
}

#[test]
fn test_recording_double_start() {
    let mut state = RecordingState::Idle;
    state
        .start(CaptureRegion {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        })
        .unwrap();
    let err = state
        .start(CaptureRegion {
            x: 0,
            y: 0,
            width: 50,
            height: 50,
        })
        .unwrap_err();
    assert_eq!(err, RecordingError::AlreadyRecording);
    assert!(state.is_recording());
}

#[test]
fn test_recording_stop_when_idle() {
    let mut state = RecordingState::Idle;
    let err = state.stop().unwrap_err();
    assert_eq!(err, RecordingError::NotRecording);
}

#[test]
fn test_recording_pause_resume_transitions() {
    let mut state = RecordingState::Idle;

    // Cannot pause while idle
    let pause_idle_err = state.pause().unwrap_err();
    assert_eq!(pause_idle_err, RecordingError::NotRecording);

    // Cannot resume while idle
    let resume_idle_err = state.resume().unwrap_err();
    assert_eq!(resume_idle_err, RecordingError::NotRecording);

    let region = CaptureRegion {
        x: 10,
        y: 20,
        width: 300,
        height: 400,
    };
    state.start(region).unwrap();
    assert!(state.is_recording());
    assert!(!state.is_paused());
    assert_eq!(state.region(), Some(&region));

    // Cannot resume while actively recording
    let resume_rec_err = state.resume().unwrap_err();
    assert_eq!(resume_rec_err, RecordingError::NotPaused);

    // Pause recording
    state.pause().unwrap();
    assert!(!state.is_recording());
    assert!(state.is_paused());
    assert_eq!(state.region(), Some(&region));

    // Cannot pause while already paused
    let pause_again_err = state.pause().unwrap_err();
    assert_eq!(pause_again_err, RecordingError::NotActiveRecording);

    // Resume recording
    state.resume().unwrap();
    assert!(state.is_recording());
    assert!(!state.is_paused());

    // Pause again, then stop directly
    state.pause().unwrap();
    assert!(state.is_paused());
    let stopped_region = state.stop().unwrap();
    assert_eq!(stopped_region, region);
    assert!(!state.is_recording());
    assert!(!state.is_paused());
    assert_eq!(state.region(), None);
}

// -----------------------------------------------------------------------------
// RequestingPortal phase: portal-aware recording state machine
// -----------------------------------------------------------------------------

mod requesting_portal_phase_tests {
    use carve_lib::capture::CaptureRegion;
    use carve_lib::window_manager::{RecordingError, RecordingState};

    fn region() -> CaptureRegion {
        CaptureRegion::new(5, 5, 320, 240)
    }

    #[test]
    fn idle_to_requesting_portal_transition() {
        let mut state = RecordingState::new();
        state.begin_portal_request(region()).unwrap();
        assert!(state.is_requesting_portal());
        assert!(!state.is_recording());
        assert!(!state.is_paused());
        assert_eq!(state.region(), Some(&region()));
    }

    #[test]
    fn begin_portal_request_rejects_invalid_region_and_stays_idle() {
        let mut state = RecordingState::new();
        let err = state
            .begin_portal_request(CaptureRegion::new(0, 0, 0, 100))
            .unwrap_err();
        assert_eq!(err, RecordingError::InvalidRegion);
        assert!(state.is_idle());
    }

    #[test]
    fn begin_portal_request_rejected_while_session_active() {
        let mut state = RecordingState::new();
        state.begin_portal_request(region()).unwrap();
        let err = state.begin_portal_request(region()).unwrap_err();
        assert_eq!(err, RecordingError::AlreadyRecording);
        assert!(state.is_requesting_portal());
    }

    #[test]
    fn requesting_portal_to_recording_on_portal_acquired() {
        let mut state = RecordingState::new();
        state.begin_portal_request(region()).unwrap();
        state.portal_acquired().unwrap();
        assert!(state.is_recording());
        assert!(!state.is_requesting_portal());
        assert!(!state.is_paused());
        assert_eq!(state.region(), Some(&region()));
    }

    #[test]
    fn portal_acquired_without_pending_request_fails() {
        let mut state = RecordingState::new();
        assert_eq!(
            state.portal_acquired(),
            Err(RecordingError::NotRequestingPortal)
        );

        // Also rejected once the session is already recording.
        state.start(region()).unwrap();
        assert_eq!(
            state.portal_acquired(),
            Err(RecordingError::NotRequestingPortal)
        );
    }

    #[test]
    fn pause_rejected_while_requesting_portal() {
        let mut state = RecordingState::new();
        state.begin_portal_request(region()).unwrap();
        assert_eq!(state.pause(), Err(RecordingError::NotActiveRecording));
        assert!(state.is_requesting_portal());
    }

    #[test]
    fn resume_rejected_while_requesting_portal() {
        let mut state = RecordingState::new();
        state.begin_portal_request(region()).unwrap();
        assert_eq!(state.resume(), Err(RecordingError::NotPaused));
    }

    #[test]
    fn stop_while_requesting_portal_returns_region_and_goes_idle() {
        let mut state = RecordingState::new();
        state.begin_portal_request(region()).unwrap();
        let stopped = state.stop().unwrap();
        assert_eq!(stopped, region());
        assert!(state.is_idle());
    }

    #[test]
    fn cancel_portal_request_rolls_back_to_idle() {
        let mut state = RecordingState::new();
        state.begin_portal_request(region()).unwrap();
        state.cancel_portal_request().unwrap();
        assert!(state.is_idle());

        // Cancelling again without a pending request is an error.
        assert_eq!(
            state.cancel_portal_request(),
            Err(RecordingError::NotRequestingPortal)
        );
    }

    #[test]
    fn full_lifecycle_idle_requesting_recording_stopped() {
        let mut state = RecordingState::new();
        assert!(state.is_idle());

        state.begin_portal_request(region()).unwrap();
        assert!(state.is_requesting_portal());

        state.portal_acquired().unwrap();
        assert!(state.is_recording());

        let stopped = state.stop().unwrap();
        assert_eq!(stopped, region());
        assert!(state.is_idle());
        assert!(!state.is_recording());
        assert!(!state.is_requesting_portal());
    }
}

// -----------------------------------------------------------------------------
// RecordingError boundary shape: what the frontend actually receives
// -----------------------------------------------------------------------------

mod error_payload_tests {
    use carve_lib::capture::CaptureError;
    use carve_lib::window_manager::RecordingError;

    #[test]
    fn unit_variant_serializes_as_flat_kind_and_message() {
        let payload = serde_json::to_value(RecordingError::AlreadyRecording).unwrap();
        assert_eq!(
            payload,
            serde_json::json!({
                "kind": "already_recording",
                "message": "Recording already in progress",
            })
        );
    }

    #[test]
    fn message_payload_carries_variant_display_message() {
        let payload = serde_json::to_value(RecordingError::Internal("boom".to_string())).unwrap();
        assert_eq!(
            payload,
            serde_json::json!({
                "kind": "internal",
                "message": "Internal error: boom",
            })
        );

        let payload =
            serde_json::to_value(RecordingError::WindowError("gone".to_string())).unwrap();
        assert_eq!(payload["kind"], "window_error");
        assert_eq!(payload["message"], "Window error: gone");
    }

    #[test]
    fn capture_error_surfaces_through_the_boundary() {
        let source = CaptureError::InvalidBufferSize {
            expected: 128,
            actual: 64,
        };
        let err: RecordingError = source.clone().into();
        assert_eq!(err, RecordingError::Capture(source.clone()));

        let payload = serde_json::to_value(&err).unwrap();
        assert_eq!(payload["kind"], "capture");
        assert_eq!(payload["message"], source.to_string());
    }

    #[test]
    fn kind_tags_are_stable_and_distinct() {
        let kinds = [
            RecordingError::InvalidRegion,
            RecordingError::AlreadyRecording,
            RecordingError::NotRecording,
            RecordingError::NotActiveRecording,
            RecordingError::NotPaused,
            RecordingError::NotRequestingPortal,
            RecordingError::WindowError(String::new()),
            RecordingError::Capture(CaptureError::NotActive),
            RecordingError::Internal(String::new()),
        ]
        .map(|e| e.kind().to_string());

        for i in 0..kinds.len() {
            assert!(!kinds[i].is_empty());
            for j in (i + 1)..kinds.len() {
                assert_ne!(kinds[i], kinds[j], "kind tags must be distinct");
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Portal acquisition integration: recording controller end-to-end (Linux)
// -----------------------------------------------------------------------------

#[cfg(target_os = "linux")]
mod portal_recording_session_tests {
    use std::sync::{Arc, Barrier, Mutex};

    use carve_lib::capture::portal_session::PortalStreamInfo;
    use carve_lib::capture::traits::ScreenCapturer;
    use carve_lib::capture::{CaptureError, CaptureRegion, LinuxPortalCapturer};
    use carve_lib::storage::RawFrame;
    use carve_lib::window_manager::{
        start_recording_session, CaptureAcquisition, CaptureBackend, PortalAcquisition,
        RecordingState,
    };

    fn region() -> CaptureRegion {
        CaptureRegion::new(0, 0, 320, 240)
    }

    /// A structurally valid stream info; tests never touch the fd because the
    /// connect seam is injected.
    fn dummy_stream_info() -> PortalStreamInfo {
        let fd = std::fs::File::open("/dev/null").unwrap().into();
        PortalStreamInfo::new(fd, 42, Some(320), Some(240))
    }

    fn solid_frame(timestamp_ms: u64) -> RawFrame {
        RawFrame::new(timestamp_ms, 320, 240, vec![7u8; 320 * 240 * 4])
    }

    fn unreachable_connect(
        info: PortalStreamInfo,
    ) -> Result<Box<dyn ScreenCapturer>, CaptureError> {
        Err(CaptureError::BackendError(format!(
            "connect must not run after a failed request (node {})",
            info.node_id
        )))
    }

    #[test]
    fn portal_stream_acquisition_flips_state_to_recording_and_frames_flow() {
        let (portal_capturer, sender) = LinuxPortalCapturer::new_mock();
        let acquisition = CaptureAcquisition::Portal(PortalAcquisition::new(
            || Ok(dummy_stream_info()),
            move |_| Ok(Box::new(portal_capturer) as Box<dyn ScreenCapturer>),
        ));
        let shared = Mutex::new(RecordingState::Idle);

        let mut start =
            start_recording_session(&shared, region(), acquisition).expect("session should start");
        assert_eq!(start.backend, CaptureBackend::PortalPipeWire);

        {
            let state = shared.lock().unwrap();
            assert!(state.is_recording());
            assert!(!state.is_requesting_portal());
            assert_eq!(state.region(), Some(&region()));
        }

        // Frames flow from the portal-backed stream once recording begins.
        sender.try_send(solid_frame(120)).expect("try_send failed");
        let frame = start
            .capturer
            .capture_frame()
            .expect("capture_frame errored")
            .expect("a frame should flow");
        assert_eq!(frame.timestamp_ms, 120);
        assert_eq!((frame.width, frame.height), (320, 240));

        let stopped = shared.lock().unwrap().stop().expect("stop failed");
        assert_eq!(stopped, region());
        assert!(shared.lock().unwrap().is_idle());
    }

    #[test]
    fn user_denied_portal_prompt_yields_clean_cancelled_and_idle_state() {
        let acquisition = CaptureAcquisition::Portal(PortalAcquisition::new(
            || Err(CaptureError::Cancelled),
            unreachable_connect,
        ));
        let shared = Mutex::new(RecordingState::Idle);

        let result = start_recording_session(&shared, region(), acquisition);
        // Clean, typed cancellation — never a panic, never a wrapped backend error.
        assert_eq!(result.unwrap_err(), CaptureError::Cancelled);
        assert!(shared.lock().unwrap().is_idle());

        // The state machine stays usable for a subsequent attempt.
        shared
            .lock()
            .unwrap()
            .begin_portal_request(CaptureRegion::new(0, 0, 10, 10))
            .expect("state machine should be reusable after denial");
    }

    #[test]
    fn portal_unavailable_falls_back_to_mock_capture() {
        let acquisition = CaptureAcquisition::Portal(PortalAcquisition::new(
            || Err(CaptureError::BackendError("no portal service".to_string())),
            unreachable_connect,
        ));
        let shared = Mutex::new(RecordingState::Idle);

        let mut start = start_recording_session(&shared, region(), acquisition)
            .expect("unavailable portal should degrade, not fail");
        assert_eq!(start.backend, CaptureBackend::Mock);
        assert!(shared.lock().unwrap().is_recording());
        assert!(start.capturer.capture_frame().unwrap().is_some());

        shared.lock().unwrap().stop().unwrap();
    }

    #[test]
    fn headless_environment_records_via_mock_capture() {
        let shared = Mutex::new(RecordingState::Idle);
        let mut start = start_recording_session(&shared, region(), CaptureAcquisition::Headless)
            .expect("headless start should succeed");
        assert_eq!(start.backend, CaptureBackend::Mock);
        assert!(shared.lock().unwrap().is_recording());
        assert!(start.capturer.capture_frame().unwrap().is_some());

        shared.lock().unwrap().stop().unwrap();
    }

    #[test]
    fn connect_failure_after_successful_request_aborts_cleanly() {
        let acquisition = CaptureAcquisition::Portal(PortalAcquisition::new(
            || Ok(dummy_stream_info()),
            |_| {
                Err(CaptureError::BackendError(
                    "pipewire connect failed".to_string(),
                ))
            },
        ));
        let shared = Mutex::new(RecordingState::Idle);

        let result = start_recording_session(&shared, region(), acquisition);
        assert_eq!(
            result.unwrap_err(),
            CaptureError::BackendError("pipewire connect failed".to_string())
        );
        assert!(shared.lock().unwrap().is_idle());
    }

    #[test]
    fn portal_prompt_is_asynchronous_requesting_phase_observable_from_another_thread() {
        let (portal_capturer, _sender) = LinuxPortalCapturer::new_mock();
        let gate = Arc::new(Barrier::new(2));
        let request_gate = gate.clone();

        // The "portal prompt" blocks until the main thread has inspected the state.
        let request = move || {
            request_gate.wait();
            Ok(dummy_stream_info())
        };
        let connect = move |_| Ok(Box::new(portal_capturer) as Box<dyn ScreenCapturer>);

        let shared = Arc::new(Mutex::new(RecordingState::Idle));
        let worker_state = shared.clone();
        let handle = std::thread::spawn(move || {
            start_recording_session(
                &worker_state,
                region(),
                CaptureAcquisition::Portal(PortalAcquisition::new(request, connect)),
            )
        });

        // The request seam has been entered: from the outside the controller must
        // report RequestingPortal (prompt pending), not Recording.
        gate.wait();
        {
            let state = shared.lock().unwrap();
            assert!(state.is_requesting_portal());
            assert!(!state.is_recording());
        }

        let start = handle.join().unwrap().expect("session should start");
        assert_eq!(start.backend, CaptureBackend::PortalPipeWire);
        assert!(shared.lock().unwrap().is_recording());
    }
}
