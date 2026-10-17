use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};

pub const MAX_IMAGE_BYTES: usize = 256 * 1024;
pub const MAX_CACHE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Default)]
pub struct ArtworkCache {
    pub images: HashMap<String, Vec<u8>>,
    bytes: usize,
}

/// Only small PNG/JPEG rasters with bounded dimensions are sent to WebView.
pub fn raster_mime(data: &[u8]) -> Option<&'static str> {
    let valid_size = |w: u32, h: u32| w > 0 && h > 0 && w <= 2048 && h <= 2048;
    if data.len() > MAX_IMAGE_BYTES {
        return None;
    }
    if data.len() >= 24 && data.starts_with(b"\x89PNG\r\n\x1a\n") && &data[12..16] == b"IHDR" {
        let w = u32::from_be_bytes(data[16..20].try_into().ok()?);
        let h = u32::from_be_bytes(data[20..24].try_into().ok()?);
        return valid_size(w, h).then_some("image/png");
    }
    if !data.starts_with(&[0xff, 0xd8]) {
        return None;
    }
    let mut i = 2;
    while i + 4 <= data.len() {
        if data[i] != 0xff {
            return None;
        }
        let marker = data[i + 1];
        i += 2;
        if marker == 0xff {
            i -= 1;
            continue;
        }
        if marker == 0xda || marker == 0xd9 {
            return None;
        }
        if marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        let length = u16::from_be_bytes([data[i], data[i + 1]]) as usize;
        if length < 2 || i + length > data.len() {
            return None;
        }
        if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) && length >= 8 {
            let h = u16::from_be_bytes([data[i + 3], data[i + 4]]) as u32;
            let w = u16::from_be_bytes([data[i + 5], data[i + 6]]) as u32;
            return valid_size(w, h).then_some("image/jpeg");
        }
        i += length;
    }
    None
}

impl ArtworkCache {
    pub fn insert(&mut self, _generation: u64, data: &[u8]) -> Option<(String, String)> {
        let mime = raster_mime(data)?;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        data.hash(&mut hasher);
        let id = format!("{:016x}", hasher.finish());
        if let Some(existing) = self.images.get(&id) {
            return (existing == data).then(|| (id, mime.into()));
        }
        if self.bytes + data.len() > MAX_CACHE_BYTES {
            return None;
        }
        self.bytes += data.len();
        self.images.insert(id.clone(), data.to_vec());
        Some((id, mime.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_and_oversized_images_are_rejected() {
        assert!(raster_mime(b"not an image").is_none());
        assert!(raster_mime(&vec![0; MAX_IMAGE_BYTES + 1]).is_none());
        assert!(raster_mime(b"<svg></svg>").is_none());
    }
    #[test]
    fn raster_dimensions_are_bounded_and_cache_deduplicates() {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend(32u32.to_be_bytes());
        png.extend(32u32.to_be_bytes());
        let mut cache = ArtworkCache::default();
        assert_eq!(cache.insert(1, &png), cache.insert(1, &png));
        assert_eq!(cache.images.len(), 1);
        png[16..20].copy_from_slice(&10000u32.to_be_bytes());
        assert!(raster_mime(&png).is_none());
    }
}
