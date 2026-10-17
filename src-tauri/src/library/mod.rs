mod artwork;
pub mod files;
mod metadata;
mod storage;

use artwork::ArtworkCache;
pub(crate) use metadata::MusicTrack;
use serde::Serialize;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
pub(crate) use storage::PlaybackPreferences;
use tauri::{
    Manager, State,
    ipc::{Channel, Response},
};
use tauri_plugin_dialog::DialogExt;

#[derive(Default)]
pub struct LibraryState {
    database: Mutex<Option<storage::Database>>,
    artwork: Mutex<ArtworkCache>,
    busy: AtomicBool,
    picking: AtomicBool,
    cancelled: AtomicBool,
    generation: AtomicU64,
}
struct BusyGuard(Arc<LibraryState>);
impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.busy.store(false, Ordering::SeqCst);
    }
}
struct PickerGuard(Arc<LibraryState>);
impl Drop for PickerGuard {
    fn drop(&mut self) {
        self.0.picking.store(false, Ordering::SeqCst);
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    discovered: usize,
    skipped: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    tracks: Vec<MusicTrack>,
    discovered: usize,
    skipped: usize,
    warnings: usize,
    recovered_tags: usize,
    cancelled: bool,
    limited: bool,
    scanned_paths: Vec<String>,
    folders: Vec<storage::SavedFolder>,
}

fn with_database<T>(
    app: &tauri::AppHandle,
    state: &LibraryState,
    action: impl FnOnce(&mut storage::Database) -> Result<T, String>,
) -> Result<T, String> {
    let mut database = state
        .database
        .lock()
        .map_err(|_| "Library database state is unavailable.")?;
    if database.is_none() {
        let directory = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Cannot locate application data directory: {e}"))?;
        *database = Some(storage::Database::open(&directory.join("library.sqlite3"))?);
    }
    action(
        database
            .as_mut()
            .ok_or("Library database initialization failed.")?,
    )
}

pub(crate) fn load_playback_preferences(
    app: &tauri::AppHandle,
    state: &LibraryState,
) -> Result<PlaybackPreferences, String> {
    with_database(app, state, |db| db.playback_preferences())
}

pub(crate) fn save_playback_preferences(
    state: &LibraryState,
    preferences: PlaybackPreferences,
) -> Result<(), String> {
    let mut database = state
        .database
        .try_lock()
        .map_err(|_| "The library is busy saving a scan. Adjust volume again to retry saving.")?;
    database
        .as_mut()
        .ok_or("The library database is unavailable.")?
        .save_playback_preferences(preferences)
}

pub(crate) fn playback_track(
    state: &LibraryState,
    id: &str,
) -> Result<(MusicTrack, std::fs::File), String> {
    // A scan commit must not stall the transport worker behind a long database lock.
    let database = state
        .database
        .try_lock()
        .map_err(|_| "The library is busy saving a scan. Try playing this track again shortly.")?;
    database
        .as_ref()
        .ok_or("The saved library is still loading. Try again when loading is complete.")?
        .playback_track(id)
}

pub(crate) fn playback_neighbor(
    state: &LibraryState,
    id: &str,
    previous: bool,
    liked: bool,
) -> Result<Option<String>, String> {
    let database = state
        .database
        .try_lock()
        .map_err(|_| "The library is busy saving a scan. Try track navigation again shortly.")?;
    let database = database
        .as_ref()
        .ok_or("The saved library is still loading.")?;
    if liked {
        database.liked_playback_neighbor(id, previous)
    } else {
        database.playback_neighbor(id, previous)
    }
}

#[tauri::command]
pub async fn load_library(
    app: tauri::AppHandle,
    state: State<'_, Arc<LibraryState>>,
) -> Result<storage::LibrarySnapshot, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        with_database(&app, &state, |database| database.snapshot())
    })
    .await
    .map_err(|e| format!("Cannot restore the saved library: {e}"))?
}

#[tauri::command]
pub async fn select_music_folder(
    app: tauri::AppHandle,
    state: State<'_, Arc<LibraryState>>,
) -> Result<Option<String>, String> {
    let state = Arc::clone(state.inner());
    if state.picking.swap(true, Ordering::SeqCst) {
        return Err("A folder picker is already open.".into());
    }
    let guard = PickerGuard(state.clone());
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let Some(folder) = app
            .dialog()
            .file()
            .set_title("Add Music Folder")
            .blocking_pick_folder()
        else {
            return Ok(None);
        };
        let path = folder.into_path().map_err(|e| e.to_string())?;
        let canonical = std::fs::canonicalize(&path)
            .map_err(|_| "The selected folder cannot be accessed.".to_string())?;
        if !canonical.is_dir() {
            return Err("Please select a directory.".into());
        }
        with_database(&app, &state, |database| database.save_folder(&canonical)).map(Some)
    })
    .await
    .map_err(|e| format!("Folder picker failed: {e}"))?
}

#[tauri::command]
pub async fn scan_music(
    app: tauri::AppHandle,
    roots: Vec<String>,
    on_progress: Channel<ScanProgress>,
    state: State<'_, Arc<LibraryState>>,
) -> Result<ScanResult, String> {
    let state = Arc::clone(state.inner());
    if state.busy.swap(true, Ordering::SeqCst) {
        return Err("A music scan is already running.".into());
    }
    state.cancelled.store(false, Ordering::SeqCst);
    let guard = BusyGuard(state.clone());
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let selected = with_database(&app, &state, |database| database.selected())?;
        let (roots, missing) = files::authorized_roots(&roots, &selected)?;
        lofty::config::apply_global_options(
            lofty::config::GlobalOptions::new()
                .allocation_limit(16 * 1024 * 1024)
                .preserve_format_specific_items(false),
        );
        let generation = state.generation.fetch_add(1, Ordering::Relaxed);
        let mut cache = ArtworkCache::default();
        let mut tracks = Vec::new();
        let mut recovered_tags = 0;
        let mut last_update = Instant::now();
        let stats = files::walk(
            &roots,
            &state.cancelled,
            |path| {
                // Parser failures (including malformed third-party tag panics) affect one file only.
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    metadata::read_track(path, &mut cache, generation)
                })) {
                    Ok(Ok((track, recovered))) => {
                        tracks.push(track);
                        recovered_tags += usize::from(recovered);
                        true
                    }
                    _ => false,
                }
            },
            |stats| {
                if last_update.elapsed() >= Duration::from_millis(200) {
                    let _ = on_progress.send(ScanProgress {
                        discovered: stats.discovered,
                        skipped: stats.skipped,
                    });
                    last_update = Instant::now();
                }
            },
        );
        let mut cancelled = stats.cancelled || state.cancelled.load(Ordering::SeqCst);
        let mut scanned_paths = Vec::new();
        let mut folders = Vec::new();
        if cancelled {
            tracks.clear();
        } else {
            let snapshot = with_database(&app, &state, |database| {
                database.save_artwork(&cache)?;
                if !database.save_scan(
                    &roots,
                    &tracks,
                    !stats.limited && stats.warnings == 0,
                    &state.cancelled,
                )? {
                    return Ok(None);
                }
                database.snapshot().map(Some)
            })?;
            if let Some(snapshot) = snapshot {
                tracks = snapshot.tracks;
                scanned_paths = snapshot.scanned_paths;
                folders = snapshot.folders;
                *state
                    .artwork
                    .lock()
                    .map_err(|_| "Artwork cache is unavailable.")? = cache;
            } else {
                cancelled = true;
                tracks.clear();
            }
        }
        Ok(ScanResult {
            tracks,
            discovered: stats.discovered,
            skipped: stats.skipped,
            warnings: stats.warnings + missing,
            recovered_tags,
            cancelled,
            limited: stats.limited,
            scanned_paths,
            folders,
        })
    })
    .await
    .map_err(|e| format!("Music scan failed: {e}"))?
}

#[tauri::command]
pub fn cancel_music_scan(state: State<'_, Arc<LibraryState>>) {
    if state.busy.load(Ordering::SeqCst) {
        state.cancelled.store(true, Ordering::SeqCst);
    }
}

#[tauri::command]
pub async fn get_music_artwork(
    app: tauri::AppHandle,
    id: String,
    state: State<'_, Arc<LibraryState>>,
) -> Result<Response, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let cached = state
            .artwork
            .lock()
            .map_err(|_| "Artwork cache is unavailable.")?
            .images
            .get(&id)
            .cloned();
        if let Some(bytes) = cached {
            return Ok(Response::new(bytes));
        }
        with_database(&app, &state, |database| database.artwork(&id)).map(Response::new)
    })
    .await
    .map_err(|e| format!("Cannot load artwork: {e}"))?
}

#[tauri::command]
pub async fn set_track_favorite(
    app: tauri::AppHandle,
    id: String,
    favorite: bool,
    state: State<'_, Arc<LibraryState>>,
) -> Result<bool, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        with_database(&app, &state, |database| {
            database.set_favorite(&id, favorite)
        })
    })
    .await
    .map_err(|e| format!("Cannot save favorite: {e}"))?
}
