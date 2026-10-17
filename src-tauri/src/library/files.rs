//! Read-only traversal and authorization, independent of the metadata parser.
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

pub const MAX_FILES: usize = 100_000;

#[derive(Default, Clone, Debug)]
pub struct WalkStats {
    pub discovered: usize,
    pub skipped: usize,
    pub warnings: usize,
    pub cancelled: bool,
    pub limited: bool,
}

pub fn supported(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "mp3" | "flac" | "wav" | "m4a" | "aac" | "ogg" | "opus"
            )
        })
}

pub fn path_key(path: &Path) -> String {
    let key = path
        .to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_owned();
    if cfg!(windows) {
        key.to_lowercase()
    } else {
        path.to_string_lossy().into_owned()
    }
}

/// Component-aware comparison; Windows identities use the same case normalization as SQLite.
pub fn within_root(path: &Path, root: &Path) -> bool {
    if cfg!(windows) {
        Path::new(&path_key(path)).starts_with(Path::new(&path_key(root)))
    } else {
        path.starts_with(root)
    }
}

pub fn text_or(value: Option<&str>, fallback: &str) -> String {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback)
        .chars()
        .take(2048)
        .collect()
}

pub fn fallback_title(path: &Path) -> String {
    text_or(path.file_stem().and_then(|s| s.to_str()), "Untitled")
}

pub fn is_link(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // Includes directory junctions and other reparse points, not just symlinks.
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    false
}

/// Only canonical paths saved through the native folder picker are authorized.
/// Re-canonicalize to reject roots replaced with a different link target.
pub fn authorized_roots(
    requested: &[String],
    selected: &[(String, PathBuf)],
) -> Result<(Vec<PathBuf>, usize), String> {
    if requested.is_empty() || requested.len() > 128 {
        return Err("Select between 1 and 128 music folders.".into());
    }
    let mut roots = Vec::new();
    let mut missing = 0;
    for request in requested {
        let key = path_key(Path::new(request));
        let original = selected
            .iter()
            .find(|(path, _)| path_key(Path::new(path)) == key)
            .map(|(_, canonical)| canonical)
            .ok_or("A scan root was not selected using the music folder picker.")?;
        match fs::canonicalize(request) {
            Ok(current) if path_key(&current) == path_key(original) && current.is_dir() => {
                if !roots.contains(&current) {
                    roots.push(current);
                }
            }
            _ => missing += 1,
        }
    }
    if roots.is_empty() {
        return Err(
            "The selected folders are missing, inaccessible, or have changed. Select them again."
                .into(),
        );
    }
    Ok((roots, missing))
}

/// Callback returns false for unreadable or malformed audio. No file is written.
pub fn walk(
    roots: &[PathBuf],
    cancel: &AtomicBool,
    mut read_audio: impl FnMut(&Path) -> bool,
    mut progress: impl FnMut(&WalkStats),
) -> WalkStats {
    let mut stats = WalkStats::default();
    let mut pending = roots.to_vec();
    let mut directories = HashSet::new();
    let mut files = HashSet::new();
    while let Some(directory) = pending.pop() {
        if cancel.load(Ordering::Relaxed) {
            stats.cancelled = true;
            break;
        }
        let directory = match fs::canonicalize(&directory) {
            Ok(path) if roots.iter().any(|root| path.starts_with(root)) => path,
            _ => {
                stats.warnings += 1;
                continue;
            }
        };
        if !directories.insert(path_key(&directory)) {
            continue;
        }
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(_) => {
                stats.warnings += 1;
                continue;
            }
        };
        for entry in entries {
            if cancel.load(Ordering::Relaxed) {
                stats.cancelled = true;
                break;
            }
            let entry = match entry {
                Ok(value) => value,
                Err(_) => {
                    stats.warnings += 1;
                    continue;
                }
            };
            let path = entry.path();
            let metadata = match fs::symlink_metadata(&path) {
                Ok(value) => value,
                Err(_) => {
                    stats.warnings += 1;
                    continue;
                }
            };
            if is_link(&metadata) {
                continue;
            }
            if metadata.is_dir() {
                pending.push(path);
                continue;
            }
            if !metadata.is_file() || !supported(&path) {
                continue;
            }
            let canonical = match fs::canonicalize(&path) {
                Ok(value) if roots.iter().any(|root| value.starts_with(root)) => value,
                _ => {
                    stats.warnings += 1;
                    continue;
                }
            };
            if !files.insert(path_key(&canonical)) {
                continue;
            }
            if stats.discovered == MAX_FILES {
                stats.limited = true;
                break;
            }
            stats.discovered += 1;
            if !read_audio(&canonical) {
                stats.skipped += 1;
            }
            progress(&stats);
        }
        progress(&stats);
        if stats.cancelled || stats.limited {
            break;
        }
    }
    stats
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    pub struct Fixture(pub PathBuf);
    impl Fixture {
        pub fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "afterhours-scan-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(fs::canonicalize(path).unwrap())
        }
        pub fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.0.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, bytes).unwrap();
            path
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn extensions_are_case_insensitive() {
        for name in [
            "a.MP3", "a.FLAC", "a.WaV", "a.m4a", "a.aac", "a.OGG", "a.OPUS",
        ] {
            assert!(supported(Path::new(name)));
        }
        for name in ["a.wma", "a.txt", "a.mp3.exe", "mp3", "cover.jpg"] {
            assert!(!supported(Path::new(name)));
        }
    }
    #[test]
    fn absent_and_blank_metadata_use_fallbacks() {
        assert_eq!(text_or(None, "Unknown Artist"), "Unknown Artist");
        assert_eq!(text_or(Some("  "), "Unknown Album"), "Unknown Album");
        assert_eq!(text_or(Some(" Artist "), "Unknown Artist"), "Artist");
        assert_eq!(fallback_title(Path::new("01 Evening.wav")), "01 Evening");
    }
    #[test]
    fn nested_files_and_overlapping_roots_are_deduplicated() {
        let f = Fixture::new();
        f.file("nested/deeper/a.MP3", b"audio");
        f.file("b.flac", b"audio");
        f.file("cover.jpg", b"image");
        let mut paths = Vec::new();
        let stats = walk(
            &[f.0.clone(), f.0.join("nested")],
            &AtomicBool::new(false),
            |path| {
                paths.push(path.to_owned());
                true
            },
            |_| {},
        );
        assert_eq!(stats.discovered, 2);
        assert_eq!(paths.len(), 2);
    }
    #[test]
    fn bad_and_deleted_files_do_not_abort_other_files() {
        let f = Fixture::new();
        f.file("bad.mp3", b"broken");
        f.file("good.wav", b"audio");
        let stats = walk(
            &[f.0.clone()],
            &AtomicBool::new(false),
            |path| {
                if path.ends_with("bad.mp3") {
                    fs::remove_file(path).unwrap();
                    return fs::read(path).is_ok();
                }
                true
            },
            |_| {},
        );
        assert_eq!(stats.discovered, 2);
        assert_eq!(stats.skipped, 1);
    }
    #[test]
    fn empty_and_missing_directories_are_safe() {
        let f = Fixture::new();
        assert_eq!(
            walk(
                &[f.0.clone()],
                &AtomicBool::new(false),
                |_| panic!(),
                |_| {}
            )
            .discovered,
            0
        );
        assert_eq!(
            walk(
                &[f.0.join("missing")],
                &AtomicBool::new(false),
                |_| panic!(),
                |_| {}
            )
            .warnings,
            1
        );
    }
    #[test]
    fn cancellation_stops_before_reading() {
        let f = Fixture::new();
        f.file("a.mp3", b"audio");
        let stats = walk(&[f.0.clone()], &AtomicBool::new(true), |_| panic!(), |_| {});
        assert!(stats.cancelled);
        assert_eq!(stats.discovered, 0);
    }
    #[test]
    fn cancellation_during_scan_stops_the_next_file() {
        let f = Fixture::new();
        f.file("a.mp3", b"audio");
        f.file("b.mp3", b"audio");
        let cancel = AtomicBool::new(false);
        let stats = walk(
            &[f.0.clone()],
            &cancel,
            |_| {
                cancel.store(true, Ordering::Relaxed);
                true
            },
            |_| {},
        );
        assert!(stats.cancelled);
        assert_eq!(stats.discovered, 1);
    }
    #[test]
    fn arbitrary_roots_and_root_prefix_lookalikes_are_rejected() {
        let f = Fixture::new();
        let selected = vec![(f.0.to_string_lossy().into_owned(), f.0.clone())];
        assert!(authorized_roots(&[f.0.to_string_lossy().into_owned()], &selected).is_ok());
        assert!(authorized_roots(&[format!("{}-other", f.0.display())], &selected).is_err());
        assert!(authorized_roots(&[], &selected).is_err());
    }
    #[test]
    fn unavailable_selected_root_reports_error() {
        let f = Fixture::new();
        let missing = f.0.join("gone");
        let requested = missing.to_string_lossy().into_owned();
        assert!(authorized_roots(&[requested.clone()], &[(requested, missing)]).is_err());
    }
    #[test]
    fn file_read_permission_errors_are_counted() {
        let f = Fixture::new();
        f.file("a.mp3", b"audio");
        let stats = walk(&[f.0.clone()], &AtomicBool::new(false), |_| false, |_| {});
        assert_eq!(stats.skipped, 1);
    }
    #[cfg(unix)]
    #[test]
    fn symlinks_outside_root_and_loops_are_not_followed() {
        use std::os::unix::fs::symlink;
        let f = Fixture::new();
        let outside = Fixture::new();
        outside.file("a.mp3", b"audio");
        symlink(&outside.0, f.0.join("outside")).unwrap();
        symlink(&f.0, f.0.join("loop")).unwrap();
        assert_eq!(
            walk(
                &[f.0.clone()],
                &AtomicBool::new(false),
                |_| panic!(),
                |_| {}
            )
            .discovered,
            0
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_junction_outside_root_is_not_followed() {
        let f = Fixture::new();
        let outside = Fixture::new();
        outside.file("outside.mp3", b"audio");
        let junction = f.0.join("junction");
        let output = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&junction)
            .arg(&outside.0)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Could not create test junction: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(is_link(&fs::symlink_metadata(&junction).unwrap()));
        let stats = walk(
            &[f.0.clone()],
            &AtomicBool::new(false),
            |_| panic!("Must not follow junction"),
            |_| {},
        );
        fs::remove_dir(&junction).unwrap();
        assert_eq!(stats.discovered, 0);
        assert!(outside.0.join("outside.mp3").exists());
    }
}
