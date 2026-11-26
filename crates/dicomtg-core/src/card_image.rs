use std::{
    error::Error,
    fmt, fs,
    fs::{FileTimes, OpenOptions},
    io::{self, Read},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use sha2::{Digest, Sha256};
use ureq::Agent;

const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;
pub const CARD_IMAGE_CACHE_CAPACITY_BYTES: u64 = 250 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardImageCacheStats {
    image_count: usize,
    total_bytes: u64,
}

impl CardImageCacheStats {
    #[must_use]
    pub fn image_count(self) -> usize {
        self.image_count
    }

    #[must_use]
    pub fn total_bytes(self) -> u64 {
        self.total_bytes
    }

    #[must_use]
    pub fn capacity_bytes(self) -> u64 {
        CARD_IMAGE_CACHE_CAPACITY_BYTES
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardImageSize {
    Small,
    Normal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardImageFace {
    Front,
    Back,
}

#[derive(Debug)]
pub enum CardImageError {
    InvalidRequest(&'static str),
    InvalidImage,
    TooLarge,
    Io(io::Error),
    Http(ureq::Error),
}

impl fmt::Display for CardImageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => {
                write!(formatter, "invalid card image request: {message}")
            }
            Self::InvalidImage => write!(formatter, "the downloaded card image is not a JPEG"),
            Self::TooLarge => write!(formatter, "the downloaded card image exceeds 5 MiB"),
            Self::Io(error) => write!(formatter, "card image cache I/O error: {error}"),
            Self::Http(error) => write!(formatter, "card image download failed: {error}"),
        }
    }
}

impl Error for CardImageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Http(error) => Some(error),
            Self::InvalidRequest(_) | Self::InvalidImage | Self::TooLarge => None,
        }
    }
}

impl From<io::Error> for CardImageError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<ureq::Error> for CardImageError {
    fn from(error: ureq::Error) -> Self {
        Self::Http(error)
    }
}

pub fn load_card_image(
    cache_directory: impl AsRef<Path>,
    scryfall_id: Option<&str>,
    set_code: &str,
    collector_number: &str,
    size: CardImageSize,
    face: CardImageFace,
) -> Result<Vec<u8>, CardImageError> {
    let agent = Agent::new_with_config(
        Agent::config_builder()
            .https_only(true)
            .timeout_global(Some(Duration::from_secs(30)))
            .build(),
    );
    load_card_image_with(
        cache_directory.as_ref(),
        scryfall_id,
        set_code,
        collector_number,
        size,
        face,
        |url| download_image(&agent, url),
    )
}

fn load_card_image_with(
    cache_directory: &Path,
    scryfall_id: Option<&str>,
    set_code: &str,
    collector_number: &str,
    size: CardImageSize,
    face: CardImageFace,
    download: impl FnOnce(&str) -> Result<Vec<u8>, CardImageError>,
) -> Result<Vec<u8>, CardImageError> {
    let url = card_image_url(scryfall_id, set_code, collector_number, size, face)?;
    let cache_path = cache_path(
        cache_directory,
        scryfall_id,
        set_code,
        collector_number,
        size,
        face,
    );
    match fs::read(&cache_path) {
        Ok(bytes) if is_jpeg(&bytes) && bytes.len() <= MAX_IMAGE_BYTES => {
            touch_cache_file(&cache_path)?;
            prune_card_image_cache(
                cache_directory,
                CARD_IMAGE_CACHE_CAPACITY_BYTES,
                Some(&cache_path),
            )?;
            return Ok(bytes);
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }

    let bytes = download(&url)?;
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(CardImageError::TooLarge);
    }
    if !is_jpeg(&bytes) {
        return Err(CardImageError::InvalidImage);
    }

    fs::create_dir_all(cache_directory)?;
    fs::write(&cache_path, &bytes)?;
    prune_card_image_cache(
        cache_directory,
        CARD_IMAGE_CACHE_CAPACITY_BYTES,
        Some(&cache_path),
    )?;
    Ok(bytes)
}

pub fn card_image_cache_stats(
    cache_directory: impl AsRef<Path>,
) -> Result<CardImageCacheStats, CardImageError> {
    let entries = cache_entries(cache_directory.as_ref())?;
    Ok(CardImageCacheStats {
        image_count: entries.len(),
        total_bytes: entries.iter().map(|entry| entry.bytes).sum(),
    })
}

pub fn clear_card_image_cache(
    cache_directory: impl AsRef<Path>,
) -> Result<CardImageCacheStats, CardImageError> {
    let cache_directory = cache_directory.as_ref();
    for entry in cache_entries(cache_directory)? {
        fs::remove_file(entry.path)?;
    }
    card_image_cache_stats(cache_directory)
}

#[derive(Debug)]
struct CacheEntry {
    path: PathBuf,
    bytes: u64,
    modified: SystemTime,
}

fn cache_entries(directory: &Path) -> Result<Vec<CacheEntry>, CardImageError> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut images = Vec::new();
    for entry in entries {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_file() && is_cache_file(&entry.path()) {
            images.push(CacheEntry {
                path: entry.path(),
                bytes: metadata.len(),
                modified: metadata.modified().unwrap_or(UNIX_EPOCH),
            });
        }
    }
    Ok(images)
}

fn prune_card_image_cache(
    directory: &Path,
    capacity_bytes: u64,
    protected_path: Option<&Path>,
) -> Result<(), CardImageError> {
    let mut entries = cache_entries(directory)?;
    let mut total_bytes = entries.iter().map(|entry| entry.bytes).sum::<u64>();
    entries.sort_by(|left, right| {
        let left_is_protected = protected_path.is_some_and(|path| path == left.path);
        let right_is_protected = protected_path.is_some_and(|path| path == right.path);
        left_is_protected
            .cmp(&right_is_protected)
            .then_with(|| left.modified.cmp(&right.modified))
    });

    for entry in entries {
        if total_bytes <= capacity_bytes {
            break;
        }
        fs::remove_file(entry.path)?;
        total_bytes = total_bytes.saturating_sub(entry.bytes);
    }
    Ok(())
}

fn touch_cache_file(path: &Path) -> Result<(), CardImageError> {
    OpenOptions::new()
        .write(true)
        .open(path)?
        .set_times(FileTimes::new().set_modified(SystemTime::now()))?;
    Ok(())
}

fn is_cache_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            (name.starts_with("small-") || name.starts_with("normal-")) && name.ends_with(".jpg")
        })
}

fn card_image_url(
    scryfall_id: Option<&str>,
    set_code: &str,
    collector_number: &str,
    size: CardImageSize,
    face: CardImageFace,
) -> Result<String, CardImageError> {
    let size = size.as_str();
    let face = face.as_str();
    if let Some(id) = scryfall_id.map(str::trim).filter(|id| is_scryfall_id(id)) {
        let id = id.to_ascii_lowercase();
        return Ok(format!(
            "https://cards.scryfall.io/{size}/{face}/{}/{}/{id}.jpg",
            &id[0..1],
            &id[1..2]
        ));
    }

    let set_code = set_code.trim();
    let collector_number = collector_number.trim();
    if set_code.is_empty() || set_code.len() > 32 {
        return Err(CardImageError::InvalidRequest("invalid set code"));
    }
    if collector_number.is_empty() || collector_number.len() > 128 {
        return Err(CardImageError::InvalidRequest("invalid collector number"));
    }

    Ok(format!(
        "https://api.scryfall.com/cards/{}/{}?format=image&version={size}&face={face}",
        encode_path_segment(&set_code.to_ascii_lowercase()),
        encode_path_segment(collector_number)
    ))
}

impl CardImageSize {
    fn as_str(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Normal => "normal",
        }
    }
}

impl CardImageFace {
    fn as_str(self) -> &'static str {
        match self {
            Self::Front => "front",
            Self::Back => "back",
        }
    }
}

fn cache_path(
    directory: &Path,
    scryfall_id: Option<&str>,
    set_code: &str,
    collector_number: &str,
    size: CardImageSize,
    face: CardImageFace,
) -> PathBuf {
    let key = scryfall_id
        .map(str::trim)
        .filter(|id| is_scryfall_id(id))
        .map(str::to_ascii_lowercase)
        .unwrap_or_else(|| {
            let mut hasher = Sha256::new();
            hasher.update(set_code.trim().to_ascii_lowercase());
            hasher.update([0]);
            hasher.update(collector_number.trim());
            format!("{:x}", hasher.finalize())
        });
    let face_suffix = match face {
        CardImageFace::Front => "",
        CardImageFace::Back => "-back",
    };
    directory.join(format!("{}{face_suffix}-{key}.jpg", size.as_str()))
}

fn download_image(agent: &Agent, url: &str) -> Result<Vec<u8>, CardImageError> {
    let response = agent.get(url).call()?;
    let mut reader = response
        .into_body()
        .into_reader()
        .take(MAX_IMAGE_BYTES as u64 + 1);
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(CardImageError::TooLarge);
    }
    Ok(bytes)
}

fn is_jpeg(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xff, 0xd8, 0xff]) && bytes.ends_with(&[0xff, 0xd9])
}

pub(crate) fn is_scryfall_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => *byte == b'-',
            _ => byte.is_ascii_hexdigit(),
        })
        && matches!(bytes[14].to_ascii_lowercase(), b'1'..=b'5')
        && matches!(bytes[19].to_ascii_lowercase(), b'8' | b'9' | b'a' | b'b')
}

pub(crate) fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use std::{
        cell::Cell,
        fs::{self, File, FileTimes},
        path::Path,
        time::{Duration, UNIX_EPOCH},
    };

    use super::{
        CardImageError, CardImageFace, CardImageSize, cache_path, card_image_cache_stats,
        card_image_url, clear_card_image_cache, load_card_image_with, prune_card_image_cache,
    };

    const ID: &str = "5feddbd0-3d29-4428-b70d-107c1e26930d";
    const JPEG: &[u8] = &[0xff, 0xd8, 0xff, 0xdb, 0x00, 0x01, 0xff, 0xd9];

    #[test]
    fn ui_007_uses_the_static_cdn_and_a_safe_legacy_fallback() {
        assert_eq!(
            card_image_url(
                Some(ID),
                "WWK",
                "1",
                CardImageSize::Small,
                CardImageFace::Front,
            )
            .unwrap(),
            format!("https://cards.scryfall.io/small/front/5/f/{ID}.jpg")
        );
        assert_eq!(
            card_image_url(
                None,
                "T ST",
                "001/a",
                CardImageSize::Normal,
                CardImageFace::Front,
            )
            .unwrap(),
            "https://api.scryfall.com/cards/t%20st/001%2Fa?format=image&version=normal&face=front"
        );
    }

    #[test]
    fn ui_007_uses_a_distinct_url_and_cache_entry_for_the_back_face() {
        assert_eq!(
            card_image_url(
                Some(ID),
                "VOW",
                "141",
                CardImageSize::Normal,
                CardImageFace::Back,
            )
            .unwrap(),
            format!("https://cards.scryfall.io/normal/back/5/f/{ID}.jpg")
        );
        assert_ne!(
            cache_path(
                Path::new("cache"),
                Some(ID),
                "VOW",
                "141",
                CardImageSize::Normal,
                CardImageFace::Front,
            ),
            cache_path(
                Path::new("cache"),
                Some(ID),
                "VOW",
                "141",
                CardImageSize::Normal,
                CardImageFace::Back,
            )
        );
    }

    #[test]
    fn ui_007_downloads_once_then_reads_the_persistent_cache() {
        let directory = std::env::temp_dir().join(format!(
            "dicomtg-image-cache-{}-{}",
            std::process::id(),
            line!()
        ));
        let calls = Cell::new(0);

        let first = load_card_image_with(
            &directory,
            Some(ID),
            "WWK",
            "1",
            CardImageSize::Normal,
            CardImageFace::Front,
            |_| {
                calls.set(calls.get() + 1);
                Ok(JPEG.to_vec())
            },
        )
        .unwrap();
        let cached = directory.join(format!("normal-{ID}.jpg"));
        let old_modified = UNIX_EPOCH + Duration::from_secs(1);
        File::options()
            .write(true)
            .open(&cached)
            .unwrap()
            .set_times(FileTimes::new().set_modified(old_modified))
            .unwrap();
        let second = load_card_image_with(
            &directory,
            Some(ID),
            "WWK",
            "1",
            CardImageSize::Normal,
            CardImageFace::Front,
            |_| panic!("a cache hit must not use the network"),
        )
        .unwrap();

        assert_eq!(first, JPEG);
        assert_eq!(second, JPEG);
        assert_eq!(calls.get(), 1);
        assert!(fs::metadata(cached).unwrap().modified().unwrap() > old_modified);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn ui_007_rejects_non_jpeg_responses() {
        let directory = std::env::temp_dir().join(format!(
            "dicomtg-image-cache-{}-{}",
            std::process::id(),
            line!()
        ));

        let error = load_card_image_with(
            &directory,
            Some(ID),
            "WWK",
            "1",
            CardImageSize::Normal,
            CardImageFace::Front,
            |_| Ok(b"not an image".to_vec()),
        )
        .expect_err("invalid remote content must not be cached");

        assert!(matches!(error, CardImageError::InvalidImage));
        assert!(!directory.exists());
    }

    #[test]
    fn ui_009_evicts_the_least_recently_used_images() {
        let directory = test_directory(line!());
        fs::create_dir_all(&directory).unwrap();
        let oldest = directory.join("small-oldest.jpg");
        let recent = directory.join("small-recent.jpg");
        write_jpeg(&oldest, 8, 1);
        write_jpeg(&recent, 8, 2);

        prune_card_image_cache(&directory, 8, None).unwrap();

        assert!(!oldest.exists());
        assert!(recent.exists());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn ui_009_reports_and_clears_only_cached_images() {
        let directory = test_directory(line!());
        fs::create_dir_all(&directory).unwrap();
        write_jpeg(&directory.join("small-one.jpg"), 8, 1);
        write_jpeg(&directory.join("normal-two.jpg"), 12, 2);
        fs::write(directory.join("keep.txt"), b"keep").unwrap();

        let before = card_image_cache_stats(&directory).unwrap();
        let after = clear_card_image_cache(&directory).unwrap();

        assert_eq!(before.image_count(), 2);
        assert_eq!(before.total_bytes(), 20);
        assert_eq!(before.capacity_bytes(), 250 * 1024 * 1024);
        assert_eq!(after.image_count(), 0);
        assert_eq!(after.total_bytes(), 0);
        assert!(directory.join("keep.txt").exists());
        fs::remove_dir_all(directory).unwrap();
    }

    fn test_directory(line: u32) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("dicomtg-image-cache-{}-{line}", std::process::id()))
    }

    fn write_jpeg(path: &std::path::Path, size: usize, modified_seconds: u64) {
        let mut bytes = vec![0_u8; size];
        bytes[..3].copy_from_slice(&[0xff, 0xd8, 0xff]);
        bytes[size - 2..].copy_from_slice(&[0xff, 0xd9]);
        fs::write(path, bytes).unwrap();
        File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_times(
                FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(modified_seconds)),
            )
            .unwrap();
    }
}
