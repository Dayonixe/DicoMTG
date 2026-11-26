use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt,
    io::Read,
};

use csv::{ReaderBuilder, StringRecord, Trim};

use crate::{CardCandidate, PrintingKey};

const REQUIRED_HEADERS: [&str; 17] = [
    "colorIdentity",
    "colors",
    "finishes",
    "keywords",
    "language",
    "manaCost",
    "manaValue",
    "name",
    "number",
    "power",
    "rarity",
    "setCode",
    "text",
    "toughness",
    "type",
    "types",
    "uuid",
];

const TOKEN_REQUIRED_HEADERS: [&str; 15] = [
    "colorIdentity",
    "colors",
    "finishes",
    "keywords",
    "language",
    "manaCost",
    "name",
    "number",
    "power",
    "setCode",
    "text",
    "toughness",
    "type",
    "types",
    "uuid",
];

#[derive(Debug, Clone, PartialEq)]
pub struct CatalogCard {
    pub(crate) uuid: String,
    pub(crate) scryfall_id: Option<String>,
    pub(crate) cardmarket_id: Option<u64>,
    pub(crate) edhrec_saltiness: Option<f64>,
    pub(crate) edhrec_rank: Option<u32>,
    pub(crate) has_back_face: bool,
    pub(crate) name: String,
    pub(crate) key: PrintingKey,
    pub(crate) rarity: String,
    pub(crate) mana_value: f64,
    pub(crate) colors: Vec<String>,
    pub(crate) color_identity: Vec<String>,
    pub(crate) types: Vec<String>,
    pub(crate) keywords: Vec<String>,
    pub(crate) finishes: Vec<String>,
    pub(crate) language: String,
    pub(crate) mana_cost: Option<String>,
    pub(crate) power: Option<String>,
    pub(crate) toughness: Option<String>,
    pub(crate) type_line: String,
    pub(crate) rules_text: Option<String>,
}

impl CatalogCard {
    #[must_use]
    pub fn uuid(&self) -> &str {
        &self.uuid
    }

    #[must_use]
    pub fn scryfall_id(&self) -> Option<&str> {
        self.scryfall_id.as_deref()
    }

    #[must_use]
    pub fn cardmarket_id(&self) -> Option<u64> {
        self.cardmarket_id
    }

    #[must_use]
    pub fn has_back_face(&self) -> bool {
        self.has_back_face
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn key(&self) -> &PrintingKey {
        &self.key
    }

    #[must_use]
    pub fn rarity(&self) -> &str {
        &self.rarity
    }

    #[must_use]
    pub fn mana_value(&self) -> f64 {
        self.mana_value
    }

    #[must_use]
    pub fn colors(&self) -> &[String] {
        &self.colors
    }

    #[must_use]
    pub fn color_identity(&self) -> &[String] {
        &self.color_identity
    }

    #[must_use]
    pub fn types(&self) -> &[String] {
        &self.types
    }

    #[must_use]
    pub fn keywords(&self) -> &[String] {
        &self.keywords
    }

    #[must_use]
    pub fn finishes(&self) -> &[String] {
        &self.finishes
    }

    #[must_use]
    pub fn language(&self) -> &str {
        &self.language
    }

    #[must_use]
    pub fn mana_cost(&self) -> Option<&str> {
        self.mana_cost.as_deref()
    }

    #[must_use]
    pub fn power(&self) -> Option<&str> {
        self.power.as_deref()
    }

    #[must_use]
    pub fn toughness(&self) -> Option<&str> {
        self.toughness.as_deref()
    }

    #[must_use]
    pub fn type_line(&self) -> &str {
        &self.type_line
    }

    #[must_use]
    pub fn rules_text(&self) -> Option<&str> {
        self.rules_text.as_deref()
    }

    #[must_use]
    pub fn matching_candidate(&self) -> CardCandidate {
        CardCandidate::new_with_details(
            &self.uuid,
            &self.name,
            self.key.set_code(),
            self.key.collector_number(),
            Some(&self.language),
            &self.finishes,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CatalogImport {
    cards: Vec<CatalogCard>,
}

impl CatalogImport {
    #[must_use]
    pub fn cards(&self) -> &[CatalogCard] {
        &self.cards
    }

    pub(crate) fn extend(&mut self, other: Self) {
        self.cards.extend(other.cards);
    }

    pub(crate) fn attach_scryfall_ids(
        &mut self,
        source: impl Read,
    ) -> Result<(), CatalogImportError> {
        let mut reader = ReaderBuilder::new().trim(Trim::All).from_reader(source);
        let headers = reader.headers()?.clone();
        let uuid_index = header_position(&headers, "uuid")?;
        let scryfall_id_index = headers.iter().position(|header| header == "scryfallId");
        let cardmarket_id_index = headers.iter().position(|header| header == "mcmId");
        if scryfall_id_index.is_none() && cardmarket_id_index.is_none() {
            return Err(CatalogImportError::MissingHeaders {
                names: vec!["scryfallId or mcmId".to_owned()],
            });
        }
        let card_positions = self
            .cards
            .iter()
            .enumerate()
            .map(|(position, card)| (card.uuid.clone(), position))
            .collect::<HashMap<_, _>>();
        let mut seen = HashSet::new();

        for result in reader.records() {
            let record = result?;
            let line = record
                .position()
                .map_or(0, |position| position.line() as usize);
            let uuid = required_field(&record, uuid_index, "uuid", line)?;
            if !seen.insert(uuid.to_owned()) {
                return Err(CatalogImportError::InvalidRow {
                    line,
                    reason: CatalogRowError::DuplicateUuid {
                        uuid: uuid.to_owned(),
                    },
                });
            }
            if let Some(position) = card_positions.get(uuid) {
                self.cards[*position].scryfall_id =
                    scryfall_id_index.and_then(|index| optional_field(&record, index));
                self.cards[*position].cardmarket_id = cardmarket_id_index
                    .and_then(|index| optional_field(&record, index))
                    .map(|value| parse_cardmarket_id(&value, line))
                    .transpose()?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogRowError {
    MissingField { field: &'static str },
    InvalidManaValue { value: String },
    DuplicateUuid { uuid: String },
    InvalidCardmarketId { value: String },
}

#[derive(Debug)]
pub enum CatalogImportError {
    Csv(csv::Error),
    MissingHeaders {
        names: Vec<String>,
    },
    InvalidRow {
        line: usize,
        reason: CatalogRowError,
    },
    EmptyCatalog,
}

impl fmt::Display for CatalogImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Csv(error) => write!(formatter, "invalid MTGJSON CSV: {error}"),
            Self::MissingHeaders { names } => {
                write!(formatter, "missing MTGJSON headers: {}", names.join(", "))
            }
            Self::InvalidRow { line, reason } => {
                write!(formatter, "invalid MTGJSON row {line}: {reason}")
            }
            Self::EmptyCatalog => write!(formatter, "MTGJSON catalog contains no cards"),
        }
    }
}

impl Error for CatalogImportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Csv(error) => Some(error),
            Self::MissingHeaders { .. } | Self::InvalidRow { .. } | Self::EmptyCatalog => None,
        }
    }
}

impl From<csv::Error> for CatalogImportError {
    fn from(error: csv::Error) -> Self {
        Self::Csv(error)
    }
}

impl fmt::Display for CatalogRowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingField { field } => write!(formatter, "missing field {field}"),
            Self::InvalidManaValue { value } => write!(formatter, "invalid mana value {value}"),
            Self::DuplicateUuid { uuid } => write!(formatter, "duplicate UUID {uuid}"),
            Self::InvalidCardmarketId { value } => {
                write!(formatter, "invalid Cardmarket product identifier {value}")
            }
        }
    }
}

pub fn import_mtgjson_cards_csv(source: impl Read) -> Result<CatalogImport, CatalogImportError> {
    import_mtgjson_csv(source, &REQUIRED_HEADERS)
}

pub fn import_mtgjson_tokens_csv(source: impl Read) -> Result<CatalogImport, CatalogImportError> {
    import_mtgjson_csv(source, &TOKEN_REQUIRED_HEADERS)
}

fn import_mtgjson_csv(
    source: impl Read,
    required_headers: &[&str],
) -> Result<CatalogImport, CatalogImportError> {
    let mut reader = ReaderBuilder::new().trim(Trim::All).from_reader(source);
    let indices = HeaderIndices::new(reader.headers()?, required_headers)?;
    let mut cards = Vec::new();
    let mut uuids = HashSet::new();

    for result in reader.records() {
        let record = result?;
        let line = record
            .position()
            .map_or(0, |position| position.line() as usize);
        if indices
            .side
            .and_then(|index| record.get(index))
            .is_some_and(|side| side.eq_ignore_ascii_case("b"))
        {
            continue;
        }
        let card = parse_record(&record, indices, line)?;
        if !uuids.insert(card.uuid.clone()) {
            return Err(CatalogImportError::InvalidRow {
                line,
                reason: CatalogRowError::DuplicateUuid { uuid: card.uuid },
            });
        }
        cards.push(card);
    }

    if cards.is_empty() {
        Err(CatalogImportError::EmptyCatalog)
    } else {
        Ok(CatalogImport { cards })
    }
}

#[derive(Clone, Copy)]
struct HeaderIndices {
    color_identity: usize,
    colors: usize,
    finishes: usize,
    keywords: usize,
    language: usize,
    mana_cost: usize,
    mana_value: Option<usize>,
    name: usize,
    number: usize,
    power: usize,
    rarity: Option<usize>,
    set_code: usize,
    text: usize,
    toughness: usize,
    type_line: usize,
    types: usize,
    uuid: usize,
    scryfall_id: Option<usize>,
    cardmarket_id: Option<usize>,
    edhrec_saltiness: Option<usize>,
    edhrec_rank: Option<usize>,
    side: Option<usize>,
    layout: Option<usize>,
}

impl HeaderIndices {
    fn new(headers: &StringRecord, required_headers: &[&str]) -> Result<Self, CatalogImportError> {
        let missing = required_headers
            .iter()
            .filter(|required| !headers.iter().any(|header| header == **required))
            .map(|header| (*header).to_owned())
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(CatalogImportError::MissingHeaders { names: missing });
        }

        Ok(Self {
            color_identity: header_position(headers, "colorIdentity")?,
            colors: header_position(headers, "colors")?,
            finishes: header_position(headers, "finishes")?,
            keywords: header_position(headers, "keywords")?,
            language: header_position(headers, "language")?,
            mana_cost: header_position(headers, "manaCost")?,
            mana_value: headers.iter().position(|header| header == "manaValue"),
            name: header_position(headers, "name")?,
            number: header_position(headers, "number")?,
            power: header_position(headers, "power")?,
            rarity: headers.iter().position(|header| header == "rarity"),
            set_code: header_position(headers, "setCode")?,
            text: header_position(headers, "text")?,
            toughness: header_position(headers, "toughness")?,
            type_line: header_position(headers, "type")?,
            types: header_position(headers, "types")?,
            uuid: header_position(headers, "uuid")?,
            scryfall_id: headers.iter().position(|header| header == "scryfallId"),
            cardmarket_id: headers.iter().position(|header| header == "mcmId"),
            edhrec_saltiness: headers
                .iter()
                .position(|header| header == "edhrecSaltiness"),
            edhrec_rank: headers.iter().position(|header| header == "edhrecRank"),
            side: headers.iter().position(|header| header == "side"),
            layout: headers.iter().position(|header| header == "layout"),
        })
    }
}

fn header_position(headers: &StringRecord, name: &str) -> Result<usize, CatalogImportError> {
    headers
        .iter()
        .position(|header| header == name)
        .ok_or_else(|| CatalogImportError::MissingHeaders {
            names: vec![name.to_owned()],
        })
}

fn parse_record(
    record: &StringRecord,
    indices: HeaderIndices,
    line: usize,
) -> Result<CatalogCard, CatalogImportError> {
    let uuid = required_field(record, indices.uuid, "uuid", line)?;
    let name = required_field(record, indices.name, "name", line)?;
    let set_code = required_field(record, indices.set_code, "setCode", line)?;
    let number = required_field(record, indices.number, "number", line)?;
    let rarity = indices.rarity.map_or(Ok("token"), |index| {
        required_field(record, index, "rarity", line)
    })?;
    let language = required_field(record, indices.language, "language", line)?;
    let type_line = required_field(record, indices.type_line, "type", line)?;
    let mana_value = indices.mana_value.map_or(Ok(0.0), |index| {
        let raw = required_field(record, index, "manaValue", line)?;
        raw.parse::<f64>()
            .ok()
            .filter(|value| value.is_finite() && *value >= 0.0)
            .ok_or_else(|| CatalogImportError::InvalidRow {
                line,
                reason: CatalogRowError::InvalidManaValue {
                    value: raw.to_owned(),
                },
            })
    })?;

    Ok(CatalogCard {
        uuid: uuid.to_owned(),
        scryfall_id: indices
            .scryfall_id
            .and_then(|index| optional_field(record, index)),
        cardmarket_id: indices
            .cardmarket_id
            .and_then(|index| optional_field(record, index))
            .map(|value| parse_cardmarket_id(&value, line))
            .transpose()?,
        edhrec_saltiness: indices
            .edhrec_saltiness
            .and_then(|index| record.get(index))
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| (0.0..=4.0).contains(value)),
        edhrec_rank: indices
            .edhrec_rank
            .and_then(|index| record.get(index))
            .and_then(parse_edhrec_rank),
        has_back_face: indices
            .layout
            .and_then(|index| record.get(index))
            .is_some_and(is_double_faced_layout),
        name: name.to_owned(),
        key: PrintingKey::new(set_code, number),
        rarity: rarity.to_owned(),
        mana_value,
        colors: list_field(record, indices.colors),
        color_identity: list_field(record, indices.color_identity),
        types: list_field(record, indices.types),
        keywords: list_field(record, indices.keywords),
        finishes: list_field(record, indices.finishes),
        language: language.to_owned(),
        mana_cost: optional_field(record, indices.mana_cost),
        power: optional_field(record, indices.power),
        toughness: optional_field(record, indices.toughness),
        type_line: type_line.to_owned(),
        rules_text: optional_field(record, indices.text),
    })
}

fn parse_edhrec_rank(value: &str) -> Option<u32> {
    let integer = value
        .trim_end_matches('0')
        .strip_suffix('.')
        .unwrap_or(value);
    integer.parse::<u32>().ok().filter(|rank| *rank > 0)
}

fn parse_cardmarket_id(value: &str, line: usize) -> Result<u64, CatalogImportError> {
    value
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| CatalogImportError::InvalidRow {
            line,
            reason: CatalogRowError::InvalidCardmarketId {
                value: value.to_owned(),
            },
        })
}

fn is_double_faced_layout(layout: &str) -> bool {
    matches!(
        layout.trim().to_ascii_lowercase().as_str(),
        "transform"
            | "modal_dfc"
            | "modal dfc"
            | "meld"
            | "double_faced"
            | "double-faced"
            | "double_faced_token"
            | "reversible"
            | "reversible_card"
    )
}

fn required_field<'a>(
    record: &'a StringRecord,
    index: usize,
    field: &'static str,
    line: usize,
) -> Result<&'a str, CatalogImportError> {
    let value = record.get(index).unwrap_or_default();
    if value.is_empty() {
        Err(CatalogImportError::InvalidRow {
            line,
            reason: CatalogRowError::MissingField { field },
        })
    } else {
        Ok(value)
    }
}

fn optional_field(record: &StringRecord, index: usize) -> Option<String> {
    record
        .get(index)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn list_field(record: &StringRecord, index: usize) -> Vec<String> {
    record
        .get(index)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}
