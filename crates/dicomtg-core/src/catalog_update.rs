use std::{
    error::Error,
    fmt,
    fs::{self, File},
    io::{self, Read, Write},
    path::Path,
};

use flate2::read::GzDecoder;

use crate::{
    CatalogImport, CatalogImportError, CollectionStore, StorageError, import_mtgjson_cards_csv,
    import_mtgjson_tokens_csv,
    mtgjson_update::{
        TemporaryArchive, UpdateFileError, download_archive, download_checksum, http_agent,
        verify_checksum,
    },
};

const CATALOG_URL: &str = "https://mtgjson.com/api/v5/csv/cards.csv.gz";
const CHECKSUM_URL: &str = "https://mtgjson.com/api/v5/csv/cards.csv.gz.sha256";
const TOKENS_URL: &str = "https://mtgjson.com/api/v5/csv/tokens.csv.gz";
const TOKENS_CHECKSUM_URL: &str = "https://mtgjson.com/api/v5/csv/tokens.csv.gz.sha256";
const CARD_IDENTIFIERS_URL: &str = "https://mtgjson.com/api/v5/csv/cardIdentifiers.csv.gz";
const CARD_IDENTIFIERS_CHECKSUM_URL: &str =
    "https://mtgjson.com/api/v5/csv/cardIdentifiers.csv.gz.sha256";
const TOKEN_IDENTIFIERS_URL: &str = "https://mtgjson.com/api/v5/csv/tokenIdentifiers.csv.gz";
const TOKEN_IDENTIFIERS_CHECKSUM_URL: &str =
    "https://mtgjson.com/api/v5/csv/tokenIdentifiers.csv.gz.sha256";
const MAX_COMPRESSED_BYTES: u64 = 128 * 1024 * 1024;
const MAX_DECOMPRESSED_BYTES: u64 = 512 * 1024 * 1024;
const MAX_TOKENS_COMPRESSED_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TOKENS_DECOMPRESSED_BYTES: u64 = 64 * 1024 * 1024;
const MAX_IDENTIFIERS_COMPRESSED_BYTES: u64 = 64 * 1024 * 1024;
const MAX_IDENTIFIERS_DECOMPRESSED_BYTES: u64 = 128 * 1024 * 1024;
const MAX_TOKEN_IDENTIFIERS_COMPRESSED_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TOKEN_IDENTIFIERS_DECOMPRESSED_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogUpdateSummary {
    card_count: usize,
    downloaded_bytes: u64,
}

impl CatalogUpdateSummary {
    #[must_use]
    pub fn card_count(&self) -> usize {
        self.card_count
    }

    #[must_use]
    pub fn downloaded_bytes(&self) -> u64 {
        self.downloaded_bytes
    }
}

#[derive(Debug)]
pub enum CatalogUpdateError {
    Io(io::Error),
    Http(ureq::Error),
    Catalog(CatalogImportError),
    Storage(StorageError),
    InvalidChecksum { value: String },
    ChecksumMismatch { expected: String, actual: String },
    DownloadTooLarge { limit: u64 },
    DecompressedTooLarge { limit: u64 },
}

impl fmt::Display for CatalogUpdateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "catalog I/O error: {error}"),
            Self::Http(error) => write!(formatter, "catalog HTTP error: {error}"),
            Self::Catalog(error) => write!(formatter, "catalog validation failed: {error}"),
            Self::Storage(error) => write!(formatter, "catalog storage failed: {error}"),
            Self::InvalidChecksum { value } => write!(formatter, "invalid SHA-256: {value}"),
            Self::ChecksumMismatch { expected, actual } => write!(
                formatter,
                "catalog SHA-256 mismatch: expected {expected}, got {actual}"
            ),
            Self::DownloadTooLarge { limit } => {
                write!(formatter, "compressed catalog exceeds {limit} bytes")
            }
            Self::DecompressedTooLarge { limit } => {
                write!(formatter, "decompressed catalog exceeds {limit} bytes")
            }
        }
    }
}

impl Error for CatalogUpdateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Http(error) => Some(error),
            Self::Catalog(error) => Some(error),
            Self::Storage(error) => Some(error),
            Self::InvalidChecksum { .. }
            | Self::ChecksumMismatch { .. }
            | Self::DownloadTooLarge { .. }
            | Self::DecompressedTooLarge { .. } => None,
        }
    }
}

impl From<io::Error> for CatalogUpdateError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<ureq::Error> for CatalogUpdateError {
    fn from(error: ureq::Error) -> Self {
        Self::Http(error)
    }
}

impl From<CatalogImportError> for CatalogUpdateError {
    fn from(error: CatalogImportError) -> Self {
        Self::Catalog(error)
    }
}

impl From<StorageError> for CatalogUpdateError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

impl From<UpdateFileError> for CatalogUpdateError {
    fn from(error: UpdateFileError) -> Self {
        match error {
            UpdateFileError::Io(error) => Self::Io(error),
            UpdateFileError::Http(error) => Self::Http(error),
            UpdateFileError::InvalidChecksum { value } => Self::InvalidChecksum { value },
            UpdateFileError::ChecksumMismatch { expected, actual } => {
                Self::ChecksumMismatch { expected, actual }
            }
            UpdateFileError::DownloadTooLarge { limit } => Self::DownloadTooLarge { limit },
        }
    }
}

pub fn update_catalog_from_mtgjson(
    store: &mut CollectionStore,
    temporary_directory: impl AsRef<Path>,
) -> Result<CatalogUpdateSummary, CatalogUpdateError> {
    let temporary_directory = temporary_directory.as_ref();
    fs::create_dir_all(temporary_directory)?;
    let agent = http_agent();
    let cards_checksum = download_checksum(&agent, CHECKSUM_URL)?;
    let tokens_checksum = download_checksum(&agent, TOKENS_CHECKSUM_URL)?;
    let card_identifiers_checksum = download_checksum(&agent, CARD_IDENTIFIERS_CHECKSUM_URL)?;
    let token_identifiers_checksum = download_checksum(&agent, TOKEN_IDENTIFIERS_CHECKSUM_URL)?;
    let (cards, mut cards_file) = TemporaryArchive::create(temporary_directory, "cards.csv.gz")?;
    download_archive(&agent, CATALOG_URL, &mut cards_file, MAX_COMPRESSED_BYTES)?;
    cards_file.flush()?;
    drop(cards_file);
    let (tokens, mut tokens_file) = TemporaryArchive::create(temporary_directory, "tokens.csv.gz")?;
    download_archive(
        &agent,
        TOKENS_URL,
        &mut tokens_file,
        MAX_TOKENS_COMPRESSED_BYTES,
    )?;
    tokens_file.flush()?;
    drop(tokens_file);
    let (card_identifiers, mut card_identifiers_file) =
        TemporaryArchive::create(temporary_directory, "cardIdentifiers.csv.gz")?;
    download_archive(
        &agent,
        CARD_IDENTIFIERS_URL,
        &mut card_identifiers_file,
        MAX_IDENTIFIERS_COMPRESSED_BYTES,
    )?;
    card_identifiers_file.flush()?;
    drop(card_identifiers_file);
    let (token_identifiers, mut token_identifiers_file) =
        TemporaryArchive::create(temporary_directory, "tokenIdentifiers.csv.gz")?;
    download_archive(
        &agent,
        TOKEN_IDENTIFIERS_URL,
        &mut token_identifiers_file,
        MAX_TOKEN_IDENTIFIERS_COMPRESSED_BYTES,
    )?;
    token_identifiers_file.flush()?;
    drop(token_identifiers_file);
    install_catalog_archives_with_identifiers(
        store,
        cards.path(),
        &cards_checksum,
        tokens.path(),
        &tokens_checksum,
        card_identifiers.path(),
        &card_identifiers_checksum,
        token_identifiers.path(),
        &token_identifiers_checksum,
    )
}

pub fn install_catalog_archive(
    store: &mut CollectionStore,
    archive_path: impl AsRef<Path>,
    expected_checksum: &str,
) -> Result<CatalogUpdateSummary, CatalogUpdateError> {
    let (catalog, downloaded_bytes) = read_cards_archive(archive_path, expected_checksum)?;
    let card_count = catalog.cards().len();
    store.replace_catalog(&catalog)?;

    Ok(CatalogUpdateSummary {
        card_count,
        downloaded_bytes,
    })
}

pub fn install_catalog_archives(
    store: &mut CollectionStore,
    cards_path: impl AsRef<Path>,
    cards_checksum: &str,
    tokens_path: impl AsRef<Path>,
    tokens_checksum: &str,
) -> Result<CatalogUpdateSummary, CatalogUpdateError> {
    let (mut catalog, cards_bytes) = read_cards_archive(cards_path, cards_checksum)?;
    let (tokens, tokens_bytes) = read_tokens_archive(tokens_path, tokens_checksum)?;
    catalog.extend(tokens);
    let card_count = catalog.cards().len();
    store.replace_catalog(&catalog)?;

    Ok(CatalogUpdateSummary {
        card_count,
        downloaded_bytes: cards_bytes + tokens_bytes,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn install_catalog_archives_with_identifiers(
    store: &mut CollectionStore,
    cards_path: impl AsRef<Path>,
    cards_checksum: &str,
    tokens_path: impl AsRef<Path>,
    tokens_checksum: &str,
    card_identifiers_path: impl AsRef<Path>,
    card_identifiers_checksum: &str,
    token_identifiers_path: impl AsRef<Path>,
    token_identifiers_checksum: &str,
) -> Result<CatalogUpdateSummary, CatalogUpdateError> {
    let (mut catalog, cards_bytes) = read_cards_archive(cards_path, cards_checksum)?;
    let (mut tokens, tokens_bytes) = read_tokens_archive(tokens_path, tokens_checksum)?;
    let card_identifiers_bytes = attach_identifiers_archive(
        &mut catalog,
        card_identifiers_path,
        card_identifiers_checksum,
        MAX_IDENTIFIERS_DECOMPRESSED_BYTES,
    )?;
    let token_identifiers_bytes = attach_identifiers_archive(
        &mut tokens,
        token_identifiers_path,
        token_identifiers_checksum,
        MAX_TOKEN_IDENTIFIERS_DECOMPRESSED_BYTES,
    )?;
    catalog.extend(tokens);
    let card_count = catalog.cards().len();
    store.replace_catalog(&catalog)?;

    Ok(CatalogUpdateSummary {
        card_count,
        downloaded_bytes: cards_bytes
            + tokens_bytes
            + card_identifiers_bytes
            + token_identifiers_bytes,
    })
}

fn read_cards_archive(
    archive_path: impl AsRef<Path>,
    expected_checksum: &str,
) -> Result<(CatalogImport, u64), CatalogUpdateError> {
    let archive_path = archive_path.as_ref();
    let downloaded_bytes = verify_checksum(archive_path, expected_checksum)?;
    let archive = File::open(archive_path)?;
    let decoder = GzDecoder::new(archive);
    let mut limited = decoder.take(MAX_DECOMPRESSED_BYTES + 1);
    let catalog = import_mtgjson_cards_csv(&mut limited)?;
    if limited.limit() == 0 {
        return Err(CatalogUpdateError::DecompressedTooLarge {
            limit: MAX_DECOMPRESSED_BYTES,
        });
    }
    Ok((catalog, downloaded_bytes))
}

fn read_tokens_archive(
    archive_path: impl AsRef<Path>,
    expected_checksum: &str,
) -> Result<(CatalogImport, u64), CatalogUpdateError> {
    let archive_path = archive_path.as_ref();
    let downloaded_bytes = verify_checksum(archive_path, expected_checksum)?;
    let archive = File::open(archive_path)?;
    let decoder = GzDecoder::new(archive);
    let mut limited = decoder.take(MAX_TOKENS_DECOMPRESSED_BYTES + 1);
    let catalog = import_mtgjson_tokens_csv(&mut limited)?;
    if limited.limit() == 0 {
        return Err(CatalogUpdateError::DecompressedTooLarge {
            limit: MAX_TOKENS_DECOMPRESSED_BYTES,
        });
    }
    Ok((catalog, downloaded_bytes))
}

fn attach_identifiers_archive(
    catalog: &mut CatalogImport,
    archive_path: impl AsRef<Path>,
    expected_checksum: &str,
    decompressed_limit: u64,
) -> Result<u64, CatalogUpdateError> {
    let archive_path = archive_path.as_ref();
    let downloaded_bytes = verify_checksum(archive_path, expected_checksum)?;
    let archive = File::open(archive_path)?;
    let decoder = GzDecoder::new(archive);
    let mut limited = decoder.take(decompressed_limit + 1);
    catalog.attach_scryfall_ids(&mut limited)?;
    if limited.limit() == 0 {
        return Err(CatalogUpdateError::DecompressedTooLarge {
            limit: decompressed_limit,
        });
    }
    Ok(downloaded_bytes)
}
