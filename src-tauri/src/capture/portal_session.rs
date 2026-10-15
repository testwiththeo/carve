use crate::capture::traits::CaptureError;
use ashpd::desktop::screencast::{CursorMode, Screencast, SelectSourcesOptions, SourceType};
use ashpd::desktop::{PersistMode, ResponseError};
use std::os::fd::OwnedFd;

/// Information about a successfully negotiated Portal ScreenCast stream.
#[derive(Debug)]
pub struct PortalStreamInfo {
    pub fd: OwnedFd,
    pub node_id: u32,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl PortalStreamInfo {
    pub fn new(fd: OwnedFd, node_id: u32, width: Option<u32>, height: Option<u32>) -> Self {
        Self {
            fd,
            node_id,
            width,
            height,
        }
    }
}

/// Map an ashpd portal error into our domain `CaptureError`.
pub fn map_portal_error(err: ashpd::Error) -> CaptureError {
    match err {
        ashpd::Error::Response(ResponseError::Cancelled) => CaptureError::Cancelled,
        other => CaptureError::BackendError(other.to_string()),
    }
}

impl From<ashpd::Error> for CaptureError {
    fn from(err: ashpd::Error) -> Self {
        map_portal_error(err)
    }
}

/// Request a screencast session via XDG Desktop Portal using ashpd.
/// Uses default options: CursorMode::Embedded and persist: false.
pub async fn request_screencast_session() -> Result<PortalStreamInfo, CaptureError> {
    request_screencast_session_with_options(CursorMode::Embedded, false).await
}

/// Request a screencast session via XDG Desktop Portal using ashpd with custom options.
pub async fn request_screencast_session_with_options(
    cursor_mode: CursorMode,
    persist: bool,
) -> Result<PortalStreamInfo, CaptureError> {
    let proxy = Screencast::new().await?;
    let session = proxy.create_session(Default::default()).await?;

    let persist_mode = if persist {
        PersistMode::Application
    } else {
        PersistMode::DoNot
    };

    let sources = SourceType::Monitor | SourceType::Window;
    let select_options = SelectSourcesOptions::default()
        .set_cursor_mode(cursor_mode)
        .set_sources(sources)
        .set_multiple(false)
        .set_persist_mode(persist_mode);

    proxy.select_sources(&session, select_options).await?;

    let start_request = proxy.start(&session, None, Default::default()).await?;
    let response = start_request.response()?;

    let stream = response.streams().first().ok_or_else(|| {
        CaptureError::BackendError("Portal returned no screencast streams".to_string())
    })?;

    let (width, height) = match stream.size() {
        Some((w, h)) => (u32::try_from(w).ok(), u32::try_from(h).ok()),
        None => (None, None),
    };

    let node_id = stream.pipe_wire_node_id();

    let fd = proxy
        .open_pipe_wire_remote(&session, Default::default())
        .await?;

    Ok(PortalStreamInfo {
        fd,
        node_id,
        width,
        height,
    })
}

/// Synchronous / blocking wrapper for `request_screencast_session_with_options`.
pub fn request_screencast_session_blocking(
    cursor_mode: CursorMode,
    persist: bool,
) -> Result<PortalStreamInfo, CaptureError> {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => match handle.runtime_flavor() {
            tokio::runtime::RuntimeFlavor::MultiThread => tokio::task::block_in_place(|| {
                handle.block_on(request_screencast_session_with_options(
                    cursor_mode,
                    persist,
                ))
            }),
            _ => std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| {
                        CaptureError::BackendError(format!("Failed to create Tokio runtime: {e}"))
                    })?;
                rt.block_on(request_screencast_session_with_options(
                    cursor_mode,
                    persist,
                ))
            })
            .join()
            .map_err(|_| {
                CaptureError::BackendError(
                    "Blocking screencast request thread panicked".to_string(),
                )
            })?,
        },
        Err(_) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| {
                    CaptureError::BackendError(format!("Failed to create Tokio runtime: {e}"))
                })?;
            rt.block_on(request_screencast_session_with_options(
                cursor_mode,
                persist,
            ))
        }
    }
}
