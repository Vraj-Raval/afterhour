# Persistent local library

The native layer uses rusqlite with bundled SQLite. The database is
`app.path().app_data_dir()/library.sqlite3`; the directory comes from Tauri's
application-specific path API, never from a frontend parameter. On Windows with
the current identifier this is normally
`%APPDATA%/com.raval.afterhours/library.sqlite3`.

## Schema version 1

- `folders`: integer ID, canonical path, unique normalized path key, last complete
  scan timestamp.
- `tracks`: unique normalized path key, original canonical path, all scanner
  metadata, artwork ID/MIME, availability flag, update timestamp.
- `folder_tracks`: many-to-many ownership for overlapping saved roots, with
  foreign keys and a composite primary key.
- `PRAGMA user_version`: migration version. Artist/album, availability, and
  ownership lookup indexes support library queries.

Initialization and schema migration are transactional. Unknown pre-existing
schemas and newer versions are rejected without resetting or replacing their
data. Foreign keys, a busy timeout, and WAL journaling are enabled. All dynamic
queries use bound parameters. Only domain commands are exposed to the frontend;
there is no SQL execution command or frontend filesystem/database permission.

## Startup and folder selection

`load_library` initializes/opens the database on a blocking worker, then returns
folders, saved metadata, and completed-scan roots. File availability is checked
without re-extracting audio metadata or traversing directories. Missing,
inaccessible, or redirected folders/files are displayed as unavailable while
their rows remain saved. Startup errors show a retry action, not a destructive
database reset or a misleading empty library.

The native folder picker saves the canonical path before reporting success.
Repeated selections use a unique normalized key. Saved roots remain the native
authorization boundary for scans after an application restart.

## Scan transactions and unavailable files

An uncancelled scan upserts metadata and folder ownership in one transaction.
Identical song titles never identify files. The returned library includes all
saved tracks, including unrelated folders and unavailable entries.

Only complete traversals without directory-access warnings reconcile missing
files. A file must belong to the scanned root and return a definite `NotFound`
filesystem error before its availability flag is cleared; permission errors
are not treated as deletion. Missing roots are not reconciled. Overlapping roots
share a single track, and newly selected overlapping roots also associate
already-indexed descendants. No track row is deleted by scanning.

Cancelled scans and transaction failures roll back all metadata changes. Partial
or limited scans may add/update discovered metadata but cannot mark unseen files
missing or record a complete scan. Reappearing files become available after a
successful rescan. Startup also reflects current access failures without
discarding metadata.

## Artwork

SQLite stores only content-derived IDs and MIME strings. Small, bounded PNG/JPEG
payloads are saved under the application-data `artwork/` directory, and restored
through the existing binary artwork command. IDs are validated and must be
referenced by the saved library. No arbitrary cache-file path is accepted from
React. Existing cache files are checked, not overwritten on a content mismatch.
Artwork writes occur before the metadata transaction; failed/cancelled scans can
leave unused cache files, but cannot leave partial database metadata. Cache
garbage collection is not implemented in this task.

## Verification

Rust tests use temporary databases and fixture directories. They cover reopening,
folder/track uniqueness, metadata updates, overlapping roots, missing paths,
cancelled/partial scans, artwork references, transaction rollback via an injected
SQLite trigger failure, and unknown/newer schemas. No personal music folder is
accessed by automated tests.

Manual verification: restart the native app, select and scan a test folder,
restart again, verify metadata and artwork restore, disconnect/reconnect a test
drive, then rescan and check unavailable markers and duplicate prevention.
