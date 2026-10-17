mod library;
mod playback;
use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(std::sync::Arc::new(library::LibraryState::default()))
        .manage(std::sync::Arc::new(playback::PlaybackManager::default()))
        .invoke_handler(tauri::generate_handler![
            greet,
            library::load_library,
            library::playlist_control,
            library::set_track_favorite,
            library::select_music_folder,
            library::scan_music,
            library::cancel_music_scan,
            library::get_music_artwork,
            playback::playback_control
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                app.state::<std::sync::Arc<playback::PlaybackManager>>()
                    .shutdown();
            }
        });
}
