mod analytics;
mod card_image;
mod card_localization;
mod catalog;
mod catalog_update;
mod collection;
mod collection_download;
mod collection_import;
mod collection_query;
mod decks;
mod game_localization;
mod mtgjson_update;
mod price;
mod price_update;
mod storage;

pub use analytics::{
    AnalyticsError, CollectionAnalytics, DistributionEntry, analyze_collection,
    analyze_collection_by_type,
};
pub use card_image::{
    CARD_IMAGE_CACHE_CAPACITY_BYTES, CardImageCache, CardImageCacheStats, CardImageError, CardImageFace,
    CardImageSize, card_image_cache_stats, clear_card_image_cache, load_card_image,
};
pub use card_localization::{
    CardLegalities, CardLocalization, CardLocalizationError, RelatedToken, find_card_french_text,
    find_card_text_for_language, find_english_related_tokens, load_card_localization,
    load_card_localization_for_language,
};
pub use card_localization::{DeckCardLegality, DeckLegalityTarget, load_deck_legalities};
pub use decks::{Deck, DeckCard, DeckFormat, SavedDeck, import_decks_csv};
pub use game_localization::{LocalizedCardNames, LocalizedSetNames, update_game_localization};
pub use storage::decks::{DeckCardInfo, DeckWorkspace};
pub use storage::game_localization::GameLocalizations;
pub use collection::{
    CardFinish, CollectionBuildError, CollectionSnapshot, OwnedPrinting, UnresolvedCard,
    build_collection, rematch_collection,
};
pub use collection_download::{
    CollectionDownloadError, download_collection_csv, google_drive_source_url,
};
pub use collection_import::import_collection_csv as import_dragonshield_csv;
pub use collection_import::{
    ImportError, ImportIssue, ImportReport, ImportedCard, RowIssueReason, import_collection_csv,
};
pub use collection_query::{
    CollectionItemStatus, CollectionListItem, CollectionQuery, CollectionQueryError,
    most_expensive_card, query_collection,
};
pub use price::{
    PriceImport, PriceImportError, ReferencePrice, import_cardmarket_price_guide,
    import_cardmarket_price_guide_with_products,
    import_cardmarket_price_guide_with_products_and_sets, import_mtgjson_prices_json,
    is_valid_iso_date,
};
pub use price_update::{
    PriceUpdateError, PriceUpdateSummary, initialize_set_release_dates, install_cardmarket_data,
    install_cardmarket_price_guide, update_prices_from_cardmarket,
};
pub use storage::{CardEdhrecStats, CollectionStore, StorageError, StoredCollection};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PrintingKey {
    set_code: String,
    collector_number: String,
}

impl PrintingKey {
    #[must_use]
    pub fn new(set_code: &str, collector_number: &str) -> Self {
        let set_code = set_code.trim().to_ascii_uppercase();
        let set_code = match set_code.as_str() {
            "LEGI" => "LEG".to_owned(),
            "DRKI" => "DRK".to_owned(),
            _ => set_code,
        };
        let collector_number = normalize_collector_number(&set_code, collector_number.trim());
        Self {
            set_code,
            collector_number,
        }
    }

    #[must_use]
    pub fn set_code(&self) -> &str {
        &self.set_code
    }

    #[must_use]
    pub fn collector_number(&self) -> &str {
        &self.collector_number
    }
}

fn normalize_collector_number(set_code: &str, collector_number: &str) -> String {
    if set_code == "PLST" && collector_number == "854" {
        return "M10-128".to_owned();
    }
    if set_code == "ARN"
        && let Some(number) = collector_number
            .strip_suffix('a')
            .or_else(|| collector_number.strip_suffix('A'))
        && !number.is_empty()
        && number.bytes().all(|byte| byte.is_ascii_digit())
    {
        return format!("{number}†");
    }
    collector_number.to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionCard {
    name: String,
    normalized_name: String,
    key: PrintingKey,
    language: Option<String>,
    printing: Option<String>,
}

impl CollectionCard {
    #[must_use]
    pub fn new(name: &str, set_code: &str, collector_number: &str) -> Self {
        Self::new_with_details(name, set_code, collector_number, None, None)
    }

    #[must_use]
    pub fn with_language(
        name: &str,
        set_code: &str,
        collector_number: &str,
        language: &str,
    ) -> Self {
        Self::new_with_details(name, set_code, collector_number, Some(language), None)
    }

    pub(crate) fn new_with_details(
        name: &str,
        set_code: &str,
        collector_number: &str,
        language: Option<&str>,
        printing: Option<&str>,
    ) -> Self {
        let name = name.trim().to_owned();
        Self {
            normalized_name: normalize_name(&name),
            name,
            key: PrintingKey::new(set_code, collector_number),
            language: trimmed_optional(language),
            printing: trimmed_optional(printing),
        }
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
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    #[must_use]
    pub fn printing(&self) -> Option<&str> {
        self.printing.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardCandidate {
    uuid: String,
    normalized_name: String,
    key: PrintingKey,
    language: Option<String>,
    finishes: Vec<String>,
}

impl CardCandidate {
    #[must_use]
    pub fn new(uuid: &str, name: &str, set_code: &str, collector_number: &str) -> Self {
        Self::new_with_details(uuid, name, set_code, collector_number, None, &[])
    }

    #[must_use]
    pub fn with_language(
        uuid: &str,
        name: &str,
        set_code: &str,
        collector_number: &str,
        language: &str,
    ) -> Self {
        Self::new_with_details(uuid, name, set_code, collector_number, Some(language), &[])
    }

    pub(crate) fn new_with_details(
        uuid: &str,
        name: &str,
        set_code: &str,
        collector_number: &str,
        language: Option<&str>,
        finishes: &[String],
    ) -> Self {
        Self {
            uuid: uuid.to_owned(),
            normalized_name: normalize_name(name),
            key: PrintingKey::new(set_code, collector_number),
            language: trimmed_optional(language),
            finishes: finishes.to_vec(),
        }
    }

    pub(crate) fn key(&self) -> &PrintingKey {
        &self.key
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchResult {
    Matched { uuid: String },
    Unmatched,
    Ambiguous { candidate_uuids: Vec<String> },
}

#[must_use]
pub fn match_card(owned: &CollectionCard, catalog: &[CardCandidate]) -> MatchResult {
    let key_matches: Vec<_> = catalog
        .iter()
        .filter(|candidate| candidate.key == owned.key)
        .collect();

    match_card_candidates(owned, &key_matches)
}

pub(crate) fn match_card_candidates(
    owned: &CollectionCard,
    candidates: &[&CardCandidate],
) -> MatchResult {
    match candidates {
        [] => MatchResult::Unmatched,
        [candidate] => matched(candidate),
        candidates => {
            let name_matches: Vec<_> = candidates
                .iter()
                .copied()
                .filter(|candidate| candidate.normalized_name == owned.normalized_name)
                .collect();

            match name_matches.as_slice() {
                [candidate] => matched(candidate),
                [] => ambiguous(candidates),
                candidates => match_discriminators(owned, candidates),
            }
        }
    }
}

fn normalize_name(name: &str) -> String {
    name.trim().to_lowercase()
}

fn trimmed_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn match_discriminators(owned: &CollectionCard, candidates: &[&CardCandidate]) -> MatchResult {
    let language_matches = owned.language.as_ref().map(|language| {
        candidates
            .iter()
            .copied()
            .filter(|candidate| {
                candidate
                    .language
                    .as_deref()
                    .is_some_and(|candidate_language| {
                        candidate_language.eq_ignore_ascii_case(language)
                    })
            })
            .collect::<Vec<_>>()
    });
    let candidates = match language_matches.as_deref() {
        Some([]) => return ambiguous(candidates),
        Some(candidates) => candidates,
        None => candidates,
    };

    match candidates {
        [candidate] => matched(candidate),
        candidates => match_printing(owned, candidates),
    }
}

fn match_printing(owned: &CollectionCard, candidates: &[&CardCandidate]) -> MatchResult {
    let Some(finish) = owned.printing.as_deref().and_then(mtgjson_finish) else {
        return ambiguous(candidates);
    };
    let finish_matches = candidates
        .iter()
        .copied()
        .filter(|candidate| {
            candidate
                .finishes
                .iter()
                .any(|candidate_finish| candidate_finish.eq_ignore_ascii_case(finish))
        })
        .collect::<Vec<_>>();

    match finish_matches.as_slice() {
        [candidate] => matched(candidate),
        [] => ambiguous(candidates),
        candidates => ambiguous(candidates),
    }
}

fn mtgjson_finish(printing: &str) -> Option<&'static str> {
    let printing = printing.trim().to_ascii_lowercase();
    if printing == "normal" {
        Some("nonfoil")
    } else if printing.contains("etched") {
        Some("etched")
    } else if printing.ends_with("foil") {
        Some("foil")
    } else {
        None
    }
}

fn matched(candidate: &CardCandidate) -> MatchResult {
    MatchResult::Matched {
        uuid: candidate.uuid.clone(),
    }
}

fn ambiguous(candidates: &[&CardCandidate]) -> MatchResult {
    MatchResult::Ambiguous {
        candidate_uuids: candidates
            .iter()
            .map(|candidate| candidate.uuid.clone())
            .collect(),
    }
}
pub use catalog::{
    CatalogCard, CatalogImport, CatalogImportError, CatalogRowError, import_mtgjson_cards_csv,
    import_mtgjson_tokens_csv,
};
pub use catalog_update::{
    CatalogUpdateError, CatalogUpdateSummary, install_catalog_archive, install_catalog_archives,
    install_catalog_archives_with_identifiers, update_catalog_from_mtgjson,
};
