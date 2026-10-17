use super::Database;
use crate::library::{
    artwork::ArtworkCache,
    files::tests::Fixture,
    metadata::{ArtworkRef, MusicTrack},
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

fn track(path: &Path, title: &str) -> MusicTrack {
    MusicTrack {
        id: path.to_string_lossy().into_owned(),
        path: path.to_string_lossy().into_owned(),
        title: title.into(),
        artist: "Artist".into(),
        album: "Album".into(),
        album_artist: Some("Ensemble".into()),
        duration_seconds: Some(123.5),
        track_number: Some(2),
        disc_number: Some(1),
        year: Some(2024),
        format: "FLAC".into(),
        artwork: None,
        available: true,
        favorite: false,
    }
}
fn setup() -> (Fixture, PathBuf, Database) {
    let f = Fixture::new();
    let location = f.0.join("app-data/library.sqlite3");
    let database = Database::open(&location).unwrap();
    (f, location, database)
}
fn save(db: &mut Database, roots: &[PathBuf], tracks: &[MusicTrack], complete: bool) {
    assert!(
        db.save_scan(roots, tracks, complete, &AtomicBool::new(false))
            .unwrap()
    );
}

#[test]
fn initializes_reopens_and_restores_folders_and_metadata() {
    let (f, location, mut db) = setup();
    let path = f.file("music/first.flac", b"fixture");
    let root = fs::canonicalize(path.parent().unwrap()).unwrap();
    db.save_folder(&root).unwrap();
    save(&mut db, &[root.clone()], &[track(&path, "First")], true);
    drop(db);
    let db = Database::open(&location).unwrap();
    let snapshot = db.snapshot().unwrap();
    assert_eq!(snapshot.folders.len(), 1);
    assert!(snapshot.folders[0].available);
    assert_eq!(snapshot.folders[0].path, root.to_string_lossy());
    assert_eq!(snapshot.tracks.len(), 1);
    let restored = &snapshot.tracks[0];
    assert_eq!(restored.title, "First");
    assert_eq!(restored.artist, "Artist");
    assert_eq!(restored.album, "Album");
    assert_eq!(restored.album_artist.as_deref(), Some("Ensemble"));
    assert_eq!(restored.duration_seconds, Some(123.5));
    assert_eq!(restored.track_number, Some(2));
    assert_eq!(restored.disc_number, Some(1));
    assert_eq!(restored.year, Some(2024));
    assert!(restored.available);
    assert_eq!(snapshot.scanned_paths.len(), 1);
}

#[test]
fn duplicate_folders_and_repeated_scans_update_instead_of_insert() {
    let (f, _, mut db) = setup();
    let path = f.file("music/first.flac", b"fixture");
    let root = fs::canonicalize(path.parent().unwrap()).unwrap();
    db.save_folder(&root).unwrap();
    db.save_folder(&root).unwrap();
    save(&mut db, &[root.clone()], &[track(&path, "Old")], true);
    save(&mut db, &[root.clone()], &[track(&path, "Updated")], true);
    let snapshot = db.snapshot().unwrap();
    assert_eq!(snapshot.folders.len(), 1);
    assert_eq!(snapshot.tracks.len(), 1);
    assert_eq!(snapshot.tracks[0].title, "Updated");
    let second = f.file("music/second.flac", b"fixture");
    save(&mut db, &[root], &[track(&second, "Updated")], true);
    assert_eq!(
        db.snapshot().unwrap().tracks.len(),
        2,
        "Song titles are not identities"
    );
}

#[test]
fn normalized_windows_paths_do_not_duplicate_tracks() {
    if !cfg!(windows) {
        return;
    }
    let (f, _, mut db) = setup();
    let path = f.file("Music/Case.flac", b"fixture");
    let root = fs::canonicalize(path.parent().unwrap()).unwrap();
    db.save_folder(&root).unwrap();
    save(&mut db, &[root.clone()], &[track(&path, "Case")], true);
    let alias = PathBuf::from(path.to_string_lossy().to_uppercase());
    // Canonical native scans return the original casing. Path keys also tolerate aliases.
    let canonical = fs::canonicalize(alias).unwrap();
    save(&mut db, &[root], &[track(&canonical, "New")], true);
    assert_eq!(db.snapshot().unwrap().tracks.len(), 1);
}

#[test]
fn overlapping_roots_and_unrelated_tracks_survive_reconciliation() {
    let (f, _, mut db) = setup();
    let shared = f.file("music/child/shared.flac", b"fixture");
    let other = f.file("other/other.flac", b"fixture");
    let parent = fs::canonicalize(f.0.join("music")).unwrap();
    let child = fs::canonicalize(f.0.join("music/child")).unwrap();
    let unrelated = fs::canonicalize(f.0.join("other")).unwrap();
    for root in [&parent, &unrelated] {
        db.save_folder(root).unwrap();
    }
    save(
        &mut db,
        &[parent.clone(), unrelated.clone()],
        &[track(&shared, "Shared"), track(&other, "Other")],
        true,
    );
    db.save_folder(&child).unwrap(); // Backfill an overlapping owner added after indexing.
    save(&mut db, &[child.clone()], &[track(&shared, "Shared")], true);
    assert_eq!(db.snapshot().unwrap().tracks.len(), 2);
    fs::remove_file(&shared).unwrap();
    save(&mut db, &[child], &[], true);
    let snapshot = db.snapshot().unwrap();
    assert!(
        !snapshot
            .tracks
            .iter()
            .find(|t| t.title == "Shared")
            .unwrap()
            .available
    );
    assert!(
        snapshot
            .tracks
            .iter()
            .find(|t| t.title == "Other")
            .unwrap()
            .available
    );
    assert_eq!(snapshot.tracks.len(), 2);
}

#[test]
fn missing_folders_restore_without_losing_saved_rows() {
    let (f, location, mut db) = setup();
    let path = f.file("music/first.flac", b"fixture");
    let root = fs::canonicalize(path.parent().unwrap()).unwrap();
    db.save_folder(&root).unwrap();
    save(&mut db, &[root.clone()], &[track(&path, "Saved")], true);
    fs::remove_file(&path).unwrap();
    fs::remove_dir(&root).unwrap();
    drop(db);
    let snapshot = Database::open(&location).unwrap().snapshot().unwrap();
    assert_eq!(snapshot.folders.len(), 1);
    assert!(!snapshot.folders[0].available);
    assert_eq!(snapshot.tracks.len(), 1);
    assert!(!snapshot.tracks[0].available);
}

#[test]
fn partial_scans_do_not_reconcile_or_mark_folders_complete() {
    let (f, _, mut db) = setup();
    let path = f.file("music/first.flac", b"fixture");
    let root = fs::canonicalize(path.parent().unwrap()).unwrap();
    db.save_folder(&root).unwrap();
    save(&mut db, &[root.clone()], &[track(&path, "Kept")], false);
    save(&mut db, &[root], &[], false);
    let snapshot = db.snapshot().unwrap();
    assert_eq!(snapshot.tracks.len(), 1);
    assert!(snapshot.tracks[0].available);
    assert!(snapshot.scanned_paths.is_empty());
}

#[test]
fn cancelled_scan_keeps_previous_metadata() {
    let (f, _, mut db) = setup();
    let path = f.file("music/first.flac", b"fixture");
    let root = fs::canonicalize(path.parent().unwrap()).unwrap();
    db.save_folder(&root).unwrap();
    save(&mut db, &[root.clone()], &[track(&path, "Kept")], true);
    assert!(
        !db.save_scan(
            &[root],
            &[track(&path, "Not committed")],
            true,
            &AtomicBool::new(true)
        )
        .unwrap()
    );
    assert_eq!(db.snapshot().unwrap().tracks[0].title, "Kept");
}

#[test]
fn artwork_references_reopen_without_database_blobs() {
    let (f, location, mut db) = setup();
    let path = f.file("music/first.flac", b"fixture");
    let root = fs::canonicalize(path.parent().unwrap()).unwrap();
    db.save_folder(&root).unwrap();
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    png.extend(32u32.to_be_bytes());
    png.extend(32u32.to_be_bytes());
    let mut cache = ArtworkCache::default();
    let (id, mime) = cache.insert(5, &png).unwrap();
    let mut item = track(&path, "Art");
    item.artwork = Some(ArtworkRef {
        id: id.clone(),
        mime,
    });
    db.save_artwork(&cache).unwrap();
    save(&mut db, &[root], &[item], true);
    drop(db);
    let db = Database::open(&location).unwrap();
    assert_eq!(db.artwork(&id).unwrap(), png);
    assert_eq!(
        db.snapshot().unwrap().tracks[0]
            .artwork
            .as_ref()
            .unwrap()
            .id,
        id
    );
    assert!(db.artwork("../../outside").is_err());
    assert!(db.artwork("0000000000000000").is_err());
}

#[test]
fn a_database_error_rolls_back_all_track_updates() {
    let (f, _, mut db) = setup();
    let path = f.file("music/first.flac", b"fixture");
    let rejected = f.file("music/rejected.flac", b"fixture");
    let root = fs::canonicalize(path.parent().unwrap()).unwrap();
    db.save_folder(&root).unwrap();
    save(&mut db, &[root.clone()], &[track(&path, "Original")], true);
    db.connection.execute_batch("CREATE TRIGGER fixture_failure BEFORE INSERT ON tracks WHEN NEW.title='Reject' BEGIN SELECT RAISE(ABORT,'fixture database failure'); END;").unwrap();
    assert!(
        db.save_scan(
            &[root],
            &[track(&path, "Should roll back"), track(&rejected, "Reject")],
            true,
            &AtomicBool::new(false)
        )
        .is_err()
    );
    let snapshot = db.snapshot().unwrap();
    assert_eq!(snapshot.tracks.len(), 1);
    assert_eq!(snapshot.tracks[0].title, "Original");
}

#[test]
fn a_newer_schema_is_rejected_without_resetting_data() {
    let (f, location, mut db) = setup();
    let path = f.file("music/first.flac", b"fixture");
    let root = fs::canonicalize(path.parent().unwrap()).unwrap();
    db.save_folder(&root).unwrap();
    save(&mut db, &[root], &[track(&path, "Preserved")], true);
    db.connection
        .pragma_update(None, "user_version", 99)
        .unwrap();
    drop(db);
    assert!(Database::open(&location).is_err());
    let connection = rusqlite::Connection::open(&location).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT title FROM tracks", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "Preserved"
    );
    assert_eq!(
        connection
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        99
    );
}

#[test]
fn an_unknown_existing_database_is_not_overwritten() {
    let f = Fixture::new();
    let location = f.0.join("unknown.sqlite3");
    let connection = rusqlite::Connection::open(&location).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE unrelated(value TEXT); INSERT INTO unrelated VALUES('Keep this');",
        )
        .unwrap();
    drop(connection);
    assert!(Database::open(&location).is_err());
    let connection = rusqlite::Connection::open(&location).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT value FROM unrelated", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "Keep this"
    );
}

#[test]
fn playback_lookup_requires_a_saved_available_file_in_authorized_roots() {
    let (f, _, mut db) = setup();
    let path = f.file("music/first.flac", b"fixture");
    let root = fs::canonicalize(path.parent().unwrap()).unwrap();
    db.save_folder(&root).unwrap();
    save(&mut db, &[root], &[track(&path, "Saved")], true);
    let id = db.snapshot().unwrap().tracks[0].id.clone();
    assert!(db.playback_track(&id).is_ok());
    assert!(db.playback_track("not-saved").is_err());
    fs::remove_file(&path).unwrap();
    assert!(db.playback_track(&id).is_err());
}

#[test]
fn playback_neighbors_follow_the_displayed_library_order_and_boundaries() {
    let (f, _, mut db) = setup();
    let paths = [
        f.file("music/c.flac", b"fixture"),
        f.file("music/a.flac", b"fixture"),
        f.file("music/b.flac", b"fixture"),
    ];
    let root = fs::canonicalize(paths[0].parent().unwrap()).unwrap();
    db.save_folder(&root).unwrap();
    let tracks: Vec<_> = paths.iter().map(|p| track(p, "Same title")).collect();
    save(&mut db, &[root], &tracks, true);
    let snapshot = db.snapshot().unwrap();
    let ids: Vec<_> = snapshot.tracks.iter().map(|t| t.id.as_str()).collect();
    assert!(snapshot.tracks[0].path.ends_with("a.flac"));
    assert_eq!(db.playback_neighbor(ids[0], true).unwrap(), None);
    assert_eq!(
        db.playback_neighbor(ids[0], false).unwrap().as_deref(),
        Some(ids[1])
    );
    assert_eq!(
        db.playback_neighbor(ids[1], true).unwrap().as_deref(),
        Some(ids[0])
    );
    assert_eq!(
        db.playback_neighbor(ids[1], false).unwrap().as_deref(),
        Some(ids[2])
    );
    assert_eq!(db.playback_neighbor(ids[2], false).unwrap(), None);
    assert!(db.playback_neighbor("not-saved", false).is_err());
}

#[test]
fn volume_preferences_default_save_reopen_and_reject_invalid_values() {
    use super::PlaybackPreferences;
    let (_f, location, mut db) = setup();
    assert_eq!(
        db.playback_preferences().unwrap(),
        PlaybackPreferences::default()
    );
    let saved = PlaybackPreferences {
        volume: 0.37,
        muted: true,
    };
    db.save_playback_preferences(saved).unwrap();
    drop(db);
    let mut db = Database::open(&location).unwrap();
    assert_eq!(db.playback_preferences().unwrap(), saved);
    for volume in [-1.0, 2.0, f64::NAN, f64::INFINITY] {
        assert!(
            db.save_playback_preferences(PlaybackPreferences {
                volume,
                muted: false
            })
            .is_err()
        );
        assert_eq!(db.playback_preferences().unwrap(), saved);
    }
    db.connection.execute_batch("PRAGMA query_only=ON").unwrap();
    assert!(
        db.save_playback_preferences(PlaybackPreferences::default())
            .is_err()
    );
    assert_eq!(db.playback_preferences().unwrap(), saved);
}

#[test]
fn version_one_migrates_preferences_without_losing_library_and_reopens_safely() {
    use super::PlaybackPreferences;
    let (f, location, mut db) = setup();
    let path = f.file("music/keep.flac", b"fixture");
    let root = fs::canonicalize(path.parent().unwrap()).unwrap();
    db.save_folder(&root).unwrap();
    save(&mut db, &[root], &[track(&path, "Preserved")], true);
    db.connection
        .execute_batch("DROP TABLE playlist_tracks; DROP TABLE playlists; ALTER TABLE tracks DROP COLUMN favorite; DROP TABLE playback_preferences; PRAGMA user_version=1;")
        .unwrap();
    drop(db);
    let mut db = Database::open(&location).unwrap();
    assert_eq!(db.snapshot().unwrap().tracks[0].title, "Preserved");
    assert_eq!(db.snapshot().unwrap().folders.len(), 1);
    assert_eq!(
        db.playback_preferences().unwrap(),
        PlaybackPreferences::default()
    );
    db.save_playback_preferences(PlaybackPreferences {
        volume: 0.0,
        muted: false,
    })
    .unwrap();
    drop(db);
    let db = Database::open(&location).unwrap();
    assert_eq!(db.playback_preferences().unwrap().volume, 0.0);
    assert_eq!(
        db.connection
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        4
    );
}

#[test]
fn failed_preferences_migration_does_not_advance_schema_or_replace_existing_data() {
    let (_f, location, db) = setup();
    // An unexpected pre-existing object must fail transactionally, not be overwritten.
    db.connection
        .execute_batch("PRAGMA user_version=1;")
        .unwrap();
    drop(db);
    assert!(Database::open(&location).is_err());
    let connection = rusqlite::Connection::open(location).unwrap();
    assert_eq!(
        connection
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row("SELECT volume FROM playback_preferences", [], |r| r
                .get::<_, f64>(0))
            .unwrap(),
        1.0
    );
}

#[test]
fn favorites_default_persist_rescan_and_remain_unique() {
    let (f, location, mut db) = setup();
    let first = f.file("music/a.flac", b"fixture");
    let second = f.file("music/b.flac", b"fixture");
    let root = fs::canonicalize(first.parent().unwrap()).unwrap();
    db.save_folder(&root).unwrap();
    let tracks = [track(&first, "First"), track(&second, "Second")];
    save(&mut db, &[root.clone()], &tracks, true);
    let snapshot = db.snapshot().unwrap();
    assert!(snapshot.tracks.iter().all(|t| !t.favorite));
    let id = snapshot.tracks[0].id.clone();
    db.set_favorite(&id, true).unwrap();
    db.set_favorite(&id, true).unwrap();
    drop(db);
    let mut db = Database::open(&location).unwrap();
    assert!(db.playback_track(&id).unwrap().0.favorite);
    save(&mut db, &[root], &tracks, true);
    let snapshot = db.snapshot().unwrap();
    assert_eq!(snapshot.tracks.len(), 2);
    let liked: Vec<_> = snapshot.tracks.iter().filter(|t| t.favorite).collect();
    assert_eq!(liked.len(), 1);
    assert_eq!(liked[0].id, id);
    db.set_favorite(&id, false).unwrap();
    db.set_favorite(&id, false).unwrap();
    assert!(db.set_favorite("unknown", true).is_err());
    drop(db);
    let snapshot = Database::open(&location).unwrap().snapshot().unwrap();
    assert_eq!(snapshot.tracks.len(), 2);
    assert!(snapshot.tracks.iter().all(|t| !t.favorite));
}

#[test]
fn version_two_migrates_favorites_without_losing_data() {
    let (f, location, mut db) = setup();
    let path = f.file("music/keep.flac", b"fixture");
    let root = fs::canonicalize(path.parent().unwrap()).unwrap();
    db.save_folder(&root).unwrap();
    save(&mut db, &[root], &[track(&path, "Preserved")], true);
    db.connection
        .execute_batch("DROP TABLE playlist_tracks; DROP TABLE playlists; ALTER TABLE tracks DROP COLUMN favorite; PRAGMA user_version=2;")
        .unwrap();
    drop(db);
    let db = Database::open(&location).unwrap();
    let snapshot = db.snapshot().unwrap();
    assert_eq!(snapshot.folders.len(), 1);
    assert_eq!(snapshot.tracks.len(), 1);
    assert_eq!(snapshot.tracks[0].title, "Preserved");
    assert!(!snapshot.tracks[0].favorite);
    assert_eq!(
        db.playback_preferences().unwrap(),
        super::PlaybackPreferences::default()
    );
    assert_eq!(
        db.connection
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        4
    );
}

#[test]
fn failed_favorite_writes_preserve_saved_state() {
    let (f, _, mut db) = setup();
    let path = f.file("music/keep.flac", b"fixture");
    let root = fs::canonicalize(path.parent().unwrap()).unwrap();
    db.save_folder(&root).unwrap();
    save(&mut db, &[root], &[track(&path, "Preserved")], true);
    let id = db.snapshot().unwrap().tracks[0].id.clone();
    db.connection.execute_batch("PRAGMA query_only=ON").unwrap();
    assert!(db.set_favorite(&id, true).is_err());
    assert!(!db.snapshot().unwrap().tracks[0].favorite);
}

#[test]
fn liked_navigation_uses_live_membership_and_removed_current_track_as_anchor() {
    let (f, _, mut db) = setup();
    let paths = [
        f.file("music/a.flac", b"fixture"),
        f.file("music/b.flac", b"fixture"),
        f.file("music/c.flac", b"fixture"),
    ];
    let root = fs::canonicalize(paths[0].parent().unwrap()).unwrap();
    db.save_folder(&root).unwrap();
    let tracks: Vec<_> = paths.iter().map(|path| track(path, "Song")).collect();
    save(&mut db, &[root], &tracks, true);
    let ids: Vec<_> = db
        .snapshot()
        .unwrap()
        .tracks
        .into_iter()
        .map(|t| t.id)
        .collect();
    db.set_favorite(&ids[0], true).unwrap();
    db.set_favorite(&ids[2], true).unwrap();
    assert_eq!(
        db.liked_playback_neighbor(&ids[0], false).unwrap(),
        Some(ids[2].clone())
    );
    assert_eq!(
        db.playback_neighbor(&ids[0], false).unwrap(),
        Some(ids[1].clone())
    );
    assert_eq!(
        db.liked_playback_neighbor(&ids[2], true).unwrap(),
        Some(ids[0].clone())
    );
    assert_eq!(db.liked_playback_neighbor(&ids[0], true).unwrap(), None);
    assert_eq!(db.liked_playback_neighbor(&ids[2], false).unwrap(), None);
    db.set_favorite(&ids[0], false).unwrap();
    assert_eq!(
        db.liked_playback_neighbor(&ids[0], false).unwrap(),
        Some(ids[2].clone())
    );
    db.set_favorite(&ids[1], true).unwrap();
    assert_eq!(
        db.liked_playback_neighbor(&ids[0], false).unwrap(),
        Some(ids[1].clone())
    );
    db.set_favorite(&ids[1], false).unwrap();
    db.set_favorite(&ids[2], false).unwrap();
    assert_eq!(db.liked_playback_neighbor(&ids[0], false).unwrap(), None);
}

#[test]
fn playlists_validate_names_and_support_rename_delete_and_restart() {
    use super::PlaylistAction::*;
    let (_f, location, mut db) = setup();
    assert!(db.playlists().unwrap().is_empty());
    for name in ["", "  ", "\t\n", "\u{2003}"] {
        assert!(db.playlist_control(Create { name: name.into() }).is_err());
    }
    let created = db
        .playlist_control(Create {
            name: "  After Midnight  ".into(),
        })
        .unwrap();
    let id = created[0].id.clone();
    assert_eq!(created[0].name, "After Midnight");
    assert_eq!(id.len(), 32);
    assert!(created[0].created_at > 0);
    assert!(
        db.playlist_control(Rename {
            playlist_id: id.clone(),
            name: " ".into()
        })
        .is_err()
    );
    db.playlist_control(Rename {
        playlist_id: id.clone(),
        name: "  Night Drive  ".into(),
    })
    .unwrap();
    drop(db);
    let mut db = Database::open(&location).unwrap();
    let restored = db.playlists().unwrap();
    assert_eq!(restored[0].id, id);
    assert_eq!(restored[0].name, "Night Drive");
    db.playlist_control(Delete {
        playlist_id: id.clone(),
    })
    .unwrap();
    assert!(db.playlists().unwrap().is_empty());
    assert!(
        db.playlist_control(Rename {
            playlist_id: id,
            name: "Gone".into()
        })
        .is_err()
    );
}

#[test]
fn playlist_membership_preserves_add_order_uniqueness_and_library_favorites() {
    use super::PlaylistAction::*;
    let (f, location, mut db) = setup();
    let paths = [
        f.file("music/a.flac", b"fixture"),
        f.file("music/b.flac", b"fixture"),
        f.file("music/c.flac", b"fixture"),
    ];
    let root = fs::canonicalize(paths[0].parent().unwrap()).unwrap();
    db.save_folder(&root).unwrap();
    save(
        &mut db,
        &[root],
        &paths.iter().map(|p| track(p, "Song")).collect::<Vec<_>>(),
        true,
    );
    let ids: Vec<_> = db
        .snapshot()
        .unwrap()
        .tracks
        .into_iter()
        .map(|t| t.id)
        .collect();
    db.set_favorite(&ids[0], true).unwrap();
    let id = db
        .playlist_control(Create {
            name: "Ordered".into(),
        })
        .unwrap()[0]
        .id
        .clone();
    for i in [2, 0, 1, 2] {
        db.playlist_control(Add {
            playlist_id: id.clone(),
            track_id: ids[i].clone(),
        })
        .unwrap();
    }
    assert_eq!(
        db.playlists().unwrap()[0].track_ids,
        vec![ids[2].clone(), ids[0].clone(), ids[1].clone()]
    );
    assert!(
        db.playlist_control(Add {
            playlist_id: id.clone(),
            track_id: "not-saved".into()
        })
        .is_err()
    );
    assert!(
        db.playlist_control(Add {
            playlist_id: "not-saved".into(),
            track_id: ids[0].clone()
        })
        .is_err()
    );
    drop(db);
    let mut db = Database::open(&location).unwrap();
    assert_eq!(
        db.playlists().unwrap()[0].track_ids,
        vec![ids[2].clone(), ids[0].clone(), ids[1].clone()]
    );
    assert!(db.playlist_contains(&id, &ids[0]).unwrap());
    db.playlist_control(Remove {
        playlist_id: id.clone(),
        track_id: ids[0].clone(),
    })
    .unwrap();
    db.playlist_control(Remove {
        playlist_id: id.clone(),
        track_id: ids[0].clone(),
    })
    .unwrap();
    assert_eq!(
        db.playlists().unwrap()[0].track_ids,
        vec![ids[2].clone(), ids[1].clone()]
    );
    assert!(!db.playlist_contains(&id, &ids[0]).unwrap());
    drop(db);
    let mut db = Database::open(&location).unwrap();
    assert_eq!(
        db.playlists().unwrap()[0].track_ids,
        vec![ids[2].clone(), ids[1].clone()]
    );
    assert_eq!(
        db.playlist_playback_neighbor(&id, &ids[0], false).unwrap(),
        Some(ids[1].clone())
    );
    assert_eq!(
        db.playlist_playback_neighbor(&id, &ids[0], true).unwrap(),
        Some(ids[2].clone())
    );
    assert_eq!(
        db.playlist_playback_neighbor(&id, &ids[2], true).unwrap(),
        None
    );
    assert_eq!(
        db.playlist_playback_neighbor(&id, &ids[1], false).unwrap(),
        None
    );
    db.playlist_control(Add {
        playlist_id: id.clone(),
        track_id: ids[0].clone(),
    })
    .unwrap();
    assert_eq!(
        db.playlists().unwrap()[0].track_ids,
        vec![ids[2].clone(), ids[1].clone(), ids[0].clone()]
    );
    fs::remove_file(&paths[1]).unwrap();
    assert_eq!(db.playlists().unwrap()[0].track_ids.len(), 3);
    assert!(!db.snapshot().unwrap().tracks[1].available);
    db.playlist_control(Delete {
        playlist_id: id.clone(),
    })
    .unwrap();
    let snapshot = db.snapshot().unwrap();
    assert_eq!(snapshot.tracks.len(), 3);
    assert!(snapshot.tracks[0].favorite);
    assert!(paths[0].exists());
    assert_eq!(
        db.playlist_playback_neighbor(&id, &ids[0], false).unwrap(),
        None
    );
    assert_eq!(
        db.connection
            .query_row("SELECT count(*) FROM playlist_tracks", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn version_three_migrates_playlists_preserving_existing_data() {
    let (f, location, mut db) = setup();
    let path = f.file("music/keep.flac", b"fixture");
    let root = fs::canonicalize(path.parent().unwrap()).unwrap();
    db.save_folder(&root).unwrap();
    save(&mut db, &[root], &[track(&path, "Preserved")], true);
    let id = db.snapshot().unwrap().tracks[0].id.clone();
    db.set_favorite(&id, true).unwrap();
    db.connection
        .execute_batch("DROP TABLE playlist_tracks; DROP TABLE playlists; PRAGMA user_version=3;")
        .unwrap();
    drop(db);
    let db = Database::open(&location).unwrap();
    let snapshot = db.snapshot().unwrap();
    assert_eq!(snapshot.tracks.len(), 1);
    assert_eq!(snapshot.folders.len(), 1);
    assert_eq!(snapshot.tracks[0].title, "Preserved");
    assert!(snapshot.tracks[0].favorite);
    assert!(db.playlists().unwrap().is_empty());
    assert_eq!(
        db.connection
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        4
    );
}

#[test]
fn failed_playlist_migration_is_transactional() {
    let (_f, location, db) = setup();
    db.connection.execute_batch("DROP TABLE playlist_tracks; DROP TABLE playlists; CREATE TABLE playlist_tracks(keep TEXT); INSERT INTO playlist_tracks VALUES('preserved'); PRAGMA user_version=3;").unwrap();
    drop(db);
    assert!(Database::open(&location).is_err());
    let connection = rusqlite::Connection::open(location).unwrap();
    assert_eq!(
        connection
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        connection
            .query_row("SELECT keep FROM playlist_tracks", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "preserved"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='playlists'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[test]
fn playlist_write_failures_preserve_confirmed_metadata() {
    use super::PlaylistAction::*;
    let (_f, _, mut db) = setup();
    let id = db
        .playlist_control(Create {
            name: "Saved".into(),
        })
        .unwrap()[0]
        .id
        .clone();
    db.connection
        .execute_batch("PRAGMA query_only=ON;")
        .unwrap();
    assert!(
        db.playlist_control(Rename {
            playlist_id: id.clone(),
            name: "Rejected".into()
        })
        .is_err()
    );
    assert!(db.playlist_control(Delete { playlist_id: id }).is_err());
    assert_eq!(db.playlists().unwrap()[0].name, "Saved");
    let action: super::PlaylistAction =
        serde_json::from_str(r#"{"type":"add","playlistId":"playlist","trackId":"track"}"#)
            .unwrap();
    assert!(
        matches!(action, Add {playlist_id,track_id} if playlist_id=="playlist" && track_id=="track")
    );
}
