use std::{error::Error, fmt, io::Read, sync::LazyLock, time::Duration};

use serde::{Deserialize, Serialize};
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
    localized_text: Option<String>,
    fallback_text: Option<String>,
    oracle_id: Option<String>,
    legalities: Option<CardLegalities>,
    related_tokens: Vec<RelatedToken>,
    needs_english_token_lookup: bool,
}

impl CardLocalization {
    #[must_use]
    pub fn set_name(&self) -> &str {
        &self.set_name
    }

    #[must_use]
    pub fn french_text(&self) -> Option<&str> {
        self.localized_text()
    }

    #[must_use]
    pub fn localized_text(&self) -> Option<&str> {
        self.localized_text.as_deref()
    }

    #[must_use]
    pub fn fallback_text(&self) -> Option<&str> {
        self.fallback_text.as_deref()
    }

    #[must_use]
    pub fn oracle_id(&self) -> Option<&str> {
        self.oracle_id.as_deref()
    }

    #[must_use]
    pub fn legalities(&self) -> Option<CardLegalities> {
        self.legalities
    }

    #[must_use]
    pub fn related_tokens(&self) -> &[RelatedToken] {
        &self.related_tokens
    }

    #[must_use]
    pub fn needs_english_token_lookup(&self) -> bool {
        self.needs_english_token_lookup
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedToken {
    scryfall_id: String,
    name: String,
    type_line: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct CardLegalities {
    commander: Option<CardLegality>,
    standard: Option<CardLegality>,
    modern: Option<CardLegality>,
    pauper: Option<CardLegality>,
    legacy: Option<CardLegality>,
    vintage: Option<CardLegality>,
    pioneer: Option<CardLegality>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum CardLegality {
    Legal,
    NotLegal,
    Banned,
    Restricted,
    Suspended,
    #[serde(other)]
    Unknown,
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
    id: Option<String>,
    set_name: String,
    oracle_id: Option<String>,
    lang: Option<String>,
    oracle_text: Option<String>,
    printed_text: Option<String>,
    card_faces: Option<Vec<ScryfallFace>>,
    legalities: Option<CardLegalities>,
    all_parts: Option<Vec<ScryfallRelatedCard>>,
}

#[derive(Debug, Deserialize)]
struct ScryfallRelatedCard {
    id: String,
    component: String,
    name: String,
    type_line: String,
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

#[derive(Deserialize)]
struct DeckLegalityResponse {
    data: Vec<DeckLegalityPrinting>,
}

#[derive(Deserialize)]
struct DeckLegalityPrinting {
    id: String,
    name: String,
    set: String,
    collector_number: String,
    legalities: Option<CardLegalities>,
    all_parts: Option<Vec<ScryfallRelatedCard>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckLegalityTarget {
    name: String,
    scryfall_id: Option<String>,
    set_code: Option<String>,
    collector_number: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckCardLegality {
    name: String,
    legalities: Option<CardLegalities>,
    related_tokens: Vec<RelatedToken>,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(untagged)]
enum DeckLookup<'a> {
    Id { id: &'a str },
    Printing { set: &'a str, collector_number: &'a str },
    Name { name: &'a str },
}

impl DeckLegalityTarget {
    fn id(&self) -> Option<DeckLookup<'_>> {
        self.scryfall_id.as_deref().filter(|id| is_scryfall_id(id)).map(|id| DeckLookup::Id { id })
    }

    fn printing(&self) -> Option<DeckLookup<'_>> {
        let set = self.set_code.as_deref()?.trim();
        let collector_number = self.collector_number.as_deref()?.trim();
        (!set.is_empty() && set.len() <= 32 && !collector_number.is_empty() && collector_number.len() <= 128)
            .then_some(DeckLookup::Printing { set, collector_number })
    }

    fn primary(&self) -> DeckLookup<'_> {
        self.printing()
            .or_else(|| self.id())
            .unwrap_or(DeckLookup::Name { name: self.name.trim() })
    }
}

impl DeckLookup<'_> {
    fn matches(&self, card: &DeckLegalityPrinting) -> bool {
        match self {
            Self::Id { id } => id.eq_ignore_ascii_case(&card.id),
            Self::Printing { set, collector_number } => {
                set.eq_ignore_ascii_case(&card.set)
                    && collector_number.eq_ignore_ascii_case(&card.collector_number)
            }
            Self::Name { name } => normalize_deck_name(name) == normalize_deck_name(&card.name),
        }
    }
}

fn normalize_deck_name(name: &str) -> String {
    name.split('/').map(str::trim).filter(|part| !part.is_empty())
        .collect::<Vec<_>>().join(" // ").to_lowercase()
}

pub fn load_deck_legalities(targets: &[DeckLegalityTarget]) -> Result<Vec<DeckCardLegality>, CardLocalizationError> {
    load_deck_legalities_with(targets, download_deck_legalities)
}

fn load_deck_legalities_with<'a>(
    targets: &'a [DeckLegalityTarget],
    mut download: impl FnMut(&[DeckLookup<'a>]) -> Result<Vec<DeckLegalityPrinting>, CardLocalizationError>,
) -> Result<Vec<DeckCardLegality>, CardLocalizationError> {
    if targets.is_empty() || targets.len() > 75
        || targets.iter().any(|target| target.name.trim().is_empty() || target.name.len() > 512)
    {
        return Err(CardLocalizationError::InvalidRequest("invalid deck card identifiers"));
    }
    let identifiers = targets.iter().map(DeckLegalityTarget::primary).collect::<Vec<_>>();
    let data = download(&identifiers)?;
    let mut matched = Vec::new();
    let mut fallback = Vec::new();
    for (target, identifier) in targets.iter().zip(identifiers) {
        if let Some(card) = data.iter().find(|card| identifier.matches(card)) {
            matched.push(deck_legality_result(target, card));
        } else {
            let name = normalize_deck_name(&target.name);
            let exact_name = target.name.trim();
            let alternate = target.id().or_else(|| target.printing()).filter(|reference| *reference != identifier)
                .or_else(|| {
                    let front = exact_name.split('/').next().unwrap_or(exact_name).trim();
                    match identifier {
                        DeckLookup::Name { .. } if front != exact_name => Some(DeckLookup::Name { name: front }),
                        DeckLookup::Name { .. } => None,
                        _ => Some(DeckLookup::Name { name: exact_name }),
                    }
                });
            if let Some(alternate) = alternate {
                fallback.push((target, alternate, name));
            }
        }
    }
    if !fallback.is_empty() {
        let identifiers = fallback.iter().map(|(_, identifier, _)| *identifier).collect::<Vec<_>>();
        if let Ok(data) = download(&identifiers) {
            for (target, identifier, name) in fallback {
                let card = data.iter().find(|card| {
                    identifier.matches(card)
                        || (matches!(identifier, DeckLookup::Name { .. })
                            && name == normalize_deck_name(&card.name))
                });
                if let Some(card) = card {
                    matched.push(deck_legality_result(target, card));
                }
            }
        }
    }
    Ok(matched)
}

fn deck_legality_result(target: &DeckLegalityTarget, card: &DeckLegalityPrinting) -> DeckCardLegality {
    DeckCardLegality {
        name: target.name.clone(),
        legalities: card.legalities,
        related_tokens: token_links(Some(&card.id), card.all_parts.as_deref().unwrap_or_default()),
    }
}

fn download_deck_legalities(identifiers: &[DeckLookup<'_>]) -> Result<Vec<DeckLegalityPrinting>, CardLocalizationError> {
    let payload = serde_json::to_vec(&serde_json::json!({ "identifiers": identifiers }))?;
    let response = SCRYFALL_AGENT
        .post("https://api.scryfall.com/cards/collection")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header(
            "User-Agent",
            concat!("DicoMTG/", env!("CARGO_PKG_VERSION"), " (desktop collection manager)"),
        )
        .send(payload.as_slice())?;
    let mut bytes = Vec::new();
    response.into_body().into_reader().take(MAX_RESPONSE_BYTES as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(CardLocalizationError::TooLarge);
    }
    Ok(serde_json::from_slice::<DeckLegalityResponse>(&bytes)?.data)
}

impl ScryfallCard {
    fn take_related_tokens(&mut self) -> Vec<RelatedToken> {
        token_links(self.id.as_deref(), self.all_parts.as_deref().unwrap_or_default())
    }

    fn printed_rules(&self, require_localized: bool) -> Option<String> {
        let oracle_text = self.oracle_text.as_deref().filter(|_| require_localized);
        if let Some(text) = usable_printed_text(self.printed_text.as_deref(), oracle_text) {
            return Some(text.to_owned());
        }

        let mut faces = Vec::new();
        for face in self.card_faces.as_ref()? {
            let oracle_text = face.oracle_text.as_deref().filter(|_| require_localized);
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

fn token_links(card_id: Option<&str>, parts: &[ScryfallRelatedCard]) -> Vec<RelatedToken> {
    parts.iter().filter(|part| {
        part.component == "token"
            && is_scryfall_id(&part.id)
            && card_id != Some(part.id.as_str())
            && !part.name.trim().is_empty()
    }).map(|part| RelatedToken {
        scryfall_id: part.id.to_ascii_lowercase(),
        name: part.name.clone(),
        type_line: part.type_line.clone(),
    }).collect()
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
    load_card_localization_for_language(set_code, collector_number, "fr")
}

pub fn load_card_localization_for_language(
    set_code: &str,
    collector_number: &str,
    language: &str,
) -> Result<CardLocalization, CardLocalizationError> {
    load_card_localization_with(set_code, collector_number, language, |url| {
        download_json(&SCRYFALL_AGENT, url)
    })
}

fn validate_card_language(language: &str) -> Result<(), CardLocalizationError> {
    if matches!(language, "en" | "fr" | "de" | "es" | "it" | "pt" | "ja") {
        Ok(())
    } else {
        Err(CardLocalizationError::InvalidRequest("invalid card language"))
    }
}

fn printing_url(
    set_code: &str,
    collector_number: &str,
) -> Result<String, CardLocalizationError> {
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

    Ok(format!(
        "https://api.scryfall.com/cards/{}/{}",
        encode_path_segment(&set_code.to_ascii_lowercase()),
        encode_path_segment(collector_number),
    ))
}

fn load_card_localization_with(
    set_code: &str,
    collector_number: &str,
    language: &str,
    mut download: impl FnMut(&str) -> Result<Option<Vec<u8>>, CardLocalizationError>,
) -> Result<CardLocalization, CardLocalizationError> {
    validate_card_language(language)?;
    let url = printing_url(set_code, collector_number)?;
    let bytes = match download(&format!("{url}/{language}"))? {
        Some(bytes) => bytes,
        None if language != "en" => {
            download(&format!("{url}/en"))?.ok_or(CardLocalizationError::Unavailable)?
        }
        None => return Err(CardLocalizationError::Unavailable),
    };
    let mut card: ScryfallCard = serde_json::from_slice(&bytes)?;
    let is_localized = language != "en" && card.lang.as_deref() == Some(language);
    let localized_text = is_localized.then(|| card.printed_rules(true)).flatten();
    let fallback_text = if is_localized && localized_text.is_none() {
        card.printed_rules(false)
    } else {
        None
    };
    let related_tokens = card.take_related_tokens();
    let needs_english_token_lookup =
        is_localized && related_tokens.is_empty() && card.oracle_id.is_some();
    Ok(CardLocalization {
        set_name: card.set_name,
        localized_text,
        fallback_text,
        oracle_id: card.oracle_id,
        legalities: card.legalities,
        related_tokens,
        needs_english_token_lookup,
    })
}

pub fn find_english_related_tokens(
    set_code: &str,
    collector_number: &str,
    oracle_id: &str,
) -> Result<Vec<RelatedToken>, CardLocalizationError> {
    find_english_related_tokens_with(set_code, collector_number, oracle_id, |url| {
        download_json(&SCRYFALL_AGENT, url)
    })
}

fn find_english_related_tokens_with(
    set_code: &str,
    collector_number: &str,
    oracle_id: &str,
    download: impl FnOnce(&str) -> Result<Option<Vec<u8>>, CardLocalizationError>,
) -> Result<Vec<RelatedToken>, CardLocalizationError> {
    if !is_scryfall_id(oracle_id) {
        return Err(CardLocalizationError::InvalidRequest(
            "invalid Oracle identifier",
        ));
    }
    let url = printing_url(set_code, collector_number)?;
    let Some(bytes) = download(&format!("{url}/en"))? else {
        return Ok(Vec::new());
    };
    let mut card: ScryfallCard = serde_json::from_slice(&bytes)?;
    if card.oracle_id.as_deref() != Some(oracle_id) {
        return Ok(Vec::new());
    }
    Ok(card.take_related_tokens())
}

pub fn find_card_french_text(oracle_id: &str) -> Result<Option<String>, CardLocalizationError> {
    find_card_text_for_language(oracle_id, "fr")
}

pub fn find_card_text_for_language(
    oracle_id: &str,
    language: &str,
) -> Result<Option<String>, CardLocalizationError> {
    find_card_text_with(oracle_id, language, |url| download_json(&SCRYFALL_AGENT, url))
}

fn find_card_text_with(
    oracle_id: &str,
    language: &str,
    download: impl FnOnce(&str) -> Result<Option<Vec<u8>>, CardLocalizationError>,
) -> Result<Option<String>, CardLocalizationError> {
    validate_card_language(language)?;
    if !is_scryfall_id(oracle_id) {
        return Err(CardLocalizationError::InvalidRequest(
            "invalid Oracle identifier",
        ));
    }

    let query = encode_path_segment(&format!("oracleid:{oracle_id} lang:{language}"));
    let url = format!("https://api.scryfall.com/cards/search?q={query}&unique=prints");
    let Some(bytes) = download(&url)? else {
        return Ok(None);
    };
    let printings: ScryfallCardList = serde_json::from_slice(&bytes)?;
    Ok(printings
        .data
        .into_iter()
        .filter(|printing| {
            printing.lang.as_deref() == Some(language)
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

    use super::{CardLocalizationError, find_card_text_with, load_card_localization_with};

    #[test]
    fn deck_legality_matches_printings_without_using_the_display_name_as_a_key() {
        let full_name = "Emeritus of Truce // Swords to Plowshares";
        let targets = serde_json::from_value::<Vec<super::DeckLegalityTarget>>(serde_json::json!([
            {"name": full_name, "setCode":"SOS", "collectorNumber":"13"},
            {"name": "Swords to Plowshares", "scryfallId":"00000000-0000-4000-8000-000000000001"}
        ])).unwrap();
        let result = super::load_deck_legalities_with(&targets, |lookups| {
            assert_eq!(serde_json::to_value(lookups).unwrap(), serde_json::json!([
                {"set":"SOS", "collector_number":"13"},
                {"id":"00000000-0000-4000-8000-000000000001"}
            ]));
            Ok(serde_json::from_value(serde_json::json!([
                {"name":"Swords to Plowshares","id":"00000000-0000-4000-8000-000000000001","set":"lea","collector_number":"40","legalities":{"standard":"not_legal"}},
                {"name":"Emeritus of Truce","id":"00000000-0000-4000-8000-000000000002","set":"sos","collector_number":"13","legalities":{"standard":"legal"},
                 "all_parts":[
                     {"name":"Clue","id":"00000000-0000-4000-8000-000000000003","type_line":"Token Artifact — Clue","component":"token"},
                     {"name":"Other","id":"00000000-0000-4000-8000-000000000004","type_line":"Artifact","component":"combo_piece"}
                 ]}
            ]))?)
        }).unwrap();
        let result = serde_json::to_value(result).unwrap();
        assert_eq!(result[0]["name"], full_name);
        assert_eq!(result[0]["legalities"]["standard"], "legal");
        assert_eq!(result[0]["relatedTokens"].as_array().unwrap().len(), 1);
        assert_eq!(result[0]["relatedTokens"][0]["name"], "Clue");
        assert_eq!(result[1]["legalities"]["standard"], "not_legal");
    }

    #[test]
    fn ui_012_returns_official_french_text_and_set_name() {
        let localization = load_card_localization_with("c21", "263", "fr", |url| {
            assert!(url.ends_with("/c21/263/fr"));
            Ok(Some(
                br#"{"set_name":"Commander 2021","lang":"fr","printed_text":"Ajoutez deux manas.","card_faces":null}"#
                    .to_vec(),
            ))
        })
        .unwrap();

        assert_eq!(localization.set_name(), "Commander 2021");
        assert_eq!(localization.french_text(), Some("Ajoutez deux manas."));
        assert!(localization.legalities().is_none());
        assert!(localization.related_tokens().is_empty());
    }

    #[test]
    fn returns_only_direct_token_links_from_the_card_response() {
        let localization = load_card_localization_with("som", "223", "fr", |url| {
            assert!(url.ends_with("/som/223/fr"));
            Ok(Some(
                br#"{"set_name":"Scars of Mirrodin","lang":"fr",
                    "id":"00000000-0000-4000-8000-000000000001",
                    "all_parts":[
                        {"id":"00000000-0000-4000-8000-000000000002","component":"token","name":"Wurm","type_line":"Token Artifact Creature"},
                        {"id":"00000000-0000-4000-8000-000000000003","component":"token","name":"Wurm","type_line":"Token Artifact Creature"},
                        {"id":"00000000-0000-4000-8000-000000000001","component":"token","name":"Self","type_line":""},
                        {"id":"00000000-0000-4000-8000-000000000004","component":"combo_piece","name":"Other card","type_line":""},
                        {"id":"invalid","component":"token","name":"Invalid card","type_line":""}
                    ]}"#
                    .to_vec(),
            ))
        })
        .unwrap();

        assert_eq!(
            serde_json::to_value(localization.related_tokens()).unwrap(),
            serde_json::json!([
                {"scryfallId":"00000000-0000-4000-8000-000000000002","name":"Wurm","typeLine":"Token Artifact Creature"},
                {"scryfallId":"00000000-0000-4000-8000-000000000003","name":"Wurm","typeLine":"Token Artifact Creature"}
            ])
        );
    }

    #[test]
    fn finds_english_token_links_when_the_french_printing_omits_them() {
        for (set, number, oracle_id, token_id, name) in [
            (
                "dmu",
                "42",
                "fb220f46-f8b8-4804-baa4-e7d50b4871f7",
                "5f3034f6-145f-4e60-9e55-c4054fd8e70f",
                "Bird",
            ),
            (
                "dsk",
                "118",
                "bd388ad9-a47b-4b0b-b94a-8e4343cd3de5",
                "bba307eb-814c-4c87-acdf-b54c87d04f82",
                "Demon",
            ),
            (
                "blb",
                "116",
                "8d72ddb3-ef6e-4847-b6d4-5d85b1dd5802",
                "5379a77e-4b98-4abc-b967-fbc76bb85fc6",
                "Starscape Cleric",
            ),
        ] {
            let localization = load_card_localization_with(set, number, "fr", |url| {
                assert!(url.ends_with(&format!("/{set}/{number}/fr")));
                Ok(Some(serde_json::to_vec(&serde_json::json!({
                    "set_name": "Set",
                    "lang": "fr",
                    "oracle_id": oracle_id,
                    "printed_text": "Texte français."
                }))?))
            })
            .unwrap();
            assert_eq!(localization.french_text(), Some("Texte français."));
            assert!(localization.needs_english_token_lookup());

            let links = super::find_english_related_tokens_with(set, number, oracle_id, |url| {
                assert!(url.ends_with(&format!("/{set}/{number}/en")));
                Ok(Some(serde_json::to_vec(&serde_json::json!({
                    "set_name": "Set",
                    "oracle_id": oracle_id,
                    "all_parts": [{
                        "id": token_id, "name": name, "component": "token", "type_line": "Token Creature"
                    }]
                }))?))
            })
            .unwrap();
            assert_eq!(links.len(), 1);
            assert_eq!(links[0].scryfall_id, token_id);
            assert_eq!(links[0].name, name);
        }
    }

    #[test]
    fn returns_card_legalities_with_the_existing_metadata_request() {
        let localization = load_card_localization_with("tst", "1", "fr", |url| {
            assert!(url.ends_with("/tst/1/fr"));
            Ok(Some(
                br#"{"set_name":"Test Set","lang":"fr","legalities":{
                    "commander":"legal","standard":"not_legal","modern":"banned",
                    "pauper":"not_legal","legacy":"legal","vintage":"restricted",
                    "pioneer":"new_status","brawl":"legal"
                }}"#
                    .to_vec(),
            ))
        })
        .unwrap();

        assert_eq!(
            serde_json::to_value(localization.legalities()).unwrap(),
            serde_json::json!({
                "commander": "legal", "standard": "not_legal", "modern": "banned",
                "pauper": "not_legal", "legacy": "legal", "vintage": "restricted",
                "pioneer": "unknown"
            })
        );
    }

    #[test]
    fn ui_012_uses_french_text_from_another_printing_of_the_same_card() {
        let oracle_id = "00000000-0000-4000-8000-000000000001";
        let french_text = find_card_text_with(oracle_id, "fr", |url| {
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
        let localization = load_card_localization_with("eoe", "128", "fr", |url| {
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
        let localization = load_card_localization_with("tst", "1", "fr", |url| {
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
        let error = load_card_localization_with("", "1", "fr", |_| {
            panic!("invalid input must not reach the network")
        })
        .unwrap_err();

        assert!(matches!(error, CardLocalizationError::InvalidRequest(_)));
        let error = find_card_text_with("not-an-oracle-id", "fr", |_| {
            panic!("invalid input must not reach the network")
        })
        .unwrap_err();
        assert!(matches!(error, CardLocalizationError::InvalidRequest(_)));
    }

    #[test]
    fn reads_only_the_selected_printed_language_and_supports_an_english_fallback() {
        let localization = load_card_localization_with("tst", "1", "de", |url| {
            assert!(url.ends_with("/de"));
            Ok(Some(br#"{"set_name":"Test","lang":"de","printed_text":"Ziehe eine Karte."}"#.to_vec()))
        }).unwrap();
        assert_eq!(localization.localized_text(), Some("Ziehe eine Karte."));

        let localization = load_card_localization_with("tst", "1", "en", |url| {
            assert!(url.ends_with("/en"));
            Ok(Some(br#"{"set_name":"Test","lang":"en","oracle_text":"Draw a card."}"#.to_vec()))
        }).unwrap();
        assert_eq!(localization.localized_text(), None);
    }
}
