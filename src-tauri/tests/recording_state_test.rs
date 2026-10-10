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
