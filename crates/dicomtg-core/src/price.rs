use std::{
    collections::{BTreeMap, HashMap, HashSet},
    error::Error,
    fmt,
    io::Read,
};

use serde::Deserialize;
use serde_json::Number;

use crate::{CardFinish, CatalogCard};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferencePrice {
    pub(crate) uuid: String,
    pub(crate) finish: CardFinish,
    pub(crate) amount_cents: u64,
    pub(crate) currency: String,
    pub(crate) as_of_date: String,
}

impl ReferencePrice {
    #[must_use]
    pub fn uuid(&self) -> &str {
        &self.uuid
    }

    #[must_use]
    pub fn finish(&self) -> CardFinish {
        self.finish
    }

    #[must_use]
    pub fn amount_cents(&self) -> u64 {
        self.amount_cents
    }

    #[must_use]
    pub fn currency(&self) -> &str {
        &self.currency
    }

    #[must_use]
    pub fn as_of_date(&self) -> &str {
        &self.as_of_date
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceImport {
    pub(crate) prices: Vec<ReferencePrice>,
    pub(crate) set_release_dates: BTreeMap<String, String>,
}

impl PriceImport {
    #[must_use]
    pub fn prices(&self) -> &[ReferencePrice] {
        &self.prices
    }
}

#[derive(Debug)]
pub enum PriceImportError {
    Json(serde_json::Error),
    EmptyUuid,
    UnsupportedCurrency { uuid: String, currency: String },
    InvalidAmount { uuid: String, value: String },
    InvalidDate { uuid: String, value: String },
    InvalidCreatedAt { value: String },
    EmptyPrices,
}

impl fmt::Display for PriceImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "invalid MTGJSON prices: {error}"),
            Self::EmptyUuid => write!(formatter, "MTGJSON price has an empty uuid"),
            Self::UnsupportedCurrency { uuid, currency } => {
                write!(
                    formatter,
                    "unsupported Cardmarket currency {currency} for {uuid}"
                )
            }
            Self::InvalidAmount { uuid, value } => {
                write!(formatter, "invalid Cardmarket amount {value} for {uuid}")
            }
            Self::InvalidDate { uuid, value } => {
                write!(formatter, "invalid Cardmarket date {value} for {uuid}")
            }
            Self::InvalidCreatedAt { value } => {
                write!(formatter, "invalid Cardmarket guide date {value}")
            }
            Self::EmptyPrices => write!(formatter, "MTGJSON contains no reference prices"),
        }
    }
}

impl Error for PriceImportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::EmptyUuid
            | Self::UnsupportedCurrency { .. }
            | Self::InvalidAmount { .. }
            | Self::InvalidDate { .. }
            | Self::InvalidCreatedAt { .. }
            | Self::EmptyPrices => None,
        }
    }
}

impl From<serde_json::Error> for PriceImportError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

pub fn import_mtgjson_prices_json(reader: impl Read) -> Result<PriceImport, PriceImportError> {
    let file: PriceFile = serde_json::from_reader(reader)?;
    let mut prices = Vec::new();

    for (uuid, formats) in file.data {
        let Some(list) = formats
            .paper
            .and_then(|paper| paper.cardmarket)
            .and_then(|list| list.retail.map(|retail| (list.currency, retail)))
        else {
            continue;
        };
        let (currency, retail) = list;
        if uuid.trim().is_empty() {
            return Err(PriceImportError::EmptyUuid);
        }
        if currency != "EUR" {
            return Err(PriceImportError::UnsupportedCurrency { uuid, currency });
        }

        for (finish, points) in [
            (CardFinish::Normal, retail.normal),
            (CardFinish::Foil, retail.foil),
            (CardFinish::Etched, retail.etched),
        ] {
            if let Some((as_of_date, amount_cents)) =
                latest_price(&uuid, points.unwrap_or_default())?
            {
                prices.push(ReferencePrice {
                    uuid: uuid.clone(),
                    finish,
                    amount_cents,
                    currency: currency.clone(),
                    as_of_date,
                });
            }
        }
    }

    if prices.is_empty() {
        Err(PriceImportError::EmptyPrices)
    } else {
        Ok(PriceImport {
            prices,
            set_release_dates: BTreeMap::new(),
        })
    }
}

pub fn import_cardmarket_price_guide(
    reader: impl Read,
    catalog: &[CatalogCard],
) -> Result<PriceImport, PriceImportError> {
    import_cardmarket_price_guide_with_ids(reader, catalog, &HashMap::new())
}

pub fn import_cardmarket_price_guide_with_products(
    reader: impl Read,
    products_reader: impl Read,
    catalog: &[CatalogCard],
) -> Result<PriceImport, PriceImportError> {
    let products: CardmarketProductFile = serde_json::from_reader(products_reader)?;
    let resolved_ids = resolve_cardmarket_product_ids(catalog, &products.products, &[]);
    import_cardmarket_price_guide_with_ids(reader, catalog, &resolved_ids)
}

pub fn import_cardmarket_price_guide_with_products_and_sets(
    reader: impl Read,
    products_reader: impl Read,
    sets_reader: impl Read,
    catalog: &[CatalogCard],
) -> Result<PriceImport, PriceImportError> {
    let products: CardmarketProductFile = serde_json::from_reader(products_reader)?;
    let sets: MtgjsonSetListFile = serde_json::from_reader(sets_reader)?;
    let resolved_ids = resolve_cardmarket_product_ids(catalog, &products.products, &sets.data);
    let mut prices = import_cardmarket_price_guide_with_ids(reader, catalog, &resolved_ids)?;
    prices.set_release_dates = sets.release_dates();
    Ok(prices)
}

fn import_cardmarket_price_guide_with_ids(
    reader: impl Read,
    catalog: &[CatalogCard],
    resolved_ids: &HashMap<String, Vec<u64>>,
) -> Result<PriceImport, PriceImportError> {
    let file: CardmarketPriceGuideFile = serde_json::from_reader(reader)?;
    let as_of_date = file
        .created_at
        .get(..10)
        .filter(|value| valid_iso_date(value))
        .ok_or_else(|| PriceImportError::InvalidCreatedAt {
            value: file.created_at.clone(),
        })?;
    let guides = file
        .price_guides
        .into_iter()
        .map(|guide| (guide.product_id, guide))
        .collect::<BTreeMap<_, _>>();
    let mut prices = Vec::new();

    for card in catalog {
        let direct_id = card.cardmarket_id().map(|product_id| [product_id]);
        let product_ids = direct_id.as_ref().map_or_else(
            || resolved_ids.get(card.uuid()).map_or(&[][..], Vec::as_slice),
            <[_; 1]>::as_slice,
        );
        if product_ids.is_empty() {
            continue;
        }
        let has_normal = has_finish(card, "nonfoil");
        let has_foil = has_finish(card, "foil");
        let has_etched = has_finish(card, "etched");
        if has_normal {
            push_product_price(
                &mut prices,
                card,
                CardFinish::Normal,
                product_ids,
                &guides,
                as_of_date,
            )?;
        }
        if has_foil || has_etched {
            push_product_price(
                &mut prices,
                card,
                CardFinish::Foil,
                product_ids,
                &guides,
                as_of_date,
            )?;
        }
        if has_etched {
            push_product_price(
                &mut prices,
                card,
                CardFinish::Etched,
                product_ids,
                &guides,
                as_of_date,
            )?;
        }
    }

    if prices.is_empty() {
        Err(PriceImportError::EmptyPrices)
    } else {
        Ok(PriceImport {
            prices,
            set_release_dates: BTreeMap::new(),
        })
    }
}

fn resolve_cardmarket_product_ids(
    catalog: &[CatalogCard],
    products: &[CardmarketProduct],
    sets: &[MtgjsonSet],
) -> HashMap<String, Vec<u64>> {
    let expansions_by_set = infer_expansions(catalog, products, sets);
    let mut products_by_expansion = HashMap::<u64, Vec<&CardmarketProduct>>::new();
    for product in products {
        products_by_expansion
            .entry(product.expansion_id)
            .or_default()
            .push(product);
    }

    let mut resolved = HashMap::new();
    for card in catalog.iter().filter(|card| card.cardmarket_id().is_none()) {
        let Some(expansions) = expansions_by_set.get(card.key().set_code()) else {
            continue;
        };
        let candidates = expansions
            .iter()
            .filter_map(|expansion| products_by_expansion.get(expansion))
            .flatten()
            .filter(|product| cardmarket_product_matches(card, &product.name))
            .map(|product| product.product_id)
            .collect::<HashSet<_>>();
        if !candidates.is_empty() {
            let mut candidates = candidates.into_iter().collect::<Vec<_>>();
            candidates.sort_unstable();
            resolved.insert(card.uuid().to_owned(), candidates);
        }
    }
    resolved
}

fn infer_expansions<'a>(
    catalog: &'a [CatalogCard],
    products: &[CardmarketProduct],
    sets: &[MtgjsonSet],
) -> HashMap<&'a str, HashSet<u64>> {
    let mut expansions_by_name = HashMap::<String, HashSet<u64>>::new();
    for product in products {
        for name in cardmarket_product_names(&product.name) {
            expansions_by_name
                .entry(name)
                .or_default()
                .insert(product.expansion_id);
        }
    }

    let mut names_by_set = HashMap::<&str, HashSet<String>>::new();
    for card in catalog {
        names_by_set
            .entry(card.key().set_code())
            .or_default()
            .insert(normalized_token_name(card.name()));
    }

    let sets_by_code = sets
        .iter()
        .flat_map(|set| {
            std::iter::once(set.code.as_str())
                .chain(set.token_set_code.as_deref())
                .map(move |code| (code, set))
        })
        .collect::<HashMap<_, _>>();
    let mut expansions_by_set = HashMap::new();
    for (set_code, names) in names_by_set {
        let mut scores = HashMap::<u64, usize>::new();
        for name in &names {
            if let Some(expansions) = expansions_by_name.get(name) {
                for expansion in expansions {
                    *scores.entry(*expansion).or_default() += 1;
                }
            }
        }
        let Some(best_score) = scores.values().copied().max().filter(|score| *score >= 2) else {
            continue;
        };
        let mut candidates = scores
            .into_iter()
            .filter_map(|(expansion, score)| (score == best_score).then_some(expansion))
            .collect::<HashSet<_>>();
        if candidates.len() > 1
            && let Some(set) = sets_by_code.get(set_code)
        {
            let preferred = [set.cardmarket_id, set.cardmarket_extras_id]
                .into_iter()
                .flatten()
                .filter(|expansion| candidates.contains(expansion))
                .collect::<HashSet<_>>();
            if !preferred.is_empty() {
                candidates = preferred;
            } else if let Some(release_day) = set.release_date.as_deref().and_then(iso_date_day) {
                candidates = closest_expansions_by_date(&candidates, &names, products, release_day);
            }
        }
        if candidates.len() == 1 {
            expansions_by_set.insert(set_code, candidates);
        }
    }
    expansions_by_set
}

fn closest_expansions_by_date(
    expansions: &HashSet<u64>,
    names: &HashSet<String>,
    products: &[CardmarketProduct],
    release_day: i64,
) -> HashSet<u64> {
    let mut dates_by_expansion = HashMap::<u64, Vec<i64>>::new();
    for product in products.iter().filter(|product| {
        expansions.contains(&product.expansion_id)
            && cardmarket_product_names(&product.name)
                .into_iter()
                .any(|name| names.contains(&name))
    }) {
        if let Some(day) = product.date_added.as_deref().and_then(timestamp_day) {
            dates_by_expansion
                .entry(product.expansion_id)
                .or_default()
                .push(day);
        }
    }
    let distances = expansions
        .iter()
        .filter_map(|expansion| {
            let dates = dates_by_expansion.get_mut(expansion)?;
            dates.sort_unstable();
            let median = dates[dates.len() / 2];
            Some((*expansion, median.abs_diff(release_day)))
        })
        .collect::<HashMap<_, _>>();
    let Some(closest) = distances.values().copied().min() else {
        return expansions.clone();
    };
    distances
        .into_iter()
        .filter_map(|(expansion, distance)| (distance == closest).then_some(expansion))
        .collect()
}

fn cardmarket_product_names(value: &str) -> HashSet<String> {
    std::iter::once(value)
        .chain(value.split(" // ").flat_map(|part| part.split(" / ")))
        .map(normalized_token_name)
        .collect()
}

fn cardmarket_product_matches(card: &CatalogCard, product_name: &str) -> bool {
    let faces = product_name
        .split(" // ")
        .flat_map(|part| part.split(" / "))
        .collect::<Vec<_>>();
    let card_faces = card.name().split(" // ").collect::<Vec<_>>();
    if card_faces.len() > 1
        && card_faces.len() == faces.len()
        && card_faces.iter().all(|card_face| {
            faces
                .iter()
                .any(|face| normalized_token_name(face) == normalized_token_name(card_face))
        })
    {
        return true;
    }
    if !is_supplementary_object(card) && card.has_back_face() != (faces.len() > 1) {
        return false;
    }
    faces
        .into_iter()
        .any(|face| cardmarket_face_matches(card, face))
}

fn cardmarket_face_matches(card: &CatalogCard, face: &str) -> bool {
    if normalized_token_name(face) != normalized_token_name(card.name()) {
        return false;
    }
    if !is_token(card) {
        return true;
    }
    let Some((power, toughness)) = card.power().zip(card.toughness()) else {
        return true;
    };
    let Some((_, descriptor)) = face.split_once(" (") else {
        return has_distinct_token_name(card);
    };
    descriptor.contains(&format!("{power}/{toughness}"))
}

fn is_token(card: &CatalogCard) -> bool {
    card.types()
        .iter()
        .any(|card_type| card_type.eq_ignore_ascii_case("token"))
}

fn is_supplementary_object(card: &CatalogCard) -> bool {
    card.types().iter().any(|card_type| {
        ["token", "emblem", "card"]
            .iter()
            .any(|expected| card_type.eq_ignore_ascii_case(expected))
    })
}

fn has_distinct_token_name(card: &CatalogCard) -> bool {
    ['—', '–', '-']
        .into_iter()
        .find_map(|separator| card.type_line().split_once(separator))
        .is_some_and(|(_, subtypes)| {
            !normalized_token_name(card.name()).eq_ignore_ascii_case(subtypes.trim())
        })
}

fn normalized_token_name(value: &str) -> String {
    let name = value
        .split_once(" (")
        .map_or(value, |(name, _)| name)
        .trim();
    name.strip_suffix(" Token")
        .unwrap_or(name)
        .trim()
        .to_lowercase()
}

fn latest_price(
    uuid: &str,
    points: BTreeMap<String, Number>,
) -> Result<Option<(String, u64)>, PriceImportError> {
    let mut latest = None;
    for (date, amount) in points {
        if !valid_iso_date(&date) {
            return Err(PriceImportError::InvalidDate {
                uuid: uuid.to_owned(),
                value: date,
            });
        }
        let amount_cents =
            amount_to_cents(&amount).ok_or_else(|| PriceImportError::InvalidAmount {
                uuid: uuid.to_owned(),
                value: amount.to_string(),
            })?;
        if latest
            .as_ref()
            .is_none_or(|(latest_date, _)| date > *latest_date)
        {
            latest = Some((date, amount_cents));
        }
    }
    Ok(latest)
}

fn has_finish(card: &CatalogCard, expected: &str) -> bool {
    card.finishes()
        .iter()
        .any(|finish| finish.eq_ignore_ascii_case(expected))
}

fn push_product_price(
    prices: &mut Vec<ReferencePrice>,
    card: &CatalogCard,
    finish: CardFinish,
    product_ids: &[u64],
    guides: &BTreeMap<u64, CardmarketPriceGuide>,
    as_of_date: &str,
) -> Result<(), PriceImportError> {
    let mut amounts = Vec::with_capacity(product_ids.len());
    for guide in product_ids.iter().filter_map(|id| guides.get(id)) {
        let candidates = match finish {
            CardFinish::Normal => [guide.trend.as_ref(), None],
            CardFinish::Foil | CardFinish::Etched => {
                [guide.trend_foil.as_ref(), guide.trend.as_ref()]
            }
        };
        for amount in candidates.into_iter().flatten() {
            let amount_cents =
                amount_to_cents(amount).ok_or_else(|| PriceImportError::InvalidAmount {
                    uuid: card.uuid().to_owned(),
                    value: amount.to_string(),
                })?;
            if amount_cents > 0 {
                amounts.push(amount_cents);
                break;
            }
        }
    }
    amounts.sort_unstable();
    let amount_cents = match amounts.as_slice() {
        [] => return Ok(()),
        amounts if amounts.len() % 2 == 1 => amounts[amounts.len() / 2],
        amounts => {
            let upper = amounts[amounts.len() / 2];
            let lower = amounts[amounts.len() / 2 - 1];
            lower + (upper - lower).div_ceil(2)
        }
    };
    prices.push(ReferencePrice {
        uuid: card.uuid().to_owned(),
        finish,
        amount_cents,
        currency: "EUR".to_owned(),
        as_of_date: as_of_date.to_owned(),
    });
    Ok(())
}

#[derive(Deserialize)]
struct PriceFile {
    data: BTreeMap<String, PriceFormats>,
}

#[derive(Deserialize)]
struct PriceFormats {
    paper: Option<PaperPrices>,
}

#[derive(Deserialize)]
struct PaperPrices {
    cardmarket: Option<PriceList>,
}

#[derive(Deserialize)]
struct PriceList {
    currency: String,
    retail: Option<PricePoints>,
}

#[derive(Deserialize)]
struct PricePoints {
    normal: Option<BTreeMap<String, Number>>,
    foil: Option<BTreeMap<String, Number>>,
    etched: Option<BTreeMap<String, Number>>,
}

#[derive(Deserialize)]
struct CardmarketPriceGuideFile {
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "priceGuides")]
    price_guides: Vec<CardmarketPriceGuide>,
}

#[derive(Deserialize)]
struct CardmarketPriceGuide {
    #[serde(rename = "idProduct")]
    product_id: u64,
    trend: Option<Number>,
    #[serde(rename = "trend-foil")]
    trend_foil: Option<Number>,
}

#[derive(Deserialize)]
struct CardmarketProductFile {
    products: Vec<CardmarketProduct>,
}

#[derive(Deserialize)]
struct CardmarketProduct {
    #[serde(rename = "idProduct")]
    product_id: u64,
    name: String,
    #[serde(rename = "idExpansion")]
    expansion_id: u64,
    #[serde(rename = "dateAdded")]
    date_added: Option<String>,
}

#[derive(Deserialize)]
struct MtgjsonSetListFile {
    data: Vec<MtgjsonSet>,
}

pub(crate) fn import_set_release_dates(
    reader: impl Read,
) -> Result<BTreeMap<String, String>, PriceImportError> {
    let sets: MtgjsonSetListFile = serde_json::from_reader(reader)?;
    Ok(sets.release_dates())
}

impl MtgjsonSetListFile {
    fn release_dates(&self) -> BTreeMap<String, String> {
        let dated = self
            .data
            .iter()
            .filter_map(|set| {
                set.release_date
                    .as_deref()
                    .filter(|date| valid_iso_date(date))
                    .map(|date| (set, date))
            });
        let mut dates = dated
            .clone()
            .map(|(set, date)| (set.code.trim().to_ascii_uppercase(), date.to_owned()))
            .collect::<BTreeMap<_, _>>();
        for (set, date) in dated {
            if let Some(token_code) = set.token_set_code.as_deref() {
                dates
                    .entry(token_code.trim().to_ascii_uppercase())
                    .or_insert_with(|| date.to_owned());
            }
        }
        dates.remove("");
        dates
    }
}

#[derive(Deserialize)]
struct MtgjsonSet {
    code: String,
    #[serde(rename = "tokenSetCode")]
    token_set_code: Option<String>,
    #[serde(rename = "mcmId")]
    cardmarket_id: Option<u64>,
    #[serde(rename = "mcmIdExtras")]
    cardmarket_extras_id: Option<u64>,
    #[serde(rename = "releaseDate")]
    release_date: Option<String>,
}

fn amount_to_cents(amount: &Number) -> Option<u64> {
    let value = amount.to_string();
    let (units, fraction) = value.split_once('.').unwrap_or((value.as_str(), ""));
    if units.is_empty()
        || !units.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() > 2
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let units = units.parse::<u64>().ok()?;
    let fraction = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<u64>().ok()?.checked_mul(10)?,
        2 => fraction.parse::<u64>().ok()?,
        _ => return None,
    };
    units
        .checked_mul(100)?
        .checked_add(fraction)
        .filter(|cents| i64::try_from(*cents).is_ok())
}

pub(crate) fn valid_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes[..4].iter().all(u8::is_ascii_digit)
        || !bytes[5..7].iter().all(u8::is_ascii_digit)
        || !bytes[8..].iter().all(u8::is_ascii_digit)
    {
        return false;
    }

    let year = value[..4].parse::<u16>().ok().unwrap_or_default();
    let month = value[5..7].parse::<u8>().ok().unwrap_or_default();
    let day = value[8..].parse::<u8>().ok().unwrap_or_default();
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => return false,
    };
    year > 0 && day > 0 && day <= max_day
}

fn timestamp_day(value: &str) -> Option<i64> {
    value.get(..10).and_then(iso_date_day)
}

fn iso_date_day(value: &str) -> Option<i64> {
    if !valid_iso_date(value) {
        return None;
    }
    let year = value[..4].parse::<i64>().ok()?;
    let month = value[5..7].parse::<usize>().ok()?;
    let day = value[8..].parse::<i64>().ok()?;
    let previous_year = year - 1;
    let leap_days = previous_year / 4 - previous_year / 100 + previous_year / 400;
    let mut days_before_month =
        [0_i64, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334][month - 1];
    if month > 2 && is_leap_year(u16::try_from(year).ok()?) {
        days_before_month += 1;
    }
    Some(previous_year * 365 + leap_days + days_before_month + day)
}

#[must_use]
pub fn is_valid_iso_date(value: &str) -> bool {
    valid_iso_date(value)
}

fn is_leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}
