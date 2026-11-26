use std::{error::Error, fmt, io::Read, sync::LazyLock, time::Duration};

use serde::Deserialize;
use ureq::Agent;

use crate::card_image::{encode_path_segment, is_scryfall_id};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
static SCRYFALL_AGENT: LazyLock<Agent> = LazyLock::new(|| {
    Agent::new_with_config(
        Agent::config_builder()
            .https_only(true)
            .timeout_global(Some(Duration::from_secs(15)))
            .build(),
    )
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardLocalization {
    set_name: String,
    french_text: Option<String>,
    fallback_text: Option<String>,
    oracle_id: Option<String>,
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

    #[must_use]
    pub fn fallback_text(&self) -> Option<&str> {
        self.fallback_text.as_deref()
    }

    #[must_use]
    pub fn oracle_id(&self) -> Option<&str> {
        self.oracle_id.as_deref()
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
    oracle_id: Option<String>,
    lang: Option<String>,
    oracle_text: Option<String>,
    printed_text: Option<String>,
    card_faces: Option<Vec<ScryfallFace>>,
}

#[derive(Debug, Deserialize)]
struct ScryfallFace {
    printed_name: Option<String>,
    oracle_text: Option<String>,
    printed_text: Option<String>,
}

#[derive(Deserialize)]
struct ScryfallCardList {
    data: Vec<ScryfallCard>,
}

impl ScryfallCard {
    fn printed_rules(&self, require_french: bool) -> Option<String> {
        let oracle_text = self.oracle_text.as_deref().filter(|_| require_french);
        if let Some(text) = usable_printed_text(self.printed_text.as_deref(), oracle_text) {
            return Some(text.to_owned());
        }

        let mut faces = Vec::new();
        for face in self.card_faces.as_ref()? {
            let oracle_text = face.oracle_text.as_deref().filter(|_| require_french);
            if let Some(text) = usable_printed_text(face.printed_text.as_deref(), oracle_text) {
                faces.push(match face.printed_name.as_deref().map(str::trim) {
                    Some(name) if !name.is_empty() => format!("{name}\n{text}"),
                    _ => text.to_owned(),
                });
            } else if face
                .oracle_text
                .as_deref()
                .is_some_and(|text| !text.trim().is_empty())
            {
                return None;
            }
        }
        (!faces.is_empty()).then(|| faces.join("\n\n"))
    }
}

fn usable_printed_text<'a>(printed: Option<&'a str>, oracle: Option<&str>) -> Option<&'a str> {
    let text = printed?.trim();
    if text.is_empty()
        || oracle.is_some_and(|oracle| oracle.split_whitespace().eq(text.split_whitespace()))
    {
        return None;
    }
    Some(text)
}

pub fn load_card_localization(
    set_code: &str,
    collector_number: &str,
) -> Result<CardLocalization, CardLocalizationError> {
    load_card_localization_with(set_code, collector_number, |url| {
        download_json(&SCRYFALL_AGENT, url)
    })
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
    let (card, is_french) = if let Some(bytes) = download(&card_url("fr"))? {
        let card: ScryfallCard = serde_json::from_slice(&bytes)?;
        let is_french = card.lang.as_deref() == Some("fr");
        (card, is_french)
    } else {
        let bytes = download(&card_url("en"))?.ok_or(CardLocalizationError::Unavailable)?;
        (serde_json::from_slice(&bytes)?, false)
    };
    let french_text = is_french.then(|| card.printed_rules(true)).flatten();
    let fallback_text = if is_french && french_text.is_none() {
        card.printed_rules(false)
    } else {
        None
    };
    Ok(CardLocalization {
        set_name: card.set_name,
        french_text,
        fallback_text,
        oracle_id: card.oracle_id,
    })
}

pub fn find_card_french_text(oracle_id: &str) -> Result<Option<String>, CardLocalizationError> {
    find_card_french_text_with(oracle_id, |url| download_json(&SCRYFALL_AGENT, url))
}

fn find_card_french_text_with(
    oracle_id: &str,
    download: impl FnOnce(&str) -> Result<Option<Vec<u8>>, CardLocalizationError>,
) -> Result<Option<String>, CardLocalizationError> {
    if !is_scryfall_id(oracle_id) {
        return Err(CardLocalizationError::InvalidRequest(
            "invalid Oracle identifier",
        ));
    }

    let query = encode_path_segment(&format!("oracleid:{oracle_id} lang:fr"));
    let url = format!("https://api.scryfall.com/cards/search?q={query}&unique=prints");
    let Some(bytes) = download(&url)? else {
        return Ok(None);
    };
    let printings: ScryfallCardList = serde_json::from_slice(&bytes)?;
    Ok(printings
        .data
        .into_iter()
        .filter(|printing| {
            printing.lang.as_deref() == Some("fr")
                && printing.oracle_id.as_deref() == Some(oracle_id)
        })
        .find_map(|printing| printing.printed_rules(true)))
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

    use super::{CardLocalizationError, find_card_french_text_with, load_card_localization_with};

    #[test]
    fn ui_012_returns_official_french_text_and_set_name() {
        let localization = load_card_localization_with("c21", "263", |url| {
            assert!(url.ends_with("/c21/263/fr"));
            Ok(Some(
                br#"{"set_name":"Commander 2021","lang":"fr","printed_text":"Ajoutez deux manas.","card_faces":null}"#
                    .to_vec(),
            ))
        })
        .unwrap();

        assert_eq!(localization.set_name(), "Commander 2021");
        assert_eq!(localization.french_text(), Some("Ajoutez deux manas."));
    }

    #[test]
    fn ui_012_uses_french_text_from_another_printing_of_the_same_card() {
        let oracle_id = "00000000-0000-4000-8000-000000000001";
        let french_text = find_card_french_text_with(oracle_id, |url| {
            assert!(url.contains(&format!("q=oracleid%3A{oracle_id}%20lang%3Afr")));
            Ok(Some(
                br#"{"data":[
                    {"set_name":"Other Set","oracle_id":"another-card","lang":"fr","printed_text":"Ignorez ce texte."},
                    {"set_name":"Other Set","oracle_id":"00000000-0000-4000-8000-000000000001","lang":"en","printed_text":"Draw one card."},
                    {"set_name":"Other Set","oracle_id":"00000000-0000-4000-8000-000000000001","lang":"fr","printed_text":"Piochez une carte."}
                ]}"#
                    .to_vec(),
            ))
        })
        .unwrap();

        assert_eq!(french_text.as_deref(), Some("Piochez une carte."));
    }

    #[test]
    fn ui_012_keeps_the_received_text_when_localized_metadata_repeats_english_rules() {
        let localization = load_card_localization_with("eoe", "128", |url| {
            assert!(url.ends_with("/eoe/128/fr"));
            Ok(Some(
                    br#"{"set_name":"Edge of Eternities","oracle_id":"same-card","lang":"fr","oracle_text":"Destroy all creatures.","printed_text":" Destroy all\ncreatures. "}"#
                        .to_vec(),
                ))
        })
        .unwrap();

        assert_eq!(localization.french_text(), None);
        assert_eq!(localization.oracle_id(), Some("same-card"));
        assert_eq!(
            localization.fallback_text(),
            Some("Destroy all\ncreatures.")
        );
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
        let error = find_card_french_text_with("not-an-oracle-id", |_| {
            panic!("invalid input must not reach the network")
        })
        .unwrap_err();
        assert!(matches!(error, CardLocalizationError::InvalidRequest(_)));
    }
}
