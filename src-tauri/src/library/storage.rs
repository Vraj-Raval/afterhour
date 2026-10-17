use super::{
    artwork::{ArtworkCache, MAX_IMAGE_BYTES, raster_mime},
    files,
    metadata::{ArtworkRef, MusicTrack},
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

const SCHEMA_VERSION: i64 = 3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaybackPreferences {
    pub volume: f64,
    pub muted: bool,
}
impl Default for PlaybackPreferences {
    fn default() -> Self {
        Self {
            volume: 1.0,
            muted: false,
        }
    }
}

pub struct Database {
    connection: Connection,
    artwork_dir: PathBuf,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedFolder {
    pub path: String,
    pub key: String,
    pub name: String,
    pub available: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySnapshot {
    pub folders: Vec<SavedFolder>,
    pub tracks: Vec<MusicTrack>,
    pub scanned_paths: Vec<String>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, String> {
        let parent = path.parent().ok_or("Invalid library database location.")?;
        fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create library data directory: {e}"))?;
        let mut connection = Connection::open(path).map_err(db_error)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(db_error)?;
        connection
            .execute_batch("PRAGMA foreign_keys=ON;")
            .map_err(db_error)?;
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(db_error)?;
        if version > SCHEMA_VERSION {
            return Err("This library was created by a newer Afterhours version. The database has not been changed.".into());
        }
        if version == 0 {
            let tables: i64 = connection.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'", [], |r| r.get(0)).map_err(db_error)?;
            if tables != 0 {
                return Err("Unrecognized library database schema. The existing database has not been changed.".into());
            }
            let transaction = connection.transaction().map_err(db_error)?;
            transaction
                .execute_batch(
                    "
                CREATE TABLE folders (
                    id INTEGER PRIMARY KEY,
                    canonical_path TEXT NOT NULL,
                    path_key TEXT NOT NULL UNIQUE,
                    last_scanned_at INTEGER
                );
                CREATE TABLE tracks (
                    path_key TEXT PRIMARY KEY,
                    path TEXT NOT NULL,
                    title TEXT NOT NULL, artist TEXT NOT NULL, album TEXT NOT NULL,
                    album_artist TEXT, duration_seconds REAL, track_number INTEGER,
                    disc_number INTEGER, year INTEGER, format TEXT NOT NULL,
                    artwork_id TEXT, artwork_mime TEXT,
                    available INTEGER NOT NULL DEFAULT 1 CHECK(available IN (0,1)),
                    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
                );
                CREATE INDEX tracks_artist_album ON tracks(artist, album);
                CREATE INDEX tracks_available ON tracks(available);
                CREATE TABLE folder_tracks (
                    folder_id INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
                    track_key TEXT NOT NULL REFERENCES tracks(path_key) ON DELETE CASCADE,
                    PRIMARY KEY(folder_id, track_key)
                );
                CREATE INDEX folder_tracks_track ON folder_tracks(track_key);
                PRAGMA user_version=1;
            ",
                )
                .map_err(db_error)?;
            transaction.commit().map_err(db_error)?;
        }
        if version < 2 {
            let transaction = connection.transaction().map_err(db_error)?;
            transaction
                .execute_batch(
                    "CREATE TABLE playback_preferences (
                id INTEGER PRIMARY KEY CHECK(id=1),
                volume REAL NOT NULL CHECK(volume >= 0 AND volume <= 1),
                muted INTEGER NOT NULL CHECK(muted IN (0,1))
            );
            INSERT INTO playback_preferences(id,volume,muted) VALUES(1,1.0,0);
            PRAGMA user_version=2;",
                )
                .map_err(db_error)?;
            transaction.commit().map_err(db_error)?;
        }
        if version < 3 {
            let transaction = connection.transaction().map_err(db_error)?;
            transaction.execute_batch("ALTER TABLE tracks ADD COLUMN favorite INTEGER NOT NULL DEFAULT 0 CHECK(favorite IN (0,1)); PRAGMA user_version=3;").map_err(db_error)?;
            transaction.commit().map_err(db_error)?;
        }
        // Do not recreate or replace existing databases. SQLite manages its own journal.
        connection
            .execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(db_error)?;
        Ok(Self {
            connection,
            artwork_dir: parent.join("artwork"),
        })
    }

    /// Assign an explicit value: retries never invert state or insert records.
    pub fn set_favorite(&mut self, id: &str, favorite: bool) -> Result<bool, String> {
        let changed = self
            .connection
            .execute(
                "UPDATE tracks SET favorite=?2 WHERE path_key=?1",
                params![id, favorite],
            )
            .map_err(db_error)?;
        if changed == 0 {
            return Err(
                "This track is no longer in your saved library. Reload your library and try again."
                    .into(),
            );
        }
        Ok(favorite)
    }

    pub fn playback_preferences(&self) -> Result<PlaybackPreferences, String> {
        self.connection
            .query_row(
                "SELECT volume,muted FROM playback_preferences WHERE id=1",
                [],
                |r| {
                    Ok(PlaybackPreferences {
                        volume: r.get(0)?,
                        muted: r.get(1)?,
                    })
                },
            )
            .map_err(db_error)
    }

    pub fn save_playback_preferences(
        &mut self,
        preferences: PlaybackPreferences,
    ) -> Result<(), String> {
        if !preferences.volume.is_finite() || !(0.0..=1.0).contains(&preferences.volume) {
            return Err("Invalid preferred volume.".into());
        }
        let transaction = self.connection.transaction().map_err(db_error)?;
        transaction.execute("INSERT INTO playback_preferences(id,volume,muted) VALUES(1,?1,?2) ON CONFLICT(id) DO UPDATE SET volume=excluded.volume,muted=excluded.muted", params![preferences.volume, preferences.muted]).map_err(db_error)?;
        transaction.commit().map_err(db_error)
    }

    pub fn selected(&self) -> Result<Vec<(String, PathBuf)>, String> {
        let mut statement = self
            .connection
            .prepare("SELECT canonical_path FROM folders ORDER BY id")
            .map_err(db_error)?;
        let selected = statement
            .query_map([], |row| {
                let path: String = row.get(0)?;
                Ok((path.clone(), PathBuf::from(path)))
            })
            .map_err(db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_error)?;
        Ok(selected)
    }

    /// Same ordering as snapshot(): search and pagination do not create a playback queue.
    pub fn playback_neighbor(&self, id: &str, previous: bool) -> Result<Option<String>, String> {
        self.connection.query_row(
            "WITH ordered AS (SELECT path_key, LAG(path_key) OVER (ORDER BY path,path_key) AS preceding, LEAD(path_key) OVER (ORDER BY path,path_key) AS following FROM tracks) SELECT CASE WHEN ?2 THEN preceding ELSE following END FROM ordered WHERE path_key=?1",
            params![id, previous], |row| row.get(0),
        ).optional().map_err(db_error)?.ok_or_else(|| "The current track is no longer in your saved library. Select another track.".into())
    }

    /// Resolve against live favorites, keeping a removed current track as an ordering anchor.
    pub fn liked_playback_neighbor(
        &self,
        id: &str,
        previous: bool,
    ) -> Result<Option<String>, String> {
        let path: String = self
            .connection
            .query_row("SELECT path FROM tracks WHERE path_key=?1", [id], |row| {
                row.get(0)
            })
            .optional()
            .map_err(db_error)?
            .ok_or("The current track is no longer in your saved library.")?;
        let sql = if previous {
            "SELECT path_key FROM tracks WHERE favorite=1 AND (path,path_key)<(?1,?2) ORDER BY path DESC,path_key DESC LIMIT 1"
        } else {
            "SELECT path_key FROM tracks WHERE favorite=1 AND (path,path_key)>(?1,?2) ORDER BY path,path_key LIMIT 1"
        };
        self.connection
            .query_row(sql, params![path, id], |row| row.get(0))
            .optional()
            .map_err(db_error)
    }

    /// Playback accepts a saved ID, never a frontend-provided filesystem path.
    pub fn playback_track(&self, id: &str) -> Result<(MusicTrack, fs::File), String> {
        let selected = self.selected()?;
        let track = self.connection.query_row(
            "SELECT path_key,path,title,artist,album,album_artist,duration_seconds,track_number,disc_number,year,format,artwork_id,artwork_mime,available,favorite FROM tracks WHERE path_key=?1", [id], |row| {
                let artwork_id: Option<String> = row.get(11)?;
                let mime: Option<String> = row.get(12)?;
                Ok(MusicTrack {
                    id: row.get(0)?, path: row.get(1)?, title: row.get(2)?, artist: row.get(3)?, album: row.get(4)?, album_artist: row.get(5)?,
                    duration_seconds: row.get(6)?, track_number: row.get(7)?, disc_number: row.get(8)?, year: row.get(9)?, format: row.get(10)?,
                    artwork: artwork_id.zip(mime).map(|(id,mime)| ArtworkRef { id, mime }), available: row.get(13)?, favorite: row.get(14)?,
                })
            }).optional().map_err(db_error)?.ok_or("This track is not in your saved library. Scan the music folder again.")?;
        let path = Path::new(&track.path);
        if !track.available || !file_available(path, &selected) {
            return Err("This audio file is missing, inaccessible, or outside its saved music folder. Reconnect the folder and scan again.".into());
        }
        let file = fs::File::open(path).map_err(|e| format!("Cannot open this audio file: {e}"))?;
        Ok((track, file))
    }

    pub fn save_folder(&mut self, canonical: &Path) -> Result<String, String> {
        let path = canonical.to_string_lossy().into_owned();
        let key = files::path_key(canonical);
        let transaction = self.connection.transaction().map_err(db_error)?;
        let count: i64 = transaction
            .query_row("SELECT count(*) FROM folders", [], |r| r.get(0))
            .map_err(db_error)?;
        let exists: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM folders WHERE path_key=?1)",
                [&key],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        if count >= 128 && !exists {
            return Err("The library supports up to 128 selected folders.".into());
        }
        transaction.execute("INSERT INTO folders(canonical_path,path_key) VALUES(?1,?2) ON CONFLICT(path_key) DO NOTHING", params![path, key]).map_err(db_error)?;
        // Adding an overlapping root also associates already-indexed descendants.
        let mut statement = transaction
            .prepare("SELECT path_key,path FROM tracks")
            .map_err(db_error)?;
        let indexed = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_error)?;
        drop(statement);
        for (track_key, track_path) in indexed {
            if files::within_root(Path::new(&track_path), canonical) {
                transaction.execute("INSERT OR IGNORE INTO folder_tracks(folder_id,track_key) SELECT id,?1 FROM folders WHERE path_key=?2", params![track_key, key]).map_err(db_error)?;
            }
        }
        let saved = transaction
            .query_row(
                "SELECT canonical_path FROM folders WHERE path_key=?1",
                [&key],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        transaction.commit().map_err(db_error)?;
        Ok(saved)
    }

    pub fn snapshot(&self) -> Result<LibrarySnapshot, String> {
        let selected = self.selected()?;
        let folders = selected
            .iter()
            .map(|(path, canonical)| {
                let name = canonical
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or(path)
                    .to_owned();
                let available = fs::canonicalize(path).is_ok_and(|current| {
                    files::path_key(&current) == files::path_key(canonical) && current.is_dir()
                }) && fs::read_dir(path).is_ok();
                SavedFolder {
                    path: path.clone(),
                    key: files::path_key(canonical),
                    name,
                    available,
                }
            })
            .collect();
        let mut statement = self.connection.prepare("SELECT path_key,path,title,artist,album,album_artist,duration_seconds,track_number,disc_number,year,format,artwork_id,artwork_mime,available,favorite FROM tracks ORDER BY path,path_key").map_err(db_error)?;
        let tracks = statement
            .query_map([], |row| {
                let path: String = row.get(1)?;
                let cached_available: bool = row.get(13)?;
                let available = cached_available && file_available(Path::new(&path), &selected);
                let artwork_id: Option<String> = row.get(11)?;
                let mime: Option<String> = row.get(12)?;
                Ok(MusicTrack {
                    id: row.get(0)?,
                    path,
                    title: row.get(2)?,
                    artist: row.get(3)?,
                    album: row.get(4)?,
                    album_artist: row.get(5)?,
                    duration_seconds: row.get(6)?,
                    track_number: row.get(7)?,
                    disc_number: row.get(8)?,
                    year: row.get(9)?,
                    format: row.get(10)?,
                    artwork: artwork_id
                        .zip(mime)
                        .map(|(id, mime)| ArtworkRef { id, mime }),
                    available,
                    favorite: row.get(14)?,
                })
            })
            .map_err(db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_error)?;
        let mut statement = self
            .connection
            .prepare(
                "SELECT canonical_path FROM folders WHERE last_scanned_at IS NOT NULL ORDER BY id",
            )
            .map_err(db_error)?;
        let scanned_paths = statement
            .query_map([], |r| r.get(0))
            .map_err(db_error)?
            .collect::<Result<Vec<String>, _>>()
            .map_err(db_error)?;
        Ok(LibrarySnapshot {
            folders,
            tracks,
            scanned_paths,
        })
    }

    /// Upserts are atomic. Reconciliation only follows complete, error-free traversal.
    /// Existing rows are never deleted; only proven missing files become unavailable.
    pub fn save_scan(
        &mut self,
        roots: &[PathBuf],
        tracks: &[MusicTrack],
        complete: bool,
        cancel: &AtomicBool,
    ) -> Result<bool, String> {
        let selected = self.selected()?;
        if roots.iter().any(|root| {
            !selected
                .iter()
                .any(|(_, saved)| files::path_key(saved) == files::path_key(root))
        }) {
            return Err("A scan root is not a saved music folder.".into());
        }
        let transaction = self.connection.transaction().map_err(db_error)?;
        for track in tracks {
            if cancel.load(Ordering::SeqCst) {
                return Ok(false);
            }
            let path = Path::new(&track.path);
            if !roots.iter().any(|root| files::within_root(path, root)) {
                return Err("Scanner returned a file outside its authorized roots.".into());
            }
            let key = files::path_key(path);
            transaction.execute("INSERT INTO tracks(path_key,path,title,artist,album,album_artist,duration_seconds,track_number,disc_number,year,format,artwork_id,artwork_mime,available)
                VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,1)
                ON CONFLICT(path_key) DO UPDATE SET path=excluded.path,title=excluded.title,artist=excluded.artist,album=excluded.album,album_artist=excluded.album_artist,
                duration_seconds=excluded.duration_seconds,track_number=excluded.track_number,disc_number=excluded.disc_number,year=excluded.year,format=excluded.format,
                artwork_id=excluded.artwork_id,artwork_mime=excluded.artwork_mime,available=1,updated_at=unixepoch()",
                params![key,track.path,track.title,track.artist,track.album,track.album_artist,track.duration_seconds,track.track_number,track.disc_number,track.year,track.format,track.artwork.as_ref().map(|a| &a.id),track.artwork.as_ref().map(|a| &a.mime)]).map_err(db_error)?;
            for (folder, canonical) in &selected {
                if files::within_root(path, canonical) {
                    transaction.execute("INSERT OR IGNORE INTO folder_tracks(folder_id,track_key) SELECT id,?1 FROM folders WHERE path_key=?2", params![key, files::path_key(Path::new(folder))]).map_err(db_error)?;
                }
            }
        }
        if complete {
            for root in roots {
                if cancel.load(Ordering::SeqCst) {
                    return Ok(false);
                }
                let root_key = files::path_key(root);
                let exists: bool = transaction
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM folders WHERE path_key=?1)",
                        [&root_key],
                        |r| r.get(0),
                    )
                    .map_err(db_error)?;
                if !exists {
                    return Err("A scan root is not a saved music folder.".into());
                }
                let mut statement = transaction.prepare("SELECT t.path_key,t.path FROM tracks t JOIN folder_tracks ft ON ft.track_key=t.path_key JOIN folders f ON f.id=ft.folder_id WHERE f.path_key=?1").map_err(db_error)?;
                let candidates = statement
                    .query_map([&root_key], |r| {
                        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                    })
                    .map_err(db_error)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(db_error)?;
                drop(statement);
                for (key, path) in candidates {
                    if cancel.load(Ordering::SeqCst) {
                        return Ok(false);
                    }
                    if files::within_root(Path::new(&path), root)
                        && fs::symlink_metadata(&path)
                            .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
                    {
                        transaction
                            .execute("UPDATE tracks SET available=0 WHERE path_key=?1", [&key])
                            .map_err(db_error)?;
                    }
                }
                transaction
                    .execute(
                        "UPDATE folders SET last_scanned_at=unixepoch() WHERE path_key=?1",
                        [&root_key],
                    )
                    .map_err(db_error)?;
            }
        }
        if cancel.load(Ordering::SeqCst) {
            return Ok(false);
        }
        transaction.commit().map_err(db_error)?;
        Ok(true)
    }

    pub fn save_artwork(&self, cache: &ArtworkCache) -> Result<(), String> {
        fs::create_dir_all(&self.artwork_dir)
            .map_err(|e| format!("Cannot create artwork cache: {e}"))?;
        for (id, bytes) in &cache.images {
            if !valid_artwork_id(id) || raster_mime(bytes).is_none() {
                continue;
            }
            let path = self.artwork_dir.join(id);
            match fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&path)
            {
                Ok(mut file) => {
                    file.write_all(bytes)
                        .map_err(|e| format!("Cannot save artwork cache: {e}"))?;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    if read_artwork_file(&path).ok().as_deref() != Some(bytes.as_slice()) {
                        return Err(
                            "Artwork cache content mismatch. Existing cache was not overwritten."
                                .into(),
                        );
                    }
                }
                Err(e) => return Err(format!("Cannot save artwork cache: {e}")),
            }
        }
        Ok(())
    }

    pub fn artwork(&self, id: &str) -> Result<Vec<u8>, String> {
        if !valid_artwork_id(id) {
            return Err("Invalid artwork identifier.".into());
        }
        let referenced: Option<String> = self
            .connection
            .query_row(
                "SELECT artwork_mime FROM tracks WHERE artwork_id=?1 LIMIT 1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_error)?;
        let bytes = read_artwork_file(&self.artwork_dir.join(id))?;
        if raster_mime(&bytes) != referenced.as_deref() {
            return Err("Artwork is unavailable.".into());
        }
        Ok(bytes)
    }
}

fn db_error(error: rusqlite::Error) -> String {
    format!("Library database error: {error}")
}
fn valid_artwork_id(id: &str) -> bool {
    id.len() == 16 && id.bytes().all(|b| b.is_ascii_hexdigit())
}
fn read_artwork_file(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if files::is_link(&metadata) || metadata.len() > MAX_IMAGE_BYTES as u64 {
        return Err("Invalid artwork cache file.".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_IMAGE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err("Artwork exceeds cache limit.".into());
    }
    Ok(bytes)
}
fn file_available(path: &Path, roots: &[(String, PathBuf)]) -> bool {
    roots.iter().any(|(_, root)| files::within_root(path, root))
        && fs::canonicalize(path).is_ok_and(|current| {
            files::path_key(&current) == files::path_key(path)
                && roots
                    .iter()
                    .any(|(_, root)| files::within_root(&current, root))
        })
        && fs::symlink_metadata(path).is_ok_and(|m| m.is_file() && !files::is_link(&m))
        && fs::File::open(path).is_ok()
}

#[cfg(test)]
#[path = "storage_tests.rs"]
mod tests;
