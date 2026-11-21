use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt,
};

use crate::{CatalogCard, CollectionSnapshot, ReferencePrice};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributionEntry {
    label: String,
    quantity: u64,
}

impl DistributionEntry {
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    #[must_use]
    pub fn quantity(&self) -> u64 {
        self.quantity
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionAnalytics {
    total_quantity: u64,
    distinct_names: usize,
    total_value_cents: u64,
    matched_printings: usize,
    priced_printings: usize,
    colors: Vec<DistributionEntry>,
    types: Vec<DistributionEntry>,
    keywords: Vec<DistributionEntry>,
    sets: Vec<DistributionEntry>,
    rarities: Vec<DistributionEntry>,
    mana_values: Vec<DistributionEntry>,
}

impl CollectionAnalytics {
    #[must_use]
    pub fn total_quantity(&self) -> u64 {
        self.total_quantity
    }

    #[must_use]
    pub fn distinct_names(&self) -> usize {
        self.distinct_names
    }

    #[must_use]
    pub fn total_value_cents(&self) -> u64 {
        self.total_value_cents
    }

    #[must_use]
    pub fn matched_printings(&self) -> usize {
        self.matched_printings
    }

    #[must_use]
    pub fn priced_printings(&self) -> usize {
        self.priced_printings
    }

    #[must_use]
    pub fn unpriced_printings(&self) -> usize {
        self.matched_printings - self.priced_printings
    }

    #[must_use]
    pub fn price_coverage_basis_points(&self) -> u16 {
        if self.matched_printings == 0 {
            0
        } else {
            ((self.priced_printings as u128 * 10_000) / self.matched_printings as u128) as u16
        }
    }

    #[must_use]
    pub fn colors(&self) -> &[DistributionEntry] {
        &self.colors
    }

    #[must_use]
    pub fn types(&self) -> &[DistributionEntry] {
        &self.types
    }

    #[must_use]
    pub fn keywords(&self) -> &[DistributionEntry] {
        &self.keywords
    }

    #[must_use]
    pub fn sets(&self) -> &[DistributionEntry] {
        &self.sets
    }

    #[must_use]
    pub fn rarities(&self) -> &[DistributionEntry] {
        &self.rarities
    }

    #[must_use]
    pub fn mana_values(&self) -> &[DistributionEntry] {
        &self.mana_values
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalyticsError {
    ValueOverflow { uuid: String },
}

impl fmt::Display for AnalyticsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValueOverflow { uuid } => {
                write!(formatter, "collection value overflow for card {uuid}")
            }
        }
    }
}

impl Error for AnalyticsError {}

pub fn analyze_collection(
    collection: &CollectionSnapshot,
    catalog: &[CatalogCard],
    prices: &[ReferencePrice],
) -> Result<CollectionAnalytics, AnalyticsError> {
    analyze_collection_filtered(collection, catalog, prices, None)
}

pub fn analyze_collection_by_type(
    collection: &CollectionSnapshot,
    catalog: &[CatalogCard],
    prices: &[ReferencePrice],
    card_type: &str,
) -> Result<CollectionAnalytics, AnalyticsError> {
    analyze_collection_filtered(collection, catalog, prices, Some(card_type))
}

fn analyze_collection_filtered(
    collection: &CollectionSnapshot,
    catalog: &[CatalogCard],
    prices: &[ReferencePrice],
    card_type: Option<&str>,
) -> Result<CollectionAnalytics, AnalyticsError> {
    let price_by_printing = prices
        .iter()
        .map(|price| ((price.uuid(), price.finish()), price.amount_cents()))
        .collect::<HashMap<_, _>>();
    let catalog_by_uuid = catalog
        .iter()
        .map(|card| (card.uuid(), card))
        .collect::<HashMap<_, _>>();
    let mut total_quantity = 0;
    let mut distinct_names = HashSet::new();
    let mut total_value_cents = 0_u64;
    let mut matched_printings = 0;
    let mut priced_printings = 0;
    let mut colors = HashMap::new();
    let mut types = HashMap::new();
    let mut keywords = HashMap::new();
    let mut sets = HashMap::new();
    let mut rarities = HashMap::new();
    let mut mana_values = HashMap::new();

    if card_type.is_none() {
        for card in collection.unresolved() {
            total_quantity += u64::from(card.quantity());
            distinct_names.insert(card.name().to_lowercase());
        }
    }

    for item in collection.items() {
        let card = catalog_by_uuid.get(item.uuid()).copied();
        if let Some(selected_type) = card_type {
            let matches = card.is_some_and(|card| {
                card.types()
                    .iter()
                    .any(|card_type| card_type.eq_ignore_ascii_case(selected_type))
            });
            if !matches {
                continue;
            }
        }

        let quantity = u64::from(item.quantity());
        total_quantity += quantity;
        matched_printings += 1;
        if let Some(card) = card {
            distinct_names.insert(card.name().to_lowercase());
            if card.colors().is_empty() {
                add_quantity(&mut colors, "C", quantity);
            } else {
                add_quantities(&mut colors, card.colors(), quantity);
            }
            add_quantities(&mut types, card.types(), quantity);
            add_quantities(&mut keywords, card.keywords(), quantity);
            add_quantity(&mut sets, card.key().set_code(), quantity);
            add_quantity(&mut rarities, card.rarity(), quantity);
            add_quantity(&mut mana_values, &card.mana_value().to_string(), quantity);
        }

        if let Some(amount_cents) = price_by_printing
            .get(&(item.uuid(), item.finish()))
            .copied()
        {
            priced_printings += 1;
            let item_value = amount_cents.checked_mul(quantity).ok_or_else(|| {
                AnalyticsError::ValueOverflow {
                    uuid: item.uuid().to_owned(),
                }
            })?;
            total_value_cents = total_value_cents.checked_add(item_value).ok_or_else(|| {
                AnalyticsError::ValueOverflow {
                    uuid: item.uuid().to_owned(),
                }
            })?;
        }
    }

    Ok(CollectionAnalytics {
        total_quantity,
        distinct_names: distinct_names.len(),
        total_value_cents,
        matched_printings,
        priced_printings,
        colors: ranked_distribution(colors),
        types: ranked_distribution(types),
        keywords: ranked_distribution(keywords),
        sets: ranked_distribution(sets),
        rarities: ranked_distribution(rarities),
        mana_values: ranked_distribution(mana_values),
    })
}

fn add_quantities(distribution: &mut HashMap<String, u64>, labels: &[String], quantity: u64) {
    for label in labels {
        add_quantity(distribution, label, quantity);
    }
}

fn add_quantity(distribution: &mut HashMap<String, u64>, label: &str, quantity: u64) {
    *distribution.entry(label.to_owned()).or_default() += quantity;
}

fn ranked_distribution(distribution: HashMap<String, u64>) -> Vec<DistributionEntry> {
    let mut entries = distribution
        .into_iter()
        .map(|(label, quantity)| DistributionEntry { label, quantity })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        right
            .quantity
            .cmp(&left.quantity)
            .then_with(|| left.label.cmp(&right.label))
    });
    entries
}
