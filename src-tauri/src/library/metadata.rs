use super::{
    artwork::ArtworkCache,
    files::{fallback_title, text_or},
};
use lofty::{
    config::ParseOptions,
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::{Accessor, ItemKey},
};
use serde::Serialize;
use std::path::Path;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtworkRef {
    pub id: String,
    pub mime: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicTrack {
    pub id: String,
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: Option<String>,
    pub duration_seconds: Option<f64>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub year: Option<u32>,
    pub format: String,
    pub artwork: Option<ArtworkRef>,
    pub available: bool,
    pub favorite: bool,
}

pub fn read_track(
    path: &Path,
    cache: &mut ArtworkCache,
    generation: u64,
) -> Result<(MusicTrack, bool), ()> {
    let read = |tags| {
        let probe = Probe::open(path).map_err(|_| ())?;
        let probe = probe.guess_file_type().map_err(|_| ())?;
        probe
            .options(ParseOptions::new().read_tags(tags))
            .read()
            .map_err(|_| ())
    };
    // A damaged tag can still leave a valid, playable audio container.
    let (file, recovered) = match read(true) {
        Ok(file) => (file, false),
        Err(_) => (read(false).map_err(|_| ())?, true),
    };
    // Prefer the primary tag, but fill incomplete fields from other tag types.
    let tags: Vec<_> = file
        .primary_tag()
        .into_iter()
        .chain(file.tags().iter())
        .collect();
    let title = tags
        .iter()
        .find_map(|t| t.title().filter(|s| !s.trim().is_empty()));
    let artist = tags
        .iter()
        .find_map(|t| t.artist().filter(|s| !s.trim().is_empty()));
    let album = tags
        .iter()
        .find_map(|t| t.album().filter(|s| !s.trim().is_empty()));
    let album_artist = tags
        .iter()
        .find_map(|t| {
            t.get_string(ItemKey::AlbumArtist)
                .filter(|s| !s.trim().is_empty())
        })
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| text_or(Some(s), ""));
    let artwork = file
        .tags()
        .iter()
        .flat_map(|t| t.pictures())
        .find_map(|picture| {
            cache
                .insert(generation, picture.data())
                .map(|(id, mime)| ArtworkRef { id, mime })
        });
    let duration = file.properties().duration().as_secs_f64();
    let full_path = path.to_string_lossy().into_owned();
    Ok((
        MusicTrack {
            id: full_path.clone(),
            path: full_path,
            title: text_or(title.as_deref(), &fallback_title(path)),
            artist: text_or(artist.as_deref(), "Unknown Artist"),
            album: text_or(album.as_deref(), "Unknown Album"),
            album_artist,
            duration_seconds: (duration > 0.0).then_some(duration),
            track_number: tags
                .iter()
                .find_map(|t| t.track().filter(|number| *number > 0)),
            disc_number: tags
                .iter()
                .find_map(|t| t.disk().filter(|number| *number > 0)),
            year: tags
                .iter()
                .find_map(|t| t.date())
                .map(|date| u32::from(date.year))
                .filter(|year| *year > 0),
            format: match format!("{:?}", file.file_type()).as_str() {
                "Mpeg" => "MP3".into(),
                "Mp4" => "M4A/MP4".into(),
                "Vorbis" => "OGG/Vorbis".into(),
                other => other.to_uppercase(),
            },
            artwork,
            available: true,
            favorite: false,
        },
        recovered,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::files::tests::Fixture;
    pub fn wav() -> Vec<u8> {
        let samples = 800u32;
        let size = samples * 2;
        let mut data = b"RIFF".to_vec();
        data.extend((36 + size).to_le_bytes());
        data.extend(b"WAVEfmt ");
        data.extend(16u32.to_le_bytes());
        data.extend(1u16.to_le_bytes());
        data.extend(1u16.to_le_bytes());
        data.extend(8000u32.to_le_bytes());
        data.extend(16000u32.to_le_bytes());
        data.extend(2u16.to_le_bytes());
        data.extend(16u16.to_le_bytes());
        data.extend(b"data");
        data.extend(size.to_le_bytes());
        data.resize(44 + size as usize, 0);
        data
    }
    #[test]
    fn valid_untagged_wav_has_real_properties_and_fallbacks() {
        let f = Fixture::new();
        let path = f.file("Evening.WAV", &wav());
        let (track, _) = read_track(&path, &mut ArtworkCache::default(), 1).unwrap();
        assert_eq!(track.title, "Evening");
        assert_eq!(track.artist, "Unknown Artist");
        assert_eq!(track.album, "Unknown Album");
        assert!(track.album_artist.is_none());
        assert!(track.year.is_none());
        assert!(track.artwork.is_none());
        assert!((track.duration_seconds.unwrap() - 0.1).abs() < 0.01);
    }
    #[test]
    fn malformed_and_missing_audio_is_rejected() {
        let f = Fixture::new();
        let path = f.file("bad.mp3", b"not audio");
        assert!(read_track(&path, &mut ArtworkCache::default(), 1).is_err());
        assert!(read_track(&f.0.join("missing.wav"), &mut ArtworkCache::default(), 1).is_err());
    }
    #[test]
    fn mixed_nested_scan_continues_after_malformed_audio() {
        let f = Fixture::new();
        f.file("nested/track.wav", &wav());
        f.file("bad.mp3", b"bad");
        let mut tracks = Vec::new();
        let mut cache = ArtworkCache::default();
        let stats = crate::library::files::walk(
            &[f.0.clone()],
            &std::sync::atomic::AtomicBool::new(false),
            |path| match read_track(path, &mut cache, 1) {
                Ok((track, _)) => {
                    tracks.push(track);
                    true
                }
                Err(_) => false,
            },
            |_| {},
        );
        assert_eq!(stats.discovered, 2);
        assert_eq!(stats.skipped, 1);
        assert_eq!(tracks.len(), 1);
    }

    #[test]
    fn tagged_fixture_extracts_metadata_and_artwork_without_changing_file() {
        use lofty::{
            config::WriteOptions,
            picture::{Picture, PictureType},
            tag::{Tag, TagExt, TagType, items::Timestamp},
        };
        let f = Fixture::new();
        let path = f.file("tagged.wav", &wav());
        let mut tag = Tag::new(TagType::Id3v2);
        tag.set_title("Evening light".into());
        tag.set_artist("Fixture Artist".into());
        tag.set_album("Fixture Album".into());
        tag.insert_text(ItemKey::AlbumArtist, "Fixture Ensemble".into());
        tag.set_track(3);
        tag.set_disk(2);
        tag.set_date(Timestamp {
            year: 2024,
            ..Default::default()
        });
        // Small raster header is enough to test bounded cache transport; UI decoding has its own fallback.
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend(32u32.to_be_bytes());
        png.extend(32u32.to_be_bytes());
        let picture = Picture::unchecked(png.clone())
            .pic_type(PictureType::CoverFront)
            .build();
        tag.push_picture(picture);
        tag.save_to_path(&path, WriteOptions::default()).unwrap();
        let before = std::fs::read(&path).unwrap();
        let mut cache = ArtworkCache::default();
        let (track, _) = read_track(&path, &mut cache, 9).unwrap();
        assert_eq!(track.title, "Evening light");
        assert_eq!(track.artist, "Fixture Artist");
        assert_eq!(track.album, "Fixture Album");
        assert_eq!(track.album_artist.as_deref(), Some("Fixture Ensemble"));
        assert_eq!(track.track_number, Some(3));
        assert_eq!(track.disc_number, Some(2));
        assert_eq!(track.year, Some(2024));
        assert_eq!(cache.images.get(&track.artwork.unwrap().id), Some(&png));
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn malformed_tag_does_not_discard_valid_wav_audio() {
        let f = Fixture::new();
        let mut bytes = wav();
        bytes.extend(b"id3 ");
        bytes.extend(10u32.to_le_bytes());
        bytes.extend(b"ID3\xff\0\0\0\0\0\0");
        let size = bytes.len() as u32 - 8;
        bytes[4..8].copy_from_slice(&size.to_le_bytes());
        let path = f.file("corrupt-tags.wav", &bytes);
        let (track, _) = read_track(&path, &mut ArtworkCache::default(), 1).unwrap();
        assert_eq!(track.title, "corrupt-tags");
        assert!(track.duration_seconds.is_some());
    }
}
