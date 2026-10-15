#[cfg(target_os = "linux")]
mod linux_portal_session_tests {
    use ashpd::desktop::screencast::CursorMode;
    use ashpd::desktop::ResponseError;
    use carve_lib::capture::portal_session::{
        map_portal_error, request_screencast_session, request_screencast_session_blocking,
        request_screencast_session_with_options, PortalStreamInfo,
    };
    use carve_lib::capture::traits::CaptureError;
    use std::fs::File;
    use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd};

    #[test]
    fn test_portal_stream_info_construction() {
        let file = File::open("/dev/null").expect("failed to open /dev/null");
        let raw_fd = file.into_raw_fd();
        let fd = unsafe { OwnedFd::from_raw_fd(raw_fd) };

        let info = PortalStreamInfo {
            fd,
            node_id: 42,
            width: Some(1920),
            height: Some(1080),
        };

        assert_eq!(info.node_id, 42);
        assert_eq!(info.width, Some(1920));
        assert_eq!(info.height, Some(1080));
        assert!(info.fd.as_raw_fd() >= 0);
    }

    #[test]
    fn test_portal_stream_info_optional_dimensions() {
        let file = File::open("/dev/null").expect("failed to open /dev/null");
        let raw_fd = file.into_raw_fd();
        let fd = unsafe { OwnedFd::from_raw_fd(raw_fd) };

        let info = PortalStreamInfo {
            fd,
            node_id: 101,
            width: None,
            height: None,
        };

        assert_eq!(info.node_id, 101);
        assert_eq!(info.width, None);
        assert_eq!(info.height, None);
    }

    #[test]
    fn test_portal_stream_info_new_helper() {
        let file = File::open("/dev/null").expect("failed to open /dev/null");
        let raw_fd = file.into_raw_fd();
        let fd = unsafe { OwnedFd::from_raw_fd(raw_fd) };

        let info = PortalStreamInfo::new(fd, 77, Some(1280), Some(720));
        assert_eq!(info.node_id, 77);
        assert_eq!(info.width, Some(1280));
        assert_eq!(info.height, Some(720));

        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("PortalStreamInfo"));
        assert!(debug_str.contains("77"));
    }

    #[test]
    fn test_cancellation_mapping() {
        let cancel_err = ashpd::Error::Response(ResponseError::Cancelled);
        let mapped = map_portal_error(cancel_err);
        assert_eq!(mapped, CaptureError::Cancelled);
    }

    #[test]
    fn test_error_conversion_from_trait() {
        let cancel_err = ashpd::Error::Response(ResponseError::Cancelled);
        let mapped: CaptureError = cancel_err.into();
        assert_eq!(mapped, CaptureError::Cancelled);
    }

    #[test]
    fn test_other_error_mapping() {
        let other_err = ashpd::Error::Response(ResponseError::Other);
        let mapped = map_portal_error(other_err);
        match mapped {
            CaptureError::BackendError(msg) => {
                assert!(!msg.is_empty());
            }
            other => panic!("Expected BackendError, got {:?}", other),
        }
    }

    #[test]
    fn test_request_screencast_session_future_signature() {
        // Compile-time check that request_screencast_session exists with 0 arguments and returns the expected future
        fn assert_future<F>(_fut: &F)
        where
            F: std::future::Future<Output = Result<PortalStreamInfo, CaptureError>>,
        {
        }

        let fut = request_screencast_session();
        assert_future(&fut);
    }

    #[test]
    fn test_request_screencast_session_with_options_future_signature() {
        // Compile-time check that request_screencast_session_with_options takes cursor_mode and persist
        fn assert_future<F>(_fut: &F)
        where
            F: std::future::Future<Output = Result<PortalStreamInfo, CaptureError>>,
        {
        }

        let fut = request_screencast_session_with_options(CursorMode::Hidden, false);
        assert_future(&fut);
    }

    #[test]
    fn test_request_screencast_session_blocking_inside_runtime_does_not_panic() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to build current thread runtime");

        rt.block_on(async {
            // Calling blocking inside the runtime should delegate safely without panicking on rt.block_on
            let res = request_screencast_session_blocking(CursorMode::Embedded, false);
            assert!(res.is_ok() || res.is_err());
        });
    }

    #[test]
    fn test_request_screencast_session_blocking_inside_multithread_runtime_does_not_panic() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build();
        if let Ok(rt) = rt {
            rt.block_on(async {
                let res = request_screencast_session_blocking(CursorMode::Embedded, false);
                assert!(res.is_ok() || res.is_err());
            });
        }
    }
}
