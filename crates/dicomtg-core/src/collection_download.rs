use std::{error::Error, fmt, io::Read, string::FromUtf8Error, time::Duration};

use ureq::Agent;

const MAX_COLLECTION_CSV_BYTES: usize = 20 * 1024 * 1024;

#[derive(Debug)]
pub enum CollectionDownloadError {
    InvalidUrl,
    TooLarge,
    UnexpectedHtml,
    Http(ureq::Error),
    Io(std::io::Error),
    Utf8(FromUtf8Error),
}

impl fmt::Display for CollectionDownloadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUrl => write!(formatter, "collection URL must be a valid HTTPS URL"),
            Self::TooLarge => write!(formatter, "collection CSV exceeds 20 MiB"),
            Self::UnexpectedHtml => write!(
                formatter,
                "the URL returned a web page instead of a CSV file; check that the link is public"
            ),
            Self::Http(error) => write!(formatter, "collection download failed: {error}"),
            Self::Io(error) => write!(formatter, "collection download I/O error: {error}"),
            Self::Utf8(error) => write!(formatter, "collection CSV is not UTF-8: {error}"),
        }
    }
}

impl Error for CollectionDownloadError {}

impl From<ureq::Error> for CollectionDownloadError {
    fn from(error: ureq::Error) -> Self {
        Self::Http(error)
    }
}

impl From<std::io::Error> for CollectionDownloadError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<FromUtf8Error> for CollectionDownloadError {
    fn from(error: FromUtf8Error) -> Self {
        Self::Utf8(error)
    }
}

pub fn google_drive_source_url(source_url: &str) -> Result<Option<&str>, CollectionDownloadError> {
    let download = collection_download_url(source_url)?;
    Ok(download
        .starts_with("https://drive.usercontent.google.com/")
        .then(|| source_url.trim()))
}

pub fn download_collection_csv(source_url: &str) -> Result<String, CollectionDownloadError> {
    let url = collection_download_url(source_url)?;
    let agent = Agent::new_with_config(
        Agent::config_builder()
            .https_only(true)
            .timeout_global(Some(Duration::from_secs(30)))
            .build(),
    );
    let response = agent
        .get(&url)
        .header(
            "Accept",
            "text/csv,text/plain,application/octet-stream;q=0.9",
        )
        .header("User-Agent", "DicoMTG/0.1 (desktop collection manager)")
        .call()?;
    read_collection_csv(response.into_body().into_reader())
}

fn read_collection_csv(reader: impl Read) -> Result<String, CollectionDownloadError> {
    let mut reader = reader.take(MAX_COLLECTION_CSV_BYTES as u64 + 1);
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    if bytes.len() > MAX_COLLECTION_CSV_BYTES {
        return Err(CollectionDownloadError::TooLarge);
    }
    let csv = String::from_utf8(bytes)?;
    if csv.trim_start().starts_with("<!DOCTYPE html") || csv.trim_start().starts_with("<html") {
        return Err(CollectionDownloadError::UnexpectedHtml);
    }
    Ok(csv)
}

fn collection_download_url(source_url: &str) -> Result<String, CollectionDownloadError> {
    let source_url = source_url.trim();
    let authority_and_path = source_url
        .strip_prefix("https://")
        .ok_or(CollectionDownloadError::InvalidUrl)?;
    let authority_end = authority_and_path
        .find(['/', '?', '#'])
        .unwrap_or(authority_and_path.len());
    let authority = &authority_and_path[..authority_end];
    if authority.is_empty() || authority.contains('@') {
        return Err(CollectionDownloadError::InvalidUrl);
    }
    let host = authority
        .split(':')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if host.is_empty() || host == "localhost" || host == "127.0.0.1" {
        return Err(CollectionDownloadError::InvalidUrl);
    }
    if host != "drive.google.com" && host != "www.drive.google.com" {
        return Ok(source_url.to_owned());
    }

    let id = google_drive_id(source_url).ok_or(CollectionDownloadError::InvalidUrl)?;
    if !id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(CollectionDownloadError::InvalidUrl);
    }
    Ok(format!(
        "https://drive.usercontent.google.com/download?id={id}&export=download&confirm=t"
    ))
}

fn google_drive_id(url: &str) -> Option<&str> {
    if let Some(rest) = url.split_once("/file/d/").map(|(_, rest)| rest) {
        return rest
            .split(['/', '?', '#'])
            .next()
            .filter(|id| !id.is_empty());
    }
    let query = url.split_once('?')?.1.split('#').next()?;
    query.split('&').find_map(|parameter| {
        let (name, value) = parameter.split_once('=')?;
        (name == "id" && !value.is_empty()).then_some(value)
    })
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{
        CollectionDownloadError, MAX_COLLECTION_CSV_BYTES, collection_download_url,
        google_drive_source_url, read_collection_csv,
    };

    #[test]
    fn col_010_converts_public_google_drive_links_to_direct_downloads() {
        assert_eq!(
            collection_download_url("https://drive.google.com/file/d/1AbC_def-23/view?usp=sharing")
                .unwrap(),
            "https://drive.usercontent.google.com/download?id=1AbC_def-23&export=download&confirm=t"
        );
        assert_eq!(
            collection_download_url("https://drive.google.com/open?id=1AbC_def-23").unwrap(),
            "https://drive.usercontent.google.com/download?id=1AbC_def-23&export=download&confirm=t"
        );
    }

    #[test]
    fn remembers_the_original_link_only_for_google_drive() {
        let link = "https://drive.google.com/file/d/1AbC_def/view?usp=sharing";
        assert_eq!(google_drive_source_url(link).unwrap(), Some(link));
        assert_eq!(
            google_drive_source_url("https://example.test/cards.csv").unwrap(),
            None
        );
        assert_eq!(
            google_drive_source_url("https://drive.google.com.example.test/file/d/1AbC_def/view")
                .unwrap(),
            None
        );
        assert!(google_drive_source_url("https://drive.google.com/unsupported").is_err());
    }

    #[test]
    fn col_010_accepts_https_csv_urls_and_rejects_other_schemes() {
        assert_eq!(
            collection_download_url("https://example.test/collection.csv").unwrap(),
            "https://example.test/collection.csv"
        );
        assert!(matches!(
            collection_download_url("http://example.test/collection.csv"),
            Err(CollectionDownloadError::InvalidUrl)
        ));
        assert!(matches!(
            collection_download_url("file:///collection.csv"),
            Err(CollectionDownloadError::InvalidUrl)
        ));
    }

    #[test]
    fn col_010_rejects_html_and_oversized_downloads() {
        assert!(matches!(
            read_collection_csv(Cursor::new("<html>Login</html>")),
            Err(CollectionDownloadError::UnexpectedHtml)
        ));
        assert!(matches!(
            read_collection_csv(Cursor::new(vec![b'x'; MAX_COLLECTION_CSV_BYTES + 1])),
            Err(CollectionDownloadError::TooLarge)
        ));
    }
}
