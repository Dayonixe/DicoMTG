use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use sha2::{Digest, Sha256};
use ureq::Agent;

const MAX_CHECKSUM_BYTES: u64 = 1_024;
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

pub(crate) enum UpdateFileError {
    Io(io::Error),
    Http(ureq::Error),
    InvalidChecksum { value: String },
    ChecksumMismatch { expected: String, actual: String },
    DownloadTooLarge { limit: u64 },
}

impl From<io::Error> for UpdateFileError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<ureq::Error> for UpdateFileError {
    fn from(error: ureq::Error) -> Self {
        Self::Http(error)
    }
}

pub(crate) fn http_agent() -> Agent {
    Agent::new_with_config(
        Agent::config_builder()
            .https_only(true)
            .timeout_global(Some(Duration::from_secs(5 * 60)))
            .build(),
    )
}

pub(crate) fn download_checksum(agent: &Agent, url: &str) -> Result<String, UpdateFileError> {
    let response = agent.get(url).call()?;
    let mut reader = response
        .into_body()
        .into_reader()
        .take(MAX_CHECKSUM_BYTES + 1);
    let mut checksum = String::new();
    reader.read_to_string(&mut checksum)?;
    if reader.limit() == 0 {
        return Err(UpdateFileError::DownloadTooLarge {
            limit: MAX_CHECKSUM_BYTES,
        });
    }
    parse_checksum(&checksum)
}

pub(crate) fn download_archive(
    agent: &Agent,
    url: &str,
    target: &mut File,
    limit: u64,
) -> Result<(), UpdateFileError> {
    let response = agent.get(url).call()?;
    copy_archive(response.into_body().into_reader(), target, limit)
}

pub(crate) fn verify_checksum(
    path: &Path,
    expected_checksum: &str,
) -> Result<u64, UpdateFileError> {
    let expected = parse_checksum(expected_checksum)?;
    let actual = sha256_file(path)?;
    if actual != expected {
        return Err(UpdateFileError::ChecksumMismatch { expected, actual });
    }
    Ok(fs::metadata(path)?.len())
}

fn copy_archive(
    reader: impl Read,
    mut target: impl Write,
    limit: u64,
) -> Result<(), UpdateFileError> {
    let mut reader = reader.take(limit + 1);
    io::copy(&mut reader, &mut target)?;
    if reader.limit() == 0 {
        Err(UpdateFileError::DownloadTooLarge { limit })
    } else {
        Ok(())
    }
}

fn parse_checksum(value: &str) -> Result<String, UpdateFileError> {
    let checksum = value.split_whitespace().next().unwrap_or_default();
    if checksum.len() == 64 && checksum.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(checksum.to_ascii_lowercase())
    } else {
        Err(UpdateFileError::InvalidChecksum {
            value: value.trim().to_owned(),
        })
    }
}

fn sha256_file(path: &Path) -> Result<String, io::Error> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub(crate) struct TemporaryArchive {
    path: PathBuf,
}

impl TemporaryArchive {
    pub(crate) fn create(directory: &Path, name: &str) -> Result<(Self, File), io::Error> {
        loop {
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let path = directory.join(format!("{name}.{}.{}.part", std::process::id(), id));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok((Self { path }, file)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TemporaryArchive {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{UpdateFileError, copy_archive};

    #[test]
    fn compressed_download_over_the_limit_is_rejected() {
        let mut target = Vec::new();

        let error = copy_archive(Cursor::new(b"four"), &mut target, 3)
            .expect_err("an oversized response must be rejected");

        assert!(matches!(
            error,
            UpdateFileError::DownloadTooLarge { limit: 3 }
        ));
    }
}
