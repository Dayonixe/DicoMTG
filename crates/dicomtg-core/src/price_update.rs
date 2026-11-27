use std::{
    error::Error,
    fmt,
    fs::{self, File},
    io::{self, BufRead, BufReader, Read, Write},
    path::Path,
    time::Duration,
};

use flate2::read::GzDecoder;
use ureq::Agent;

use crate::{
    CollectionStore, PriceImportError, StorageError, import_cardmarket_price_guide,
    import_cardmarket_price_guide_with_products_and_sets,
    mtgjson_update::{TemporaryArchive, UpdateFileError, download_archive, http_agent},
    price::import_set_release_dates,
};

const PRICES_URL: &str =
    "https://downloads.s3.cardmarket.com/productCatalog/priceGuide/price_guide_1.json";
const PRODUCTS_URL: &str =
    "https://downloads.s3.cardmarket.com/productCatalog/productList/products_singles_1.json";
const SETS_URL: &str = "https://mtgjson.com/api/v5/SetList.json.gz";
const MAX_DOWNLOAD_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceUpdateSummary {
    price_count: usize,
    downloaded_bytes: u64,
}

impl PriceUpdateSummary {
    #[must_use]
    pub fn price_count(&self) -> usize {
        self.price_count
    }

    #[must_use]
    pub fn downloaded_bytes(&self) -> u64 {
        self.downloaded_bytes
    }
}

#[derive(Debug)]
pub enum PriceUpdateError {
    Io(io::Error),
    Http(ureq::Error),
    Price(PriceImportError),
    Storage(StorageError),
    InvalidChecksum { value: String },
    ChecksumMismatch { expected: String, actual: String },
    DownloadTooLarge { limit: u64 },
}

impl fmt::Display for PriceUpdateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "price I/O error: {error}"),
            Self::Http(error) => write!(formatter, "price HTTP error: {error}"),
            Self::Price(error) => write!(formatter, "price validation failed: {error}"),
            Self::Storage(error) => write!(formatter, "price storage failed: {error}"),
            Self::InvalidChecksum { value } => write!(formatter, "invalid SHA-256: {value}"),
            Self::ChecksumMismatch { expected, actual } => write!(
                formatter,
                "price SHA-256 mismatch: expected {expected}, got {actual}"
            ),
            Self::DownloadTooLarge { limit } => {
                write!(formatter, "Cardmarket data exceeds {limit} bytes")
            }
        }
    }
}

impl Error for PriceUpdateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Http(error) => Some(error),
            Self::Price(error) => Some(error),
            Self::Storage(error) => Some(error),
            Self::InvalidChecksum { .. }
            | Self::ChecksumMismatch { .. }
            | Self::DownloadTooLarge { .. } => None,
        }
    }
}

impl From<io::Error> for PriceUpdateError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<PriceImportError> for PriceUpdateError {
    fn from(error: PriceImportError) -> Self {
        Self::Price(error)
    }
}

impl From<StorageError> for PriceUpdateError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

impl From<UpdateFileError> for PriceUpdateError {
    fn from(error: UpdateFileError) -> Self {
        match error {
            UpdateFileError::Io(error) => Self::Io(error),
            UpdateFileError::Http(error) => Self::Http(error),
            UpdateFileError::DownloadTooLarge { limit } => Self::DownloadTooLarge { limit },
            UpdateFileError::InvalidChecksum { value } => Self::InvalidChecksum { value },
            UpdateFileError::ChecksumMismatch { expected, actual } => {
                Self::ChecksumMismatch { expected, actual }
            }
        }
    }
}

pub fn update_prices_from_cardmarket(
    store: &mut CollectionStore,
    temporary_directory: impl AsRef<Path>,
) -> Result<PriceUpdateSummary, PriceUpdateError> {
    let temporary_directory = temporary_directory.as_ref();
    fs::create_dir_all(temporary_directory)?;
    let agent = http_agent();
    let (temporary, mut file) =
        TemporaryArchive::create(temporary_directory, "cardmarket-price-guide.json")?;
    download_archive(&agent, PRICES_URL, &mut file, MAX_DOWNLOAD_BYTES)?;
    file.flush()?;
    drop(file);
    let (products, mut products_file) =
        TemporaryArchive::create(temporary_directory, "cardmarket-products.json")?;
    download_archive(&agent, PRODUCTS_URL, &mut products_file, MAX_DOWNLOAD_BYTES)?;
    products_file.flush()?;
    drop(products_file);
    let (sets, mut sets_file) =
        TemporaryArchive::create(temporary_directory, "mtgjson-set-list.json.gz")?;
    download_archive(&agent, SETS_URL, &mut sets_file, MAX_DOWNLOAD_BYTES)?;
    sets_file.flush()?;
    drop(sets_file);
    install_cardmarket_data(store, temporary.path(), products.path(), sets.path())
}

pub fn initialize_set_release_dates(
    store: &mut CollectionStore,
    temporary_directory: impl AsRef<Path>,
) -> Result<bool, PriceUpdateError> {
    if store.has_set_release_dates()? {
        return Ok(false);
    }
    let temporary_directory = temporary_directory.as_ref();
    fs::create_dir_all(temporary_directory)?;
    let agent = Agent::new_with_config(
        Agent::config_builder()
            .https_only(true)
            .timeout_global(Some(Duration::from_secs(15)))
            .build(),
    );
    let (sets, mut file) =
        TemporaryArchive::create(temporary_directory, "set-release-dates.json.gz")?;
    download_archive(&agent, SETS_URL, &mut file, MAX_DOWNLOAD_BYTES)?;
    file.flush()?;
    drop(file);
    let dates = import_set_release_dates(GzDecoder::new(File::open(sets.path())?))?;
    if dates.is_empty() {
        return Ok(false);
    }
    store.save_set_release_dates(&dates)?;
    Ok(true)
}

pub fn install_cardmarket_price_guide(
    store: &mut CollectionStore,
    guide_path: impl AsRef<Path>,
) -> Result<PriceUpdateSummary, PriceUpdateError> {
    let guide_path = guide_path.as_ref();
    let downloaded_bytes = fs::metadata(guide_path)?.len();
    validate_download_size(downloaded_bytes)?;
    let catalog = store.load_catalog()?;
    let prices = import_cardmarket_price_guide(File::open(guide_path)?, &catalog)?;
    let price_count = prices.prices().len();
    store.replace_prices(&prices)?;

    Ok(PriceUpdateSummary {
        price_count,
        downloaded_bytes,
    })
}

pub fn install_cardmarket_data(
    store: &mut CollectionStore,
    guide_path: impl AsRef<Path>,
    products_path: impl AsRef<Path>,
    sets_path: impl AsRef<Path>,
) -> Result<PriceUpdateSummary, PriceUpdateError> {
    let guide_path = guide_path.as_ref();
    let products_path = products_path.as_ref();
    let sets_path = sets_path.as_ref();
    let guide_bytes = fs::metadata(guide_path)?.len();
    let products_bytes = fs::metadata(products_path)?.len();
    let sets_bytes = fs::metadata(sets_path)?.len();
    validate_download_size(guide_bytes)?;
    validate_download_size(products_bytes)?;
    validate_download_size(sets_bytes)?;
    let mut sets = BufReader::new(File::open(sets_path)?);
    let sets: Box<dyn Read> = if sets.fill_buf()?.starts_with(&[0x1f, 0x8b]) {
        Box::new(GzDecoder::new(sets))
    } else {
        Box::new(sets)
    };
    let mut sets = sets.take(MAX_DOWNLOAD_BYTES + 1);
    let catalog = store.load_catalog()?;
    let prices = import_cardmarket_price_guide_with_products_and_sets(
        File::open(guide_path)?,
        File::open(products_path)?,
        &mut sets,
        &catalog,
    )?;
    validate_download_size(MAX_DOWNLOAD_BYTES + 1 - sets.limit())?;
    let price_count = prices.prices().len();
    store.replace_prices(&prices)?;

    Ok(PriceUpdateSummary {
        price_count,
        downloaded_bytes: guide_bytes + products_bytes + sets_bytes,
    })
}

fn validate_download_size(bytes: u64) -> Result<(), PriceUpdateError> {
    if bytes > MAX_DOWNLOAD_BYTES {
        Err(PriceUpdateError::DownloadTooLarge {
            limit: MAX_DOWNLOAD_BYTES,
        })
    } else {
        Ok(())
    }
}
