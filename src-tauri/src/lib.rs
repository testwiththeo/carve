use std::sync::Mutex;

pub mod capture;
pub mod encoder;
pub mod export;
pub mod model;
pub mod storage;
pub mod window_manager;

pub use export::{
    composite_frame_with_vectors, encode_raw_frames, export_session_to_gif,
    run_mock_export_pipeline, ExportError, RawCompositedFrame, SessionState,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(std::sync::Arc::new(Mutex::new(
            window_manager::RecordingState::Idle,
        )))
        .manage(export::SessionState::default())
        .invoke_handler(tauri::generate_handler![
            window_manager::commands::start_recording,
            window_manager::commands::stop_recording,
            window_manager::commands::pause_recording,
            window_manager::commands::resume_recording,
            window_manager::commands::set_overlay_clickthrough,
            export::commands::export_gif,
            export::commands::render_frame_with_vectors,
            export::commands::update_vector_ledger,
            export::commands::encode_raw_frames,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
