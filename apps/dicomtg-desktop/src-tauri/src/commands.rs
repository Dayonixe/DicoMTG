use std::{
    collections::HashMap,
    io::Cursor,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use dicomtg_core::{
    CardImageCacheStats, CardImageFace, CardImageSize, CollectionItemStatus, CollectionListItem,
    CollectionQuery, CollectionSnapshot, CollectionStore, DistributionEntry, MatchResult,
    RowIssueReason, analyze_collection, analyze_collection_by_type, build_collection,
    card_image_cache_stats, clear_card_image_cache,
    download_collection_csv as fetch_collection_csv, import_dragonshield_csv, is_valid_iso_date,
    load_card_image, load_card_localization as fetch_card_localization, most_expensive_card,
    query_collection, rematch_collection, update_catalog_from_mtgjson,
    update_prices_from_cardmarket,
};
use serde::{Deserialize, Serialize};
use tauri::{State, ipc::Response};

pub struct AppState {
    store: Mutex<CollectionStore>,
    database_path: PathBuf,
    update_directory: PathBuf,
    image_directory: PathBuf,
    image_cache_lock: Mutex<()>,
    card_localizations: Mutex<HashMap<String, CardLocalizationDto>>,
    reference_update_running: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(
        store: CollectionStore,
        database_path: PathBuf,
        update_directory: PathBuf,
        image_directory: PathBuf,
    ) -> Self {
        Self {
            store: Mutex::new(store),
            database_path,
            update_directory,
            image_directory,
            image_cache_lock: Mutex::new(()),
            card_localizations: Mutex::new(HashMap::new()),
            reference_update_running: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DashboardQuery {
    search: String,
    colors: Vec<String>,
    rarities: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardDto {
    metrics: DashboardMetricsDto,
    distributions: DashboardDistributionsDto,
    most_expensive_card: Option<CollectionItemDto>,
    items: Vec<CollectionItemDto>,
    diagnostics: DiagnosticsDto,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeAnalysisDto {
    selected_type: String,
    metrics: TypeAnalysisMetricsDto,
    colors: Vec<DistributionEntryDto>,
    mana_values: Vec<DistributionEntryDto>,
    items: Vec<CollectionItemDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TypeAnalysisMetricsDto {
    total_quantity: u64,
    distinct_names: usize,
    total_value_cents: u64,
    matched_printings: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DashboardMetricsDto {
    total_quantity: u64,
    distinct_names: usize,
    total_value_cents: u64,
    catalog_cards: usize,
    reference_prices: usize,
    matched_printings: usize,
    priced_printings: usize,
    unpriced_printings: usize,
    price_coverage_basis_points: u16,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DashboardDistributionsDto {
    colors: Vec<DistributionEntryDto>,
    types: Vec<DistributionEntryDto>,
    keywords: Vec<DistributionEntryDto>,
    sets: Vec<DistributionEntryDto>,
    rarities: Vec<DistributionEntryDto>,
    mana_values: Vec<DistributionEntryDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DistributionEntryDto {
    label: String,
    quantity: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CollectionItemDto {
    uuid: Option<String>,
    scryfall_id: Option<String>,
    has_back_face: bool,
    name: String,
    set_code: String,
    collector_number: String,
    quantity: u32,
    finish: &'static str,
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
    status: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummaryDto {
    valid_rows: usize,
    invalid_rows: usize,
    matched_printings: usize,
    unmatched_rows: usize,
    ambiguous_rows: usize,
    total_quantity: u64,
    issues: Vec<ImportIssueDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedCollectionDto {
    id: i64,
    name: String,
    selected: bool,
    total_quantity: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportIssueDto {
    line: usize,
    reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticsDto {
    invalid_rows: Vec<InvalidRowDto>,
    unresolved_rows: Vec<UnresolvedRowDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InvalidRowDto {
    line: usize,
    reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UnresolvedRowDto {
    name: String,
    set_code: String,
    collector_number: String,
    quantity: u32,
    language: Option<String>,
    printing: Option<String>,
    status: &'static str,
    candidate_uuids: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceUpdateDto {
    status: &'static str,
    catalog_cards: usize,
    reference_prices: usize,
    update_date: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageCacheDto {
    image_count: usize,
    total_bytes: u64,
    capacity_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardLocalizationDto {
    set_name: String,
    french_text: Option<String>,
}

pub(crate) fn build_dashboard(
    store: &CollectionStore,
    query: DashboardQuery,
) -> Result<DashboardDto, String> {
    let collection = store.load_collection().map_err(|error| error.to_string())?;
    let catalog = store.load_catalog().map_err(|error| error.to_string())?;
    let prices = store.load_prices().map_err(|error| error.to_string())?;
    let metrics =
        analyze_collection(&collection, &catalog, &prices).map_err(|error| error.to_string())?;
    let diagnostics = build_diagnostics(&collection);
    let distributions = DashboardDistributionsDto {
        colors: distribution_entries(metrics.colors()),
        types: distribution_entries(metrics.types()),
        keywords: distribution_entries(metrics.keywords()),
        sets: distribution_entries(metrics.sets()),
        rarities: distribution_entries(metrics.rarities()),
        mana_values: distribution_entries(metrics.mana_values()),
    };
    let has_filters =
        !query.search.trim().is_empty() || !query.colors.is_empty() || !query.rarities.is_empty();
    let all_items = query_collection(&collection, &catalog, &prices, &CollectionQuery::default())
        .map_err(|error| error.to_string())?;
    let most_expensive_card = most_expensive_card(&all_items)
        .cloned()
        .map(collection_item_dto);
    let items = if has_filters {
        let query = CollectionQuery::new(&query.search, query.colors, query.rarities);
        query_collection(&collection, &catalog, &prices, &query)
            .map_err(|error| error.to_string())?
    } else {
        all_items
    }
    .into_iter()
    .map(collection_item_dto)
    .collect();

    Ok(DashboardDto {
        metrics: DashboardMetricsDto {
            total_quantity: metrics.total_quantity(),
            distinct_names: metrics.distinct_names(),
            total_value_cents: metrics.total_value_cents(),
            catalog_cards: catalog.len(),
            reference_prices: prices.len(),
            matched_printings: metrics.matched_printings(),
            priced_printings: metrics.priced_printings(),
            unpriced_printings: metrics.unpriced_printings(),
            price_coverage_basis_points: metrics.price_coverage_basis_points(),
        },
        distributions,
        most_expensive_card,
        items,
        diagnostics,
    })
}

pub(crate) fn build_type_analysis(
    store: &CollectionStore,
    card_type: &str,
) -> Result<TypeAnalysisDto, String> {
    let selected_type = card_type.trim();
    if selected_type.is_empty() {
        return Err("card type is required".to_owned());
    }

    let collection = store.load_collection().map_err(|error| error.to_string())?;
    let catalog = store.load_catalog().map_err(|error| error.to_string())?;
    let prices = store.load_prices().map_err(|error| error.to_string())?;
    let metrics = analyze_collection_by_type(&collection, &catalog, &prices, selected_type)
        .map_err(|error| error.to_string())?;
    let query = CollectionQuery::default().with_types(vec![selected_type.to_owned()]);
    let items = query_collection(&collection, &catalog, &prices, &query)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(collection_item_dto)
        .collect();

    Ok(TypeAnalysisDto {
        selected_type: selected_type.to_owned(),
        metrics: TypeAnalysisMetricsDto {
            total_quantity: metrics.total_quantity(),
            distinct_names: metrics.distinct_names(),
            total_value_cents: metrics.total_value_cents(),
            matched_printings: metrics.matched_printings(),
        },
        colors: distribution_entries(metrics.colors()),
        mana_values: distribution_entries(metrics.mana_values()),
        items,
    })
}

fn collection_item_dto(item: CollectionListItem) -> CollectionItemDto {
    CollectionItemDto {
        uuid: item.uuid().map(str::to_owned),
        scryfall_id: item.scryfall_id().map(str::to_owned),
        has_back_face: item.has_back_face(),
        name: item.name().to_owned(),
        set_code: item.set_code().to_owned(),
        collector_number: item.collector_number().to_owned(),
        quantity: item.quantity(),
        finish: item.finish().as_str(),
        rarity: item.rarity().map(str::to_owned),
        mana_value: item.mana_value(),
        colors: item.colors().to_vec(),
        color_identity: item.color_identity().to_vec(),
        types: item.types().to_vec(),
        keywords: item.keywords().to_vec(),
        finishes: item.finishes().to_vec(),
        language: item.language().map(str::to_owned),
        mana_cost: item.mana_cost().map(str::to_owned),
        power: item.power().map(str::to_owned),
        toughness: item.toughness().map(str::to_owned),
        type_line: item.type_line().map(str::to_owned),
        rules_text: item.rules_text().map(str::to_owned),
        price_cents: item.price_cents(),
        normal_price_cents: item.normal_price_cents(),
        foil_price_cents: item.foil_price_cents(),
        price_date: item.price_date().map(str::to_owned),
        status: match item.status() {
            CollectionItemStatus::Matched => "matched",
            CollectionItemStatus::Unmatched => "unmatched",
            CollectionItemStatus::Ambiguous => "ambiguous",
        },
    }
}

fn distribution_entries(entries: &[DistributionEntry]) -> Vec<DistributionEntryDto> {
    entries
        .iter()
        .map(|entry| DistributionEntryDto {
            label: entry.label().to_owned(),
            quantity: entry.quantity(),
        })
        .collect()
}

fn build_diagnostics(collection: &CollectionSnapshot) -> DiagnosticsDto {
    let invalid_rows = collection
        .import_issues()
        .iter()
        .map(|issue| InvalidRowDto {
            line: issue.line(),
            reason: import_issue_reason(issue.reason()),
        })
        .collect();
    let unresolved_rows = collection
        .unresolved()
        .iter()
        .map(|card| {
            let (status, candidate_uuids) = match card.match_result() {
                MatchResult::Matched { uuid } => ("matched", vec![uuid.clone()]),
                MatchResult::Unmatched => ("unmatched", Vec::new()),
                MatchResult::Ambiguous { candidate_uuids } => {
                    ("ambiguous", candidate_uuids.clone())
                }
            };
            UnresolvedRowDto {
                name: card.name().to_owned(),
                set_code: card.key().set_code().to_owned(),
                collector_number: card.key().collector_number().to_owned(),
                quantity: card.quantity(),
                language: card.language().map(str::to_owned),
                printing: card.printing().map(str::to_owned),
                status,
                candidate_uuids,
            }
        })
        .collect();

    DiagnosticsDto {
        invalid_rows,
        unresolved_rows,
    }
}

fn import_issue_reason(reason: &RowIssueReason) -> String {
    match reason {
        RowIssueReason::MissingField { field } => format!("Champ requis manquant : {field}"),
        RowIssueReason::InvalidQuantity { value } => format!("Quantité invalide : {value}"),
    }
}

pub(crate) fn import_collection_csv(
    store: &mut CollectionStore,
    csv: &str,
) -> Result<ImportSummaryDto, String> {
    let (collection, summary) = build_collection_import(store, csv)?;
    store
        .replace_collection(&collection)
        .map_err(|error| error.to_string())?;
    Ok(summary)
}

pub(crate) fn create_collection_csv(
    store: &mut CollectionStore,
    name: &str,
    csv: &str,
) -> Result<ImportSummaryDto, String> {
    let (collection, summary) = build_collection_import(store, csv)?;
    store
        .create_collection(name, &collection)
        .map_err(|error| error.to_string())?;
    Ok(summary)
}

pub(crate) fn update_collection_csv(
    store: &mut CollectionStore,
    collection_id: i64,
    csv: &str,
) -> Result<ImportSummaryDto, String> {
    let (collection, summary) = build_collection_import(store, csv)?;
    store
        .replace_collection_for(collection_id, &collection)
        .map_err(|error| error.to_string())?;
    Ok(summary)
}

fn build_collection_import(
    store: &CollectionStore,
    csv: &str,
) -> Result<(CollectionSnapshot, ImportSummaryDto), String> {
    let catalog = store.load_catalog().map_err(|error| error.to_string())?;
    let report =
        import_dragonshield_csv(Cursor::new(csv.as_bytes())).map_err(|error| error.to_string())?;
    let valid_rows = report.cards().len();
    let invalid_rows = report.issues().len();

    if valid_rows == 0 && invalid_rows > 0 {
        return Err("Import refusé : aucune ligne valide dans le fichier.".to_owned());
    }

    let issues = report
        .issues()
        .iter()
        .map(|issue| ImportIssueDto {
            line: issue.line(),
            reason: import_issue_reason(issue.reason()),
        })
        .collect();
    let candidates = catalog
        .iter()
        .map(|card| card.matching_candidate())
        .collect::<Vec<_>>();
    let collection = build_collection(report, &candidates).map_err(|error| error.to_string())?;
    let unmatched_rows = collection
        .unresolved()
        .iter()
        .filter(|card| matches!(card.match_result(), MatchResult::Unmatched))
        .count();
    let ambiguous_rows = collection
        .unresolved()
        .iter()
        .filter(|card| matches!(card.match_result(), MatchResult::Ambiguous { .. }))
        .count();
    let total_quantity = collection
        .items()
        .iter()
        .map(|card| u64::from(card.quantity()))
        .chain(
            collection
                .unresolved()
                .iter()
                .map(|card| u64::from(card.quantity())),
        )
        .sum();
    let matched_printings = collection.items().len();

    let summary = ImportSummaryDto {
        valid_rows,
        invalid_rows,
        matched_printings,
        unmatched_rows,
        ambiguous_rows,
        total_quantity,
        issues,
    };
    Ok((collection, summary))
}

fn managed_collections(store: &CollectionStore) -> Result<Vec<ManagedCollectionDto>, String> {
    store
        .collections()
        .map_err(|error| error.to_string())
        .map(|collections| {
            collections
                .into_iter()
                .map(|collection| ManagedCollectionDto {
                    id: collection.id(),
                    name: collection.name().to_owned(),
                    selected: collection.selected(),
                    total_quantity: collection.total_quantity(),
                })
                .collect()
        })
}

#[tauri::command(async)]
pub fn load_dashboard(
    state: State<'_, AppState>,
    query: DashboardQuery,
) -> Result<DashboardDto, String> {
    let store = state
        .store
        .lock()
        .map_err(|_| "local database lock is poisoned".to_owned())?;
    build_dashboard(&store, query)
}

#[tauri::command(async)]
pub fn load_type_analysis(
    state: State<'_, AppState>,
    card_type: String,
) -> Result<TypeAnalysisDto, String> {
    let store = state
        .store
        .lock()
        .map_err(|_| "local database lock is poisoned".to_owned())?;
    build_type_analysis(&store, &card_type)
}

#[tauri::command(async)]
pub fn import_collection(
    state: State<'_, AppState>,
    csv: String,
) -> Result<ImportSummaryDto, String> {
    let mut store = state
        .store
        .lock()
        .map_err(|_| "local database lock is poisoned".to_owned())?;
    import_collection_csv(&mut store, &csv)
}

#[tauri::command(async)]
pub fn list_collections(state: State<'_, AppState>) -> Result<Vec<ManagedCollectionDto>, String> {
    let store = state
        .store
        .lock()
        .map_err(|_| "local database lock is poisoned".to_owned())?;
    managed_collections(&store)
}

#[tauri::command(async)]
pub fn create_collection(
    state: State<'_, AppState>,
    name: String,
    csv: String,
) -> Result<ImportSummaryDto, String> {
    let mut store = state
        .store
        .lock()
        .map_err(|_| "local database lock is poisoned".to_owned())?;
    create_collection_csv(&mut store, &name, &csv)
}

#[tauri::command(async)]
pub fn update_collection(
    state: State<'_, AppState>,
    collection_id: i64,
    csv: String,
) -> Result<ImportSummaryDto, String> {
    let mut store = state
        .store
        .lock()
        .map_err(|_| "local database lock is poisoned".to_owned())?;
    update_collection_csv(&mut store, collection_id, &csv)
}

#[tauri::command(async)]
pub fn rename_collection(
    state: State<'_, AppState>,
    collection_id: i64,
    name: String,
) -> Result<(), String> {
    state
        .store
        .lock()
        .map_err(|_| "local database lock is poisoned".to_owned())?
        .rename_collection(collection_id, &name)
        .map_err(|error| error.to_string())
}

#[tauri::command(async)]
pub fn set_collection_selected(
    state: State<'_, AppState>,
    collection_id: i64,
    selected: bool,
) -> Result<(), String> {
    state
        .store
        .lock()
        .map_err(|_| "local database lock is poisoned".to_owned())?
        .set_collection_selected(collection_id, selected)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn download_collection_csv(url: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || fetch_collection_csv(&url))
        .await
        .map_err(|error| format!("Échec de la tâche de téléchargement : {error}"))?
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn update_reference_data(
    state: State<'_, AppState>,
    local_date: String,
    force: bool,
) -> Result<ReferenceUpdateDto, String> {
    if !valid_local_date(&local_date) {
        return Err("Date locale invalide".to_owned());
    }
    if state
        .reference_update_running
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Ok(ReferenceUpdateDto {
            status: "running",
            catalog_cards: 0,
            reference_prices: 0,
            update_date: local_date,
        });
    }

    let database_path = state.database_path.clone();
    let update_directory = state.update_directory.clone();
    let running = Arc::clone(&state.reference_update_running);
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = ReferenceUpdateGuard(running);
        refresh_reference_data(&database_path, &update_directory, &local_date, force)
    })
    .await
    .map_err(|error| format!("Échec de la tâche d’actualisation : {error}"))?
}

fn refresh_reference_data(
    database_path: &PathBuf,
    update_directory: &PathBuf,
    local_date: &str,
    force: bool,
) -> Result<ReferenceUpdateDto, String> {
    let mut store = CollectionStore::open(database_path).map_err(|error| error.to_string())?;
    if !reference_update_due(
        store
            .last_reference_update_date()
            .map_err(|error| error.to_string())?
            .as_deref(),
        local_date,
        force,
    ) {
        return Ok(ReferenceUpdateDto {
            status: "current",
            catalog_cards: store
                .load_catalog()
                .map_err(|error| error.to_string())?
                .len(),
            reference_prices: store
                .load_prices()
                .map_err(|error| error.to_string())?
                .len(),
            update_date: local_date.to_owned(),
        });
    }

    let catalog = update_catalog_from_mtgjson(&mut store, update_directory)
        .map_err(|error| format!("Échec de l’actualisation du catalogue : {error}"))?;
    let prices = update_prices_from_cardmarket(&mut store, update_directory)
        .map_err(|error| format!("Échec de l’actualisation des prix : {error}"))?;
    let candidates = store
        .load_catalog()
        .map_err(|error| error.to_string())?
        .iter()
        .map(|card| card.matching_candidate())
        .collect::<Vec<_>>();
    let collection_ids = store
        .collections()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|collection| collection.id())
        .collect::<Vec<_>>();
    for collection_id in collection_ids {
        let collection = store
            .load_collection_by_id(collection_id)
            .map_err(|error| error.to_string())?;
        let collection = rematch_collection(collection, &candidates)
            .map_err(|error| format!("Échec du réappariement : {error}"))?;
        store
            .replace_collection_for(collection_id, &collection)
            .map_err(|error| error.to_string())?;
    }
    store
        .set_last_reference_update_date(local_date)
        .map_err(|error| error.to_string())?;

    Ok(ReferenceUpdateDto {
        status: "updated",
        catalog_cards: catalog.card_count(),
        reference_prices: prices.price_count(),
        update_date: local_date.to_owned(),
    })
}

struct ReferenceUpdateGuard(Arc<AtomicBool>);

impl Drop for ReferenceUpdateGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn reference_update_due(last_successful: Option<&str>, local_date: &str, force: bool) -> bool {
    force || last_successful != Some(local_date)
}

fn valid_local_date(value: &str) -> bool {
    is_valid_iso_date(value)
}

#[tauri::command(async)]
pub fn load_card_image_bytes(
    state: State<'_, AppState>,
    scryfall_id: Option<String>,
    set_code: String,
    collector_number: String,
    size: String,
    face: String,
) -> Result<Response, String> {
    let size = match size.as_str() {
        "small" => CardImageSize::Small,
        "normal" => CardImageSize::Normal,
        _ => return Err("Taille d’image non prise en charge".to_owned()),
    };
    let face = match face.as_str() {
        "front" => CardImageFace::Front,
        "back" => CardImageFace::Back,
        _ => return Err("Face d’image non prise en charge".to_owned()),
    };
    let _cache_guard = state
        .image_cache_lock
        .lock()
        .map_err(|_| "local image cache lock is poisoned".to_owned())?;
    let bytes = load_card_image(
        &state.image_directory,
        scryfall_id.as_deref(),
        &set_code,
        &collector_number,
        size,
        face,
    )
    .map_err(|error| error.to_string())?;
    Ok(Response::new(bytes))
}

#[tauri::command(async)]
pub fn load_card_localization(
    state: State<'_, AppState>,
    set_code: String,
    collector_number: String,
) -> Result<CardLocalizationDto, String> {
    let key = format!(
        "{}\0{}",
        set_code.trim().to_ascii_lowercase(),
        collector_number.trim()
    );
    if let Some(localization) = state
        .card_localizations
        .lock()
        .map_err(|_| "card localization cache lock is poisoned".to_owned())?
        .get(&key)
        .cloned()
    {
        return Ok(localization);
    }

    let localization = fetch_card_localization(&set_code, &collector_number)
        .map(|localization| CardLocalizationDto {
            set_name: localization.set_name().to_owned(),
            french_text: localization.french_text().map(str::to_owned),
        })
        .map_err(|error| error.to_string())?;
    state
        .card_localizations
        .lock()
        .map_err(|_| "card localization cache lock is poisoned".to_owned())?
        .insert(key, localization.clone());
    Ok(localization)
}

#[tauri::command(async)]
pub fn load_image_cache_stats(state: State<'_, AppState>) -> Result<ImageCacheDto, String> {
    let _cache_guard = state
        .image_cache_lock
        .lock()
        .map_err(|_| "local image cache lock is poisoned".to_owned())?;
    card_image_cache_stats(&state.image_directory)
        .map(image_cache_dto)
        .map_err(|error| error.to_string())
}

#[tauri::command(async)]
pub fn clear_image_cache(state: State<'_, AppState>) -> Result<ImageCacheDto, String> {
    let _cache_guard = state
        .image_cache_lock
        .lock()
        .map_err(|_| "local image cache lock is poisoned".to_owned())?;
    clear_card_image_cache(&state.image_directory)
        .map(image_cache_dto)
        .map_err(|error| error.to_string())
}

fn image_cache_dto(stats: CardImageCacheStats) -> ImageCacheDto {
    ImageCacheDto {
        image_count: stats.image_count(),
        total_bytes: stats.total_bytes(),
        capacity_bytes: stats.capacity_bytes(),
    }
}

#[cfg(test)]
mod tests {
    use std::{error::Error, io::Cursor};

    use dicomtg_core::{
        CardCandidate, CollectionStore, build_collection, import_dragonshield_csv,
        import_mtgjson_cards_csv, import_mtgjson_prices_json,
    };

    use super::{
        DashboardQuery, build_dashboard, build_type_analysis, create_collection_csv,
        import_collection_csv, reference_update_due, update_collection_csv, valid_local_date,
    };

    #[test]
    fn price_007_refreshes_once_per_local_day_unless_forced() {
        assert!(reference_update_due(None, "2026-11-19", false));
        assert!(!reference_update_due(
            Some("2026-11-19"),
            "2026-11-19",
            false
        ));
        assert!(reference_update_due(Some("2026-11-19"), "2026-11-19", true));
        assert!(valid_local_date("2026-11-19"));
        assert!(!valid_local_date("19/11/2026"));
        assert!(!valid_local_date("2026-02-30"));
    }

    #[test]
    fn app_001_builds_the_dashboard_contract_from_the_local_store() -> Result<(), Box<dyn Error>> {
        let mut store = CollectionStore::in_memory()?;
        let catalog = import_mtgjson_cards_csv(Cursor::new(include_bytes!(
            "../../../../crates/dicomtg-core/tests/fixtures/mtgjson_cards.csv"
        )))?;
        let candidates = catalog
            .cards()
            .iter()
            .map(|card| card.matching_candidate())
            .collect::<Vec<_>>();
        let report = import_dragonshield_csv(Cursor::new(
            "Quantity,Card Name,Set Code,Card Number\n2,Admonition Angel,WWK,1\n",
        ))?;
        let collection = build_collection(report, &candidates)?;
        let prices = import_mtgjson_prices_json(Cursor::new(
            r#"{"data":{"admonition-uuid":{"paper":{"cardmarket":{"currency":"EUR","retail":{"normal":{"2026-11-15":1.25}}}}}}}"#,
        ))?;
        store.replace_catalog(&catalog)?;
        store.replace_collection(&collection)?;
        store.replace_prices(&prices)?;

        let dashboard = build_dashboard(&store, DashboardQuery::default())?;

        assert_eq!(dashboard.metrics.total_quantity, 2);
        assert_eq!(dashboard.metrics.distinct_names, 1);
        assert_eq!(dashboard.metrics.matched_printings, 1);
        assert_eq!(dashboard.metrics.total_value_cents, 250);
        assert_eq!(dashboard.metrics.price_coverage_basis_points, 10_000);
        assert_eq!(dashboard.metrics.catalog_cards, catalog.cards().len());
        assert_eq!(dashboard.metrics.reference_prices, prices.prices().len());
        assert_eq!(dashboard.items.len(), 1);
        assert_eq!(
            dashboard
                .most_expensive_card
                .as_ref()
                .map(|item| item.name.as_str()),
            Some("Admonition Angel")
        );
        assert_eq!(dashboard.items[0].name, "Admonition Angel");
        assert_eq!(dashboard.items[0].finish, "normal");
        assert_eq!(dashboard.items[0].price_cents, Some(125));
        assert_eq!(dashboard.items[0].status, "matched");
        assert_eq!(dashboard.items[0].color_identity, ["W"]);
        assert_eq!(dashboard.items[0].types, ["Creature"]);
        assert_eq!(dashboard.items[0].keywords, ["Flying"]);
        assert_eq!(dashboard.items[0].finishes, ["foil", "nonfoil"]);
        assert_eq!(dashboard.items[0].language.as_deref(), Some("English"));
        assert_eq!(
            dashboard.items[0].mana_cost.as_deref(),
            Some("{3}{W}{W}{W}")
        );
        assert_eq!(dashboard.items[0].power.as_deref(), Some("6"));
        assert_eq!(dashboard.items[0].toughness.as_deref(), Some("6"));
        assert_eq!(
            dashboard.items[0].type_line.as_deref(),
            Some("Creature — Angel")
        );
        assert!(dashboard.items[0].rules_text.is_some());
        assert_eq!(dashboard.distributions.colors[0].label, "W");
        assert_eq!(dashboard.distributions.colors[0].quantity, 2);
        assert_eq!(dashboard.distributions.types[0].label, "Creature");
        assert_eq!(dashboard.distributions.keywords[0].label, "Flying");
        assert_eq!(dashboard.distributions.sets[0].label, "WWK");
        assert_eq!(dashboard.distributions.rarities[0].label, "mythic");
        assert_eq!(dashboard.distributions.mana_values[0].label, "6");
        Ok(())
    }

    #[test]
    fn ui_008_featured_card_ignores_the_list_filters() -> Result<(), Box<dyn Error>> {
        let mut store = CollectionStore::in_memory()?;
        let catalog = import_mtgjson_cards_csv(Cursor::new(include_bytes!(
            "../../../../crates/dicomtg-core/tests/fixtures/mtgjson_cards.csv"
        )))?;
        let candidates = catalog
            .cards()
            .iter()
            .map(|card| card.matching_candidate())
            .collect::<Vec<_>>();
        let report = import_dragonshield_csv(Cursor::new(concat!(
            "Quantity,Card Name,Set Code,Card Number\n",
            "1,Admonition Angel,WWK,1\n",
            "10,Who What,TST,001a\n",
        )))?;
        let prices = import_mtgjson_prices_json(Cursor::new(
            r#"{"data":{"admonition-uuid":{"paper":{"cardmarket":{"currency":"EUR","retail":{"normal":{"2026-11-15":5.00}}}}},"split-uuid":{"paper":{"cardmarket":{"currency":"EUR","retail":{"normal":{"2026-11-15":1.00}}}}}}}"#,
        ))?;
        store.replace_catalog(&catalog)?;
        store.replace_collection(&build_collection(report, &candidates)?)?;
        store.replace_prices(&prices)?;

        let dashboard = build_dashboard(
            &store,
            DashboardQuery {
                search: "Who".to_owned(),
                ..DashboardQuery::default()
            },
        )?;

        assert_eq!(dashboard.items.len(), 1);
        assert_eq!(dashboard.items[0].name, "Who, What");
        assert_eq!(
            dashboard
                .most_expensive_card
                .as_ref()
                .map(|item| item.name.as_str()),
            Some("Admonition Angel")
        );
        Ok(())
    }

    #[test]
    fn ui_006_builds_live_type_analysis_from_the_local_store() -> Result<(), Box<dyn Error>> {
        let mut store = CollectionStore::in_memory()?;
        let catalog = import_mtgjson_cards_csv(Cursor::new(include_bytes!(
            "../../../../crates/dicomtg-core/tests/fixtures/mtgjson_cards.csv"
        )))?;
        let candidates = catalog
            .cards()
            .iter()
            .map(|card| card.matching_candidate())
            .collect::<Vec<_>>();
        let report = import_dragonshield_csv(Cursor::new(concat!(
            "Quantity,Card Name,Set Code,Card Number\n",
            "2,Admonition Angel,WWK,1\n",
            "3,Who What,TST,001a\n",
        )))?;
        store.replace_catalog(&catalog)?;
        store.replace_collection(&build_collection(report, &candidates)?)?;

        let analysis = build_type_analysis(&store, "Legendary")?;

        assert_eq!(analysis.selected_type, "Legendary");
        assert_eq!(analysis.metrics.total_quantity, 3);
        assert_eq!(analysis.metrics.distinct_names, 1);
        assert_eq!(analysis.metrics.total_value_cents, 0);
        assert_eq!(analysis.colors.len(), 2);
        assert_eq!(analysis.mana_values[0].label, "3");
        assert_eq!(analysis.items.len(), 1);
        assert_eq!(analysis.items[0].name, "Who, What");
        assert_eq!(
            analysis.items[0].scryfall_id.as_deref(),
            Some("11111111-1111-4111-8111-111111111111")
        );
        assert_eq!(
            build_type_analysis(&store, " ").expect_err("blank types must be rejected"),
            "card type is required"
        );
        Ok(())
    }

    #[test]
    fn col_001_imports_a_dragonshield_snapshot_and_reports_invalid_rows()
    -> Result<(), Box<dyn Error>> {
        let mut store = CollectionStore::in_memory()?;
        let catalog = import_mtgjson_cards_csv(Cursor::new(include_bytes!(
            "../../../../crates/dicomtg-core/tests/fixtures/mtgjson_cards.csv"
        )))?;
        store.replace_catalog(&catalog)?;

        let summary = import_collection_csv(
            &mut store,
            "Quantity,Card Name,Set Code,Card Number\n\
             2,Admonition Angel,WWK,1\n\
             0,Broken quantity,WWK,2\n",
        )?;

        assert_eq!(summary.valid_rows, 1);
        assert_eq!(summary.invalid_rows, 1);
        assert_eq!(summary.matched_printings, 1);
        assert_eq!(summary.unmatched_rows, 0);
        assert_eq!(summary.ambiguous_rows, 0);
        assert_eq!(summary.total_quantity, 2);
        assert_eq!(summary.issues.len(), 1);
        assert_eq!(summary.issues[0].line, 3);
        assert_eq!(store.load_collection()?.items()[0].quantity(), 2);
        Ok(())
    }

    #[test]
    fn col_009_creates_and_replaces_only_the_named_collection() -> Result<(), Box<dyn Error>> {
        let mut store = CollectionStore::in_memory()?;
        let summary = create_collection_csv(
            &mut store,
            "Collection de Léa",
            "Quantity,Card Name,Set Code,Card Number\n2,Unknown card,TST,1\n",
        )?;
        let friend_id = store.collections()?[1].id();

        assert_eq!(summary.total_quantity, 2);
        assert_eq!(store.load_collection()?.unresolved()[0].quantity(), 2);

        update_collection_csv(
            &mut store,
            friend_id,
            "Quantity,Card Name,Set Code,Card Number\n4,Replacement,TST,2\n",
        )?;

        assert_eq!(store.load_collection_by_id(1)?.unresolved().len(), 0);
        assert_eq!(
            store.load_collection_by_id(friend_id)?.unresolved()[0].name(),
            "Replacement"
        );
        assert_eq!(store.load_collection()?.unresolved()[0].quantity(), 4);
        Ok(())
    }

    #[test]
    fn acc_008_import_uses_dragonshield_language_to_select_the_printing()
    -> Result<(), Box<dyn Error>> {
        let mut store = CollectionStore::in_memory()?;
        let header = "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,side\n";
        let english =
            ",,nonfoil,,English,,2,Counterspell,15,,uncommon,STA,,,Instant,Instant,english,a\n";
        let french =
            ",,nonfoil,,French,,2,Counterspell,15,,uncommon,STA,,,Instant,Instant,french,a\n";
        let catalog = import_mtgjson_cards_csv(Cursor::new(format!("{header}{english}{french}")))?;
        store.replace_catalog(&catalog)?;

        let summary = import_collection_csv(
            &mut store,
            "Quantity,Card Name,Set Code,Card Number,Language\n1,Counterspell,STA,15,French\n",
        )?;

        assert_eq!(summary.matched_printings, 1);
        assert_eq!(summary.ambiguous_rows, 0);
        assert_eq!(store.load_collection()?.items()[0].uuid(), "french");
        Ok(())
    }

    #[test]
    fn acc_009_import_uses_dragonshield_printing_when_language_is_not_enough()
    -> Result<(), Box<dyn Error>> {
        let mut store = CollectionStore::in_memory()?;
        let header = "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,side\n";
        let normal =
            ",,nonfoil,,French,,2,Counterspell,15,,uncommon,STA,,,Instant,Instant,normal,a\n";
        let foil = ",,foil,,French,,2,Counterspell,15,,uncommon,STA,,,Instant,Instant,foil,a\n";
        let etched =
            ",,etched,,French,,2,Counterspell,15,,uncommon,STA,,,Instant,Instant,etched,a\n";
        let catalog =
            import_mtgjson_cards_csv(Cursor::new(format!("{header}{normal}{foil}{etched}")))?;
        store.replace_catalog(&catalog)?;

        let summary = import_collection_csv(
            &mut store,
            "Quantity,Card Name,Set Code,Card Number,Printing,Language\n1,Counterspell,STA,15,Halo Foil,French\n",
        )?;

        assert_eq!(summary.matched_printings, 1);
        assert_eq!(summary.ambiguous_rows, 0);
        assert_eq!(store.load_collection()?.items()[0].uuid(), "foil");

        import_collection_csv(
            &mut store,
            "Quantity,Card Name,Set Code,Card Number,Printing,Language\n1,Counterspell,STA,15,Normal,French\n",
        )?;
        assert_eq!(store.load_collection()?.items()[0].uuid(), "normal");

        import_collection_csv(
            &mut store,
            "Quantity,Card Name,Set Code,Card Number,Printing,Language\n1,Counterspell,STA,15,Etched Foil,French\n",
        )?;
        assert_eq!(store.load_collection()?.items()[0].uuid(), "etched");
        Ok(())
    }

    #[test]
    fn col_006_import_without_a_valid_row_preserves_the_previous_collection()
    -> Result<(), Box<dyn Error>> {
        let mut store = CollectionStore::in_memory()?;
        import_collection_csv(
            &mut store,
            "Quantity,Card Name,Set Code,Card Number\n1,Existing card,OLD,7\n",
        )?;
        let previous = store.load_collection()?;

        let error = import_collection_csv(
            &mut store,
            "Quantity,Card Name,Set Code,Card Number\n0,Broken quantity,NEW,8\n",
        )
        .expect_err("an import containing only invalid rows must fail");

        assert!(error.contains("aucune ligne valide"));
        assert_eq!(store.load_collection()?, previous);
        Ok(())
    }

    #[test]
    fn dq_001_dashboard_exposes_persisted_invalid_and_ambiguous_rows() -> Result<(), Box<dyn Error>>
    {
        let mut store = CollectionStore::in_memory()?;
        let report = import_dragonshield_csv(Cursor::new(
            "Quantity,Card Name,Set Code,Card Number,Language,Printing\n\
             1,Mystery Card,TST,007,French,Galaxy Foil\n\
             0,Broken quantity,TST,008,French,Normal\n",
        ))?;
        let candidates = vec![
            CardCandidate::with_language("candidate-a", "Mystery Card", "TST", "007", "French"),
            CardCandidate::with_language("candidate-b", "Mystery Card", "TST", "007", "French"),
        ];
        let collection = build_collection(report, &candidates)?;
        store.replace_collection(&collection)?;

        let dashboard = build_dashboard(&store, DashboardQuery::default())?;

        assert_eq!(dashboard.diagnostics.invalid_rows.len(), 1);
        assert_eq!(dashboard.diagnostics.invalid_rows[0].line, 3);
        assert_eq!(
            dashboard.diagnostics.invalid_rows[0].reason,
            "Quantité invalide : 0"
        );
        assert_eq!(dashboard.diagnostics.unresolved_rows.len(), 1);
        let unresolved = &dashboard.diagnostics.unresolved_rows[0];
        assert_eq!(unresolved.name, "Mystery Card");
        assert_eq!(unresolved.set_code, "TST");
        assert_eq!(unresolved.collector_number, "007");
        assert_eq!(unresolved.quantity, 1);
        assert_eq!(unresolved.language.as_deref(), Some("French"));
        assert_eq!(unresolved.printing.as_deref(), Some("Galaxy Foil"));
        assert_eq!(unresolved.status, "ambiguous");
        assert_eq!(unresolved.candidate_uuids, ["candidate-a", "candidate-b"]);
        Ok(())
    }
}
