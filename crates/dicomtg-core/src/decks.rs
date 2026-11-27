use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{PrintingKey, collection_import::csv_body};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DeckFormat {
    #[default]
    Freeform,
    Commander,
    Standard,
    Pioneer,
    Modern,
    Pauper,
    Legacy,
    Vintage,
    Limited,
}

impl DeckFormat {
    fn from_sorted(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "mtg_commander" => Self::Commander,
            "mtg_standard" => Self::Standard,
            "mtg_pioneer" => Self::Pioneer,
            "mtg_modern" => Self::Modern,
            "mtg_pauper" => Self::Pauper,
            "mtg_legacy" => Self::Legacy,
            "mtg_vintage" => Self::Vintage,
            "mtg_limited" => Self::Limited,
            _ => Self::Freeform,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckCard {
    pub name: String,
    pub quantity: u32,
    pub board: String,
    #[serde(default)]
    pub set_code: String,
    #[serde(default)]
    pub collector_number: String,
    #[serde(default)]
    pub printing: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub card_type: String,
    #[serde(default)]
    pub mana_cost: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Deck {
    pub name: String,
    pub format: DeckFormat,
    #[serde(default)]
    pub colors: Vec<String>,
    #[serde(default)]
    pub cards: Vec<DeckCard>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SavedDeck {
    pub id: i64,
    pub revision: i64,
    #[serde(flatten)]
    pub deck: Deck,
}

impl Deck {
    pub(crate) fn validate(&mut self) -> Result<(), String> {
        self.name = self.name.trim().to_owned();
        if self.name.is_empty() || self.name.chars().count() > 160 {
            return Err("Donnez au deck un nom de 1 à 160 caractères.".to_owned());
        }
        if self
            .colors
            .iter()
            .any(|color| !matches!(color.as_str(), "W" | "U" | "B" | "R" | "G"))
        {
            return Err("Couleur de deck inconnue.".to_owned());
        }
        self.colors.sort_unstable();
        self.colors.dedup();
        for card in &mut self.cards {
            card.name = card.name.trim().to_owned();
            card.board = card.board.trim().to_owned();
            if card.name.is_empty() || card.quantity == 0 || card.board.is_empty() {
                return Err(
                    "Chaque ligne doit indiquer une carte, une quantité positive et une section."
                        .to_owned(),
                );
            }
        }
        Ok(())
    }
}

pub fn import_decks_csv(text: &str, filename: &str) -> Result<Vec<Deck>, String> {
    if text.len() > 20 * 1024 * 1024 {
        return Err("Le fichier CSV dépasse 20 Mio.".to_owned());
    }
    let (body, offset) = csv_body(text.as_bytes());
    let mut csv = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(body);
    let headers = csv.headers().map_err(|error| error.to_string())?.clone();
    let column = |name: &str| headers.iter().position(|header| header == name);
    let required =
        |name: &str| column(name).ok_or_else(|| format!("Colonne manquante dans le CSV : {name}."));
    let quantity = required("Quantity")?;
    let name = required("Card Name")?;
    let sorted = if column("List Type").is_some() || column("Collection").is_some() {
        Some((
            required("List Type")?,
            required("List Name")?,
            required("Collection")?,
        ))
    } else {
        None
    };
    let default_name = filename
        .trim()
        .trim_end_matches(".csv")
        .trim_end_matches(".CSV")
        .trim();
    let mut decks = BTreeMap::<String, Deck>::new();

    for record in csv.records() {
        let record = record.map_err(|error| error.to_string())?;
        let value = |field| {
            column(field)
                .and_then(|index| record.get(index))
                .unwrap_or_default()
        };
        let (deck_name, format) = match sorted {
            Some((list_type, list_name, game)) => {
                if !record
                    .get(list_type)
                    .unwrap_or_default()
                    .eq_ignore_ascii_case("Deck")
                    || !record
                        .get(game)
                        .unwrap_or_default()
                        .eq_ignore_ascii_case("mtg")
                {
                    continue;
                }
                (
                    record.get(list_name).unwrap_or_default(),
                    DeckFormat::from_sorted(value("Format")),
                )
            }
            None => (default_name, DeckFormat::Freeform),
        };
        let line = record
            .position()
            .map_or(0, |position| position.line() as usize + offset);
        let quantity = record
            .get(quantity)
            .unwrap_or_default()
            .parse::<u32>()
            .ok()
            .filter(|quantity| *quantity > 0)
            .ok_or_else(|| format!("Quantité invalide à la ligne {line}."))?;
        let name = record.get(name).unwrap_or_default();
        if deck_name.is_empty() || name.is_empty() {
            return Err(format!(
                "Nom du deck ou de la carte manquant à la ligne {line}."
            ));
        }
        let board = match value("Board").to_ascii_lowercase().as_str() {
            "" | "main deck" => "main".to_owned(),
            "commander" => "commander".to_owned(),
            "sideboard" => "sideboard".to_owned(),
            "considering" | "maybeboard" | "maybe board" => "considering".to_owned(),
            _ => value("Board").to_owned(),
        };
        let key = PrintingKey::new(value("Set Code"), value("Card Number"));
        let deck = decks.entry(deck_name.to_owned()).or_insert_with(|| Deck {
            name: deck_name.to_owned(),
            format,
            ..Deck::default()
        });
        if deck.format != format {
            return Err(format!(
                "Formats différents pour le deck « {deck_name} » à la ligne {line}."
            ));
        }
        deck.cards.push(DeckCard {
            name: name.to_owned(),
            quantity,
            board,
            set_code: key.set_code().to_owned(),
            collector_number: key.collector_number().to_owned(),
            printing: value("Printing").to_owned(),
            language: value("Language").to_owned(),
            card_type: value("Card Type").to_owned(),
            mana_cost: value("Mana Cost").to_owned(),
        });
    }

    if decks.is_empty() {
        return Err("Aucun deck Magic trouvé dans ce fichier.".to_owned());
    }
    decks
        .into_values()
        .map(|mut deck| {
            deck.validate()?;
            Ok(deck)
        })
        .collect()
}
