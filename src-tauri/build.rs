fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "greet",
            "load_library",
            "set_track_favorite",
            "select_music_folder",
            "scan_music",
            "cancel_music_scan",
            "get_music_artwork",
            "playback_control",
        ]),
    ))
    .expect("failed to build Tauri application")
}
