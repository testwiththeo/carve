use std::sync::Mutex;

pub mod capture;
pub mod encoder;
pub mod model;
pub mod storage;
pub mod window_manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(window_manager::RecordingState::Idle))
        .invoke_handler(tauri::generate_handler![
            window_manager::commands::start_recording,
            window_manager::commands::stop_recording,
            window_manager::commands::pause_recording,
            window_manager::commands::resume_recording,
            window_manager::commands::set_overlay_clickthrough,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
