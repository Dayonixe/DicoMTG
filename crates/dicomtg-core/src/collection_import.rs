use std::{error::Error, fmt, io::Read};

use csv::{ReaderBuilder, StringRecord, Trim};

use crate::{CollectionCard, PrintingKey};

const REQUIRED_HEADERS: [&str; 4] = ["Quantity", "Card Name", "Set Code", "Card Number"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedCard {
    quantity: u32,
    card: CollectionCard,
}

impl ImportedCard {
    pub(crate) fn new(quantity: u32, card: CollectionCard) -> Self {
        Self { quantity, card }
    }

    #[must_use]
    pub fn quantity(&self) -> u32 {
        self.quantity
    }

    #[must_use]
    pub fn name(&self) -> &str {
        self.card.name()
    }

    #[must_use]
    pub fn key(&self) -> &PrintingKey {
        self.card.key()
    }

    #[must_use]
    pub fn language(&self) -> Option<&str> {
        self.card.language()
    }

    #[must_use]
    pub fn printing(&self) -> Option<&str> {
        self.card.printing()
    }

    pub(crate) fn card(&self) -> &CollectionCard {
        &self.card
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowIssueReason {
    MissingField { field: &'static str },
    InvalidQuantity { value: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportIssue {
    line: usize,
    reason: RowIssueReason,
}

impl ImportIssue {
    pub(crate) fn new(line: usize, reason: RowIssueReason) -> Self {
        Self { line, reason }
    }

    #[must_use]
    pub fn line(&self) -> usize {
        self.line
    }

    #[must_use]
    pub fn reason(&self) -> &RowIssueReason {
        &self.reason
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportReport {
    cards: Vec<ImportedCard>,
    issues: Vec<ImportIssue>,
    ignored_rows: usize,
}

impl ImportReport {
    #[must_use]
    pub fn cards(&self) -> &[ImportedCard] {
        &self.cards
    }

    #[must_use]
    pub fn issues(&self) -> &[ImportIssue] {
        &self.issues
    }

    #[must_use]
    pub fn ignored_rows(&self) -> usize {
        self.ignored_rows
    }

    pub(crate) fn into_parts(self) -> (Vec<ImportedCard>, Vec<ImportIssue>) {
        (self.cards, self.issues)
    }
}

#[derive(Debug)]
pub enum ImportError {
    Io(std::io::Error),
    Csv(csv::Error),
    MissingHeaders { names: Vec<String> },
    NoSortedMagicFolders,
}

impl fmt::Display for ImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "failed to read collection: {error}"),
            Self::Csv(error) => write!(formatter, "invalid collection CSV: {error}"),
            Self::MissingHeaders { names } => {
                write!(formatter, "missing required headers: {}", names.join(", "))
            }
            Self::NoSortedMagicFolders => write!(formatter, "no Magic cards in Sorted folders"),
        }
    }
}

impl Error for ImportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Csv(error) => Some(error),
            Self::MissingHeaders { .. } | Self::NoSortedMagicFolders => None,
        }
    }
}

impl From<std::io::Error> for ImportError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<csv::Error> for ImportError {
    fn from(error: csv::Error) -> Self {
        Self::Csv(error)
    }
}

pub fn import_collection_csv(mut source: impl Read) -> Result<ImportReport, ImportError> {
    let mut bytes = Vec::new();
    source.read_to_end(&mut bytes)?;
    let (csv, line_offset) = csv_body(&bytes);
    let mut reader = ReaderBuilder::new().trim(Trim::All).from_reader(csv);
    let indices = HeaderIndices::new(reader.headers()?)?;
    let mut cards = Vec::new();
    let mut issues = Vec::new();
    let mut ignored_rows = 0;

    for result in reader.records() {
        let record = result?;
        if indices.sorted.is_some_and(|(list_type, game)| {
            !record
                .get(list_type)
                .unwrap_or_default()
                .eq_ignore_ascii_case("Folder")
                || !record
                    .get(game)
                    .unwrap_or_default()
                    .eq_ignore_ascii_case("mtg")
        }) {
            ignored_rows += 1;
            continue;
        }
        let line = record
            .position()
            .map_or(0, |position| position.line() as usize + line_offset);
        match parse_record(&record, indices, line) {
            Ok(card) => cards.push(card),
            Err(issue) => issues.push(issue),
        }
    }

    if indices.sorted.is_some() && cards.is_empty() && issues.is_empty() {
        return Err(ImportError::NoSortedMagicFolders);
    }
    Ok(ImportReport {
        cards,
        issues,
        ignored_rows,
    })
}

#[derive(Clone, Copy)]
struct HeaderIndices {
    quantity: usize,
    name: usize,
    set_code: usize,
    collector_number: usize,
    language: Option<usize>,
    printing: Option<usize>,
    sorted: Option<(usize, usize)>,
}

impl HeaderIndices {
    fn new(headers: &StringRecord) -> Result<Self, ImportError> {
        let missing = REQUIRED_HEADERS
            .iter()
            .filter(|required| !headers.iter().any(|header| header == **required))
            .map(|header| (*header).to_owned())
            .collect::<Vec<_>>();

        if !missing.is_empty() {
            return Err(ImportError::MissingHeaders { names: missing });
        }

        Ok(Self {
            quantity: header_position(headers, "Quantity")?,
            name: header_position(headers, "Card Name")?,
            set_code: header_position(headers, "Set Code")?,
            collector_number: header_position(headers, "Card Number")?,
            language: headers.iter().position(|header| header == "Language"),
            printing: headers.iter().position(|header| header == "Printing"),
            sorted: if headers
                .iter()
                .any(|header| matches!(header, "List Type" | "Collection"))
            {
                Some((
                    header_position(headers, "List Type")?,
                    header_position(headers, "Collection")?,
                ))
            } else {
                None
            },
        })
    }
}

fn header_position(headers: &StringRecord, name: &str) -> Result<usize, ImportError> {
    headers
        .iter()
        .position(|header| header == name)
        .ok_or_else(|| ImportError::MissingHeaders {
            names: vec![name.to_owned()],
        })
}

fn parse_record(
    record: &StringRecord,
    indices: HeaderIndices,
    line: usize,
) -> Result<ImportedCard, ImportIssue> {
    let quantity = required_field(record, indices.quantity, "Quantity", line)?;
    let name = required_field(record, indices.name, "Card Name", line)?;
    let set_code = required_field(record, indices.set_code, "Set Code", line)?;
    let collector_number = required_field(record, indices.collector_number, "Card Number", line)?;
    let language = indices
        .language
        .and_then(|index| record.get(index))
        .filter(|value| !value.is_empty());
    let language = if indices.sorted.is_some() {
        language.map(sorted_language)
    } else {
        language
    };
    let printing = indices
        .printing
        .and_then(|index| record.get(index))
        .filter(|value| !value.is_empty());
    let quantity = quantity
        .parse::<u32>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| ImportIssue {
            line,
            reason: RowIssueReason::InvalidQuantity {
                value: quantity.to_owned(),
            },
        })?;

    Ok(ImportedCard {
        quantity,
        card: CollectionCard::new_with_details(
            name,
            set_code,
            collector_number,
            language,
            printing,
        ),
    })
}

fn sorted_language(code: &str) -> &str {
    match code.to_ascii_lowercase().as_str() {
        "en" => "English",
        "fr" => "French",
        "de" => "German",
        "es" => "Spanish",
        "it" => "Italian",
        "pt" => "Portuguese",
        "ja" | "jp" => "Japanese",
        "ko" => "Korean",
        "ru" => "Russian",
        _ => code,
    }
}

fn required_field<'a>(
    record: &'a StringRecord,
    index: usize,
    field: &'static str,
    line: usize,
) -> Result<&'a str, ImportIssue> {
    let value = record.get(index).unwrap_or_default();
    if value.is_empty() {
        Err(ImportIssue {
            line,
            reason: RowIssueReason::MissingField { field },
        })
    } else {
        Ok(value)
    }
}

pub(crate) fn csv_body(bytes: &[u8]) -> (&[u8], usize) {
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let Some(newline) = bytes.iter().position(|byte| *byte == b'\n') else {
        return (bytes, 0);
    };
    let first_line = bytes[..newline]
        .strip_suffix(b"\r")
        .unwrap_or(&bytes[..newline]);

    if first_line == b"sep=," || first_line == br#""sep=,""# {
        (&bytes[newline + 1..], 1)
    } else {
        (bytes, 0)
    }
}
