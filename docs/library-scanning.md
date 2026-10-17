# Local library scanning

Select folders in the sidebar, then use **Scan Music** in the library. Scan Again
updates the saved SQLite library from all selected roots. There is no audio
playback or music-file modification. See [library-storage.md](library-storage.md)
for persistence and reconciliation details.

## Native boundary

`select_music_folder` uses the official Tauri dialog plugin and records the chosen
canonical path in the native SQLite library. React does not grant
filesystem access by submitting an arbitrary root. `scan_music` accepts only
registered roots, canonicalizes them again, and rejects replaced targets.
Missing roots produce a warning if other selected roots remain usable; if none
are usable the command reports an error and retains the previous library.

Recursive traversal skips symlinks and all Windows reparse points, including
junctions. Every discovered file is canonicalized, checked against the selected
roots, and deduplicated. Selected overlapping directories are visited once.
No frontend filesystem plugin permission, asset protocol scope, or directory
write access is enabled. Command permissions are restricted to the local main
window through Tauri's generated application command manifest.

## Metadata and resource bounds

Lofty 0.25.4 reads MP3, FLAC, WAV, M4A/MP4, AAC, OGG Vorbis, and OPUS. WMA is not
supported by this parser and is ignored. Metadata includes path, title, artist,
album, album artist, duration, track/disc number, year, and detected format.
Absent numeric fields stay null. Missing text uses filename, Unknown Artist,
or Unknown Album as appropriate. The primary tag takes priority; other tags can
fill incomplete fields. Unparseable audio is skipped. If a file's properties
can be read with tags disabled, it remains in the library with fallback text
and a recovered-tag warning.

Scanning runs on Tauri's blocking worker pool, with one concurrent scan allowed.
Progress messages are throttled to 200 ms. Cancellation is cooperative between
files and directory entries; it cannot interrupt the current parser call.
Cancelled or failed scans keep the previous React library. Scans stop at 100,000
supported candidate files and explicitly report that limit. The track list uses
50-row pages and deferred search to bound React rendering work.

## Embedded artwork

Artwork stays in a deduplicated native memory cache, with a 16 MiB total budget.
Only PNG/JPEG payloads at most 256 KiB, with header dimensions no larger than
2048 by 2048, are retained. Oversized/unsupported images use the common fallback;
no thumbnail dependency is needed. Lofty's per-tag-item allocation limit is
16 MiB, including during artwork parsing.

Scan results contain opaque artwork IDs and MIME types, not base64 images or
file URLs. Visible track rows request binary data through `get_music_artwork`.
React stores only blob URL strings and revokes them on unmount. WebView image
decode failures show the same fallback. SVG and remote artwork are never loaded.
The native cache is replaced after a completed scan, and preserved on cancellation.

## Verification

Run `cargo test --locked --offline --manifest-path src-tauri/Cargo.toml`,
`npm.cmd run build`, and `node --experimental-strip-types --test tests/*.test.mjs`.
Rust tests create their own temporary audio/tag fixtures. The Windows junction
test creates and removes a junction entirely within temporary test fixtures.

After native command changes, restart `tauri dev`. Saved folders restore automatically.
Manual Windows testing should cover multiple audio formats, Unicode/long paths,
removable drives, cancellation of a large collection, rescans, and embedded covers.
Automated fixtures do not verify a personal music library or real playback.
