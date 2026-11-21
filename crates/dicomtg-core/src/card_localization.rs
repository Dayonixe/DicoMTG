use std::{error::Error, fmt, io::Read, time::Duration};

use serde::Deserialize;
use ureq::Agent;

use crate::card_image::encode_path_segment;

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardLocalization {
    set_name: String,
    french_text: Option<String>,
}

impl CardLocalization {
    #[must_use]
    pub fn set_name(&self) -> &str {
        &self.set_name
    }

    #[must_use]
    pub fn french_text(&self) -> Option<&str> {
        self.french_text.as_deref()
    }
}

#[derive(Debug)]
pub enum CardLocalizationError {
    InvalidRequest(&'static str),
    Unavailable,
    TooLarge,
    Http(ureq::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl fmt::Display for CardLocalizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => write!(formatter, "invalid card request: {message}"),
            Self::Unavailable => write!(formatter, "card localization is unavailable"),
            Self::TooLarge => write!(formatter, "card localization response exceeds 1 MiB"),
            Self::Http(error) => write!(formatter, "card localization download failed: {error}"),
            Self::Io(error) => write!(formatter, "card localization I/O error: {error}"),
            Self::Json(error) => write!(formatter, "invalid card localization response: {error}"),
        }
    }
}

impl Error for CardLocalizationError {}

impl From<ureq::Error> for CardLocalizationError {
    fn from(error: ureq::Error) -> Self {
        Self::Http(error)
    }
}

impl From<std::io::Error> for CardLocalizationError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for CardLocalizationError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[derive(Debug, Deserialize)]
struct ScryfallCard {
    set_name: String,
    printed_text: Option<String>,
    card_faces: Option<Vec<ScryfallFace>>,
}

#[derive(Debug, Deserialize)]
struct ScryfallFace {
    printed_name: Option<String>,
    printed_text: Option<String>,
}

impl ScryfallCard {
    fn french_text(&self) -> Option<String> {
        self.printed_text
            .as_deref()
            .filter(|text| !text.trim().is_empty())
            .map(str::to_owned)
            .or_else(|| {
                let faces = self
                    .card_faces
                    .as_ref()?
                    .iter()
                    .filter_map(|face| {
                        let text = face.printed_text.as_deref()?.trim();
                        if text.is_empty() {
                            return None;
                        }
                        Some(match face.printed_name.as_deref().map(str::trim) {
                            Some(name) if !name.is_empty() => format!("{name}\n{text}"),
                            _ => text.to_owned(),
                        })
                    })
                    .collect::<Vec<_>>();
                (!faces.is_empty()).then(|| faces.join("\n\n"))
            })
    }
}

pub fn load_card_localization(
    set_code: &str,
    collector_number: &str,
) -> Result<CardLocalization, CardLocalizationError> {
    let agent = Agent::new_with_config(
        Agent::config_builder()
            .https_only(true)
            .timeout_global(Some(Duration::from_secs(15)))
            .build(),
    );
    load_card_localization_with(set_code, collector_number, |url| download_json(&agent, url))
}

fn load_card_localization_with(
    set_code: &str,
    collector_number: &str,
    mut download: impl FnMut(&str) -> Result<Option<Vec<u8>>, CardLocalizationError>,
) -> Result<CardLocalization, CardLocalizationError> {
    let set_code = set_code.trim();
    let collector_number = collector_number.trim();
    if set_code.is_empty() || set_code.len() > 32 {
        return Err(CardLocalizationError::InvalidRequest("invalid set code"));
    }
    if collector_number.is_empty() || collector_number.len() > 128 {
        return Err(CardLocalizationError::InvalidRequest(
            "invalid collector number",
        ));
    }

    let card_url = |language: &str| {
        format!(
            "https://api.scryfall.com/cards/{}/{}/{language}",
            encode_path_segment(&set_code.to_ascii_lowercase()),
            encode_path_segment(collector_number),
        )
    };
    if let Some(bytes) = download(&card_url("fr"))? {
        let card: ScryfallCard = serde_json::from_slice(&bytes)?;
        let french_text = card.french_text();
        return Ok(CardLocalization {
            set_name: card.set_name,
            french_text,
        });
    }

    let bytes = download(&card_url("en"))?.ok_or(CardLocalizationError::Unavailable)?;
    let card: ScryfallCard = serde_json::from_slice(&bytes)?;
    Ok(CardLocalization {
        set_name: card.set_name,
        french_text: None,
    })
}

fn download_json(agent: &Agent, url: &str) -> Result<Option<Vec<u8>>, CardLocalizationError> {
    let response = match agent
        .get(url)
        .header("Accept", "application/json")
        .header("User-Agent", "DicoMTG/0.1 (desktop collection manager)")
        .call()
    {
        Ok(response) => response,
        Err(ureq::Error::StatusCode(404)) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut reader = response
        .into_body()
        .into_reader()
        .take(MAX_RESPONSE_BYTES as u64 + 1);
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(CardLocalizationError::TooLarge);
    }
    Ok(Some(bytes))
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::{CardLocalizationError, load_card_localization_with};

    #[test]
    fn ui_012_returns_official_french_text_and_set_name() {
        let localization = load_card_localization_with("c21", "263", |url| {
            assert!(url.ends_with("/c21/263/fr"));
            Ok(Some(
                br#"{"set_name":"Commander 2021","printed_text":"Ajoutez deux manas.","card_faces":null}"#
                    .to_vec(),
            ))
        })
        .unwrap();

        assert_eq!(localization.set_name(), "Commander 2021");
        assert_eq!(localization.french_text(), Some("Ajoutez deux manas."));
    }

    #[test]
    fn ui_012_uses_english_only_for_the_set_name_when_french_is_missing() {
        let calls = Cell::new(0);
        let localization = load_card_localization_with("tst", "1", |url| {
            calls.set(calls.get() + 1);
            if url.ends_with("/fr") {
                Ok(None)
            } else {
                Ok(Some(
                    br#"{"set_name":"Test Set","printed_text":"English text","card_faces":null}"#
                        .to_vec(),
                ))
            }
        })
        .unwrap();

        assert_eq!(calls.get(), 2);
        assert_eq!(localization.set_name(), "Test Set");
        assert_eq!(localization.french_text(), None);
    }

    #[test]
    fn ui_012_rejects_invalid_keys_before_network_access() {
        let error = load_card_localization_with("", "1", |_| {
            panic!("invalid input must not reach the network")
        })
        .unwrap_err();

        assert!(matches!(error, CardLocalizationError::InvalidRequest(_)));
    }
}
