use std::{collections::HashMap, error::Error, fmt};

use crate::{CardFinish, CatalogCard, CollectionSnapshot, MatchResult, ReferencePrice};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CollectionQuery {
    search: String,
    colors: Vec<String>,
    rarities: Vec<String>,
    types: Vec<String>,
}

impl CollectionQuery {
    #[must_use]
    pub fn new(search: &str, colors: Vec<String>, rarities: Vec<String>) -> Self {
        Self {
            search: search.to_owned(),
            colors,
            rarities,
            types: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_types(mut self, types: Vec<String>) -> Self {
        self.types = types;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionItemStatus {
    Matched,
    Unmatched,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CollectionListItem {
    uuid: Option<String>,
    scryfall_id: Option<String>,
    has_back_face: bool,
    name: String,
    set_code: String,
    collector_number: String,
    quantity: u32,
    finish: CardFinish,
    rarity: Option<String>,
    mana_value: Option<f64>,
    colors: Vec<String>,
    color_identity: Vec<String>,
    types: Vec<String>,
    keywords: Vec<String>,
    finishes: Vec<String>,
    language: Option<String>,
    mana_cost: Option<String>,
    power: Option<String>,
    toughness: Option<String>,
    type_line: Option<String>,
    rules_text: Option<String>,
    price_cents: Option<u64>,
    normal_price_cents: Option<u64>,
    foil_price_cents: Option<u64>,
    price_date: Option<String>,
    status: CollectionItemStatus,
}

impl CollectionListItem {
    #[must_use]
    pub fn uuid(&self) -> Option<&str> {
        self.uuid.as_deref()
    }

    #[must_use]
    pub fn scryfall_id(&self) -> Option<&str> {
        self.scryfall_id.as_deref()
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
    pub fn set_code(&self) -> &str {
        &self.set_code
    }

    #[must_use]
    pub fn collector_number(&self) -> &str {
        &self.collector_number
    }

    #[must_use]
    pub fn quantity(&self) -> u32 {
        self.quantity
    }

    #[must_use]
    pub fn finish(&self) -> CardFinish {
        self.finish
    }

    #[must_use]
    pub fn rarity(&self) -> Option<&str> {
        self.rarity.as_deref()
    }

    #[must_use]
    pub fn mana_value(&self) -> Option<f64> {
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
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
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
    pub fn type_line(&self) -> Option<&str> {
        self.type_line.as_deref()
    }

    #[must_use]
    pub fn rules_text(&self) -> Option<&str> {
        self.rules_text.as_deref()
    }

    #[must_use]
    pub fn price_cents(&self) -> Option<u64> {
        self.price_cents
    }

    #[must_use]
    pub fn normal_price_cents(&self) -> Option<u64> {
        self.normal_price_cents
    }

    #[must_use]
    pub fn foil_price_cents(&self) -> Option<u64> {
        self.foil_price_cents
    }

    #[must_use]
    pub fn price_date(&self) -> Option<&str> {
        self.price_date.as_deref()
    }

    #[must_use]
    pub fn status(&self) -> CollectionItemStatus {
        self.status
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionQueryError {
    MissingCatalogCard { uuid: String },
}

impl fmt::Display for CollectionQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCatalogCard { uuid } => {
                write!(formatter, "owned card {uuid} is missing from the catalog")
            }
        }
    }
}

impl Error for CollectionQueryError {}

pub fn query_collection(
    collection: &CollectionSnapshot,
    catalog: &[CatalogCard],
    prices: &[ReferencePrice],
    query: &CollectionQuery,
) -> Result<Vec<CollectionListItem>, CollectionQueryError> {
    let catalog = catalog
        .iter()
        .map(|card| (card.uuid(), card))
        .collect::<HashMap<_, _>>();
    let prices = prices
        .iter()
        .map(|price| ((price.uuid(), price.finish()), price))
        .collect::<HashMap<_, _>>();
    let search = query.search.trim().to_lowercase();
    let mut result = Vec::new();

    for item in collection.items() {
        let card = catalog.get(item.uuid()).copied().ok_or_else(|| {
            CollectionQueryError::MissingCatalogCard {
                uuid: item.uuid().to_owned(),
            }
        })?;
        if !matches_search(card.name(), &search)
            || !matches_any(&query.colors, card.colors())
            || !matches_value(&query.rarities, card.rarity())
            || !matches_any(&query.types, card.types())
        {
            continue;
        }
        let price = prices.get(&(item.uuid(), item.finish())).copied();
        let normal_price = prices.get(&(item.uuid(), CardFinish::Normal)).copied();
        let foil_price = prices
            .get(&(item.uuid(), CardFinish::Foil))
            .copied()
            .or_else(|| prices.get(&(item.uuid(), CardFinish::Etched)).copied());
        result.push(CollectionListItem {
            uuid: Some(item.uuid().to_owned()),
            scryfall_id: card.scryfall_id().map(str::to_owned),
            has_back_face: card.has_back_face(),
            name: card.name().to_owned(),
            set_code: card.key().set_code().to_owned(),
            collector_number: card.key().collector_number().to_owned(),
            quantity: item.quantity(),
            finish: item.finish(),
            rarity: Some(card.rarity().to_owned()),
            mana_value: Some(card.mana_value()),
            colors: card.colors().to_vec(),
            color_identity: card.color_identity().to_vec(),
            types: card.types().to_vec(),
            keywords: card.keywords().to_vec(),
            finishes: card.finishes().to_vec(),
            language: Some(card.language().to_owned()),
            mana_cost: card.mana_cost().map(str::to_owned),
            power: card.power().map(str::to_owned),
            toughness: card.toughness().map(str::to_owned),
            type_line: Some(card.type_line().to_owned()),
            rules_text: card.rules_text().map(str::to_owned),
            price_cents: price.map(ReferencePrice::amount_cents),
            normal_price_cents: normal_price.map(ReferencePrice::amount_cents),
            foil_price_cents: foil_price.map(ReferencePrice::amount_cents),
            price_date: price.map(|price| price.as_of_date().to_owned()),
            status: CollectionItemStatus::Matched,
        });
    }

    if query.colors.is_empty() && query.rarities.is_empty() && query.types.is_empty() {
        for unresolved in collection.unresolved() {
            if !matches_search(unresolved.name(), &search) {
                continue;
            }
            let status = match unresolved.match_result() {
                MatchResult::Unmatched => CollectionItemStatus::Unmatched,
                MatchResult::Ambiguous { .. } => CollectionItemStatus::Ambiguous,
                MatchResult::Matched { .. } => CollectionItemStatus::Matched,
            };
            result.push(CollectionListItem {
                uuid: None,
                scryfall_id: None,
                has_back_face: false,
                name: unresolved.name().to_owned(),
                set_code: unresolved.key().set_code().to_owned(),
                collector_number: unresolved.key().collector_number().to_owned(),
                quantity: unresolved.quantity(),
                finish: CardFinish::from_printing(unresolved.printing()),
                rarity: None,
                mana_value: None,
                colors: Vec::new(),
                color_identity: Vec::new(),
                types: Vec::new(),
                keywords: Vec::new(),
                finishes: Vec::new(),
                language: None,
                mana_cost: None,
                power: None,
                toughness: None,
                type_line: None,
                rules_text: None,
                price_cents: None,
                normal_price_cents: None,
                foil_price_cents: None,
                price_date: None,
                status,
            });
        }
    }

    Ok(result)
}

#[must_use]
pub fn most_expensive_card(items: &[CollectionListItem]) -> Option<&CollectionListItem> {
    items
        .iter()
        .filter(|item| item.status == CollectionItemStatus::Matched)
        .filter(|item| item.price_cents.is_some())
        .max_by_key(|item| item.price_cents)
}

fn matches_search(name: &str, search: &str) -> bool {
    search.is_empty() || name.to_lowercase().contains(search)
}

fn matches_any(selected: &[String], values: &[String]) -> bool {
    selected.is_empty()
        || selected.iter().any(|selected| {
            values
                .iter()
                .any(|value| value.eq_ignore_ascii_case(selected))
        })
}

fn matches_value(selected: &[String], value: &str) -> bool {
    selected.is_empty()
        || selected
            .iter()
            .any(|selected| value.eq_ignore_ascii_case(selected))
}
