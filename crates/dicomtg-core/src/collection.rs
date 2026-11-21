use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt,
};

use crate::{
    CardCandidate, ImportIssue, ImportReport, ImportedCard, MatchResult, PrintingKey,
    match_card_candidates,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CardFinish {
    Normal,
    Foil,
    Etched,
}

impl CardFinish {
    #[must_use]
    pub fn from_printing(printing: Option<&str>) -> Self {
        let printing = printing.unwrap_or_default().to_ascii_lowercase();
        if printing.contains("etched") {
            Self::Etched
        } else if printing.contains("foil") {
            Self::Foil
        } else {
            Self::Normal
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Foil => "foil",
            Self::Etched => "etched",
        }
    }

    pub(crate) fn from_storage(value: &str) -> Option<Self> {
        match value {
            "normal" => Some(Self::Normal),
            "foil" => Some(Self::Foil),
            "etched" => Some(Self::Etched),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedPrinting {
    uuid: String,
    finish: CardFinish,
    quantity: u32,
}

impl OwnedPrinting {
    pub(crate) fn new(uuid: String, finish: CardFinish, quantity: u32) -> Self {
        Self {
            uuid,
            finish,
            quantity,
        }
    }

    #[must_use]
    pub fn uuid(&self) -> &str {
        &self.uuid
    }

    #[must_use]
    pub fn finish(&self) -> CardFinish {
        self.finish
    }

    #[must_use]
    pub fn quantity(&self) -> u32 {
        self.quantity
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedCard {
    card: ImportedCard,
    match_result: MatchResult,
}

impl UnresolvedCard {
    pub(crate) fn new(card: ImportedCard, match_result: MatchResult) -> Self {
        Self { card, match_result }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        self.card.name()
    }

    #[must_use]
    pub fn quantity(&self) -> u32 {
        self.card.quantity()
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

    #[must_use]
    pub fn match_result(&self) -> &MatchResult {
        &self.match_result
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionSnapshot {
    items: Vec<OwnedPrinting>,
    unresolved: Vec<UnresolvedCard>,
    import_issues: Vec<ImportIssue>,
}

impl CollectionSnapshot {
    pub(crate) fn new(
        items: Vec<OwnedPrinting>,
        unresolved: Vec<UnresolvedCard>,
        import_issues: Vec<ImportIssue>,
    ) -> Self {
        Self {
            items,
            unresolved,
            import_issues,
        }
    }

    #[must_use]
    pub fn items(&self) -> &[OwnedPrinting] {
        &self.items
    }

    #[must_use]
    pub fn unresolved(&self) -> &[UnresolvedCard] {
        &self.unresolved
    }

    #[must_use]
    pub fn import_issues(&self) -> &[ImportIssue] {
        &self.import_issues
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionBuildError {
    QuantityOverflow { uuid: String },
}

impl fmt::Display for CollectionBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QuantityOverflow { uuid } => {
                write!(formatter, "quantity overflow for card {uuid}")
            }
        }
    }
}

impl Error for CollectionBuildError {}

pub fn build_collection(
    report: ImportReport,
    catalog: &[CardCandidate],
) -> Result<CollectionSnapshot, CollectionBuildError> {
    let catalog = index_catalog(catalog);
    let (cards, import_issues) = report.into_parts();
    let mut items: Vec<OwnedPrinting> = Vec::new();
    let mut item_positions = HashMap::<(String, CardFinish), usize>::new();
    let mut unresolved = Vec::new();

    for card in cards {
        let candidates = catalog.get(card.key()).map_or(&[][..], Vec::as_slice);
        match match_card_candidates(card.card(), candidates) {
            MatchResult::Matched { uuid } => {
                let finish = owned_finish(&card, candidates, &uuid);
                add_owned_printing(
                    &mut items,
                    &mut item_positions,
                    uuid,
                    finish,
                    card.quantity(),
                )?;
            }
            MatchResult::Unmatched => unresolved.push(UnresolvedCard {
                card,
                match_result: MatchResult::Unmatched,
            }),
            MatchResult::Ambiguous { candidate_uuids } => unresolved.push(UnresolvedCard {
                card,
                match_result: MatchResult::Ambiguous { candidate_uuids },
            }),
        }
    }

    Ok(CollectionSnapshot {
        items,
        unresolved,
        import_issues,
    })
}

pub(crate) fn merge_collections(
    collections: Vec<CollectionSnapshot>,
) -> Result<CollectionSnapshot, CollectionBuildError> {
    let mut items = Vec::new();
    let mut item_positions = HashMap::new();
    let mut unresolved: Vec<UnresolvedCard> = Vec::new();
    let mut import_issues = Vec::new();

    for collection in collections {
        for item in collection.items {
            add_owned_printing(
                &mut items,
                &mut item_positions,
                item.uuid,
                item.finish,
                item.quantity,
            )?;
        }
        for card in collection.unresolved {
            if let Some(existing) = unresolved.iter_mut().find(|existing| {
                existing.card.card() == card.card.card()
                    && existing.match_result == card.match_result
            }) {
                let quantity = existing
                    .card
                    .quantity()
                    .checked_add(card.card.quantity())
                    .ok_or(CollectionBuildError::QuantityOverflow {
                        uuid: card.card.name().to_owned(),
                    })?;
                existing.card = ImportedCard::new(quantity, existing.card.card().clone());
            } else {
                unresolved.push(card);
            }
        }
        import_issues.extend(collection.import_issues);
    }

    Ok(CollectionSnapshot::new(items, unresolved, import_issues))
}

pub fn rematch_collection(
    mut collection: CollectionSnapshot,
    catalog: &[CardCandidate],
) -> Result<CollectionSnapshot, CollectionBuildError> {
    let candidates_by_uuid = catalog
        .iter()
        .map(|candidate| (candidate.uuid.as_str(), candidate))
        .collect::<HashMap<_, _>>();
    let mut refreshed_items = Vec::new();
    let mut item_positions = HashMap::new();
    for item in std::mem::take(&mut collection.items) {
        let finish = candidates_by_uuid
            .get(item.uuid())
            .and_then(|candidate| corrected_catalog_finish(item.finish, candidate))
            .unwrap_or(item.finish);
        add_owned_printing(
            &mut refreshed_items,
            &mut item_positions,
            item.uuid,
            finish,
            item.quantity,
        )?;
    }
    collection.items = refreshed_items;
    let catalog = index_catalog(catalog);
    let mut unresolved = Vec::new();

    for unresolved_card in std::mem::take(&mut collection.unresolved) {
        let card = unresolved_card.card;
        let candidates = catalog.get(card.key()).map_or(&[][..], Vec::as_slice);
        match match_card_candidates(card.card(), candidates) {
            MatchResult::Matched { uuid } => {
                let finish = owned_finish(&card, candidates, &uuid);
                add_owned_printing(
                    &mut collection.items,
                    &mut item_positions,
                    uuid,
                    finish,
                    card.quantity(),
                )?;
            }
            match_result => unresolved.push(UnresolvedCard { card, match_result }),
        }
    }
    collection.unresolved = unresolved;
    Ok(collection)
}

fn owned_finish(card: &ImportedCard, candidates: &[&CardCandidate], uuid: &str) -> CardFinish {
    if card.printing().is_some() {
        return CardFinish::from_printing(card.printing());
    }
    let Some(candidate) = candidates.iter().find(|candidate| candidate.uuid == uuid) else {
        return CardFinish::Normal;
    };
    only_catalog_finish(candidate).unwrap_or(CardFinish::Normal)
}

fn corrected_catalog_finish(current: CardFinish, candidate: &CardCandidate) -> Option<CardFinish> {
    let finishes = catalog_finishes(candidate);
    if finishes.contains(&current) {
        None
    } else {
        finishes
            .iter()
            .copied()
            .next()
            .filter(|_| finishes.len() == 1)
    }
}

fn only_catalog_finish(candidate: &CardCandidate) -> Option<CardFinish> {
    let finishes = catalog_finishes(candidate);
    finishes
        .iter()
        .copied()
        .next()
        .filter(|_| finishes.len() == 1)
}

fn catalog_finishes(candidate: &CardCandidate) -> HashSet<CardFinish> {
    candidate
        .finishes
        .iter()
        .filter_map(|finish| match finish.to_ascii_lowercase().as_str() {
            "nonfoil" => Some(CardFinish::Normal),
            "foil" => Some(CardFinish::Foil),
            "etched" => Some(CardFinish::Etched),
            _ => None,
        })
        .collect()
}

fn add_owned_printing(
    items: &mut Vec<OwnedPrinting>,
    item_positions: &mut HashMap<(String, CardFinish), usize>,
    uuid: String,
    finish: CardFinish,
    quantity: u32,
) -> Result<(), CollectionBuildError> {
    let key = (uuid.clone(), finish);
    if let Some(position) = item_positions.get(&key).copied() {
        items[position].quantity = items[position]
            .quantity
            .checked_add(quantity)
            .ok_or(CollectionBuildError::QuantityOverflow { uuid })?;
    } else {
        item_positions.insert(key, items.len());
        items.push(OwnedPrinting {
            uuid,
            finish,
            quantity,
        });
    }
    Ok(())
}

fn index_catalog(catalog: &[CardCandidate]) -> HashMap<PrintingKey, Vec<&CardCandidate>> {
    let mut index = HashMap::new();
    for candidate in catalog {
        index
            .entry(candidate.key().clone())
            .or_insert_with(Vec::new)
            .push(candidate);
    }
    index
}
