use std::{
    collections::HashMap,
    error::Error,
    fmt,
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};

use crate::{
    CardFinish, CatalogCard, CatalogImport, CollectionCard, CollectionSnapshot, ImportIssue,
    ImportedCard, MatchResult, OwnedPrinting, PriceImport, PrintingKey, ReferencePrice,
    RowIssueReason, UnresolvedCard, collection::merge_collections, price::valid_iso_date,
};

const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("../migrations/0001_collection.up.sql")),
    (2, include_str!("../migrations/0002_catalog.up.sql")),
    (3, include_str!("../migrations/0003_prices.up.sql")),
    (
        4,
        include_str!("../migrations/0004_collection_language.up.sql"),
    ),
    (
        5,
        include_str!("../migrations/0005_collection_printing.up.sql"),
    ),
    (
        6,
        include_str!("../migrations/0006_catalog_scryfall_id.up.sql"),
    ),
    (
        7,
        include_str!("../migrations/0007_catalog_has_back_face.up.sql"),
    ),
    (
        8,
        include_str!("../migrations/0008_cardmarket_prices.up.sql"),
    ),
    (
        9,
        include_str!("../migrations/0009_named_collections.up.sql"),
    ),
];

const SELECTED_PRINTING_UUIDS: &str = "SELECT items.uuid FROM collections
     JOIN collection_items AS items ON items.collection_id = collections.id
     WHERE collections.selected = 1";

fn reference_scope(uuid_column: &str, selected_only: bool) -> String {
    if selected_only {
        format!("WHERE {uuid_column} IN ({SELECTED_PRINTING_UUIDS})")
    } else {
        String::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCollection {
    id: i64,
    name: String,
    selected: bool,
    total_quantity: u64,
}

impl StoredCollection {
    #[must_use]
    pub fn id(&self) -> i64 {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn selected(&self) -> bool {
        self.selected
    }

    #[must_use]
    pub fn total_quantity(&self) -> u64 {
        self.total_quantity
    }
}

pub struct CollectionStore {
    connection: Connection,
    writer: Arc<Mutex<()>>,
}

impl CollectionStore {
    pub fn in_memory() -> Result<Self, StorageError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let store = Self::from_connection(Connection::open(path)?)?;
        // Analysis reads must not block commits from the reference updater.
        store
            .connection
            .pragma_update(None, "journal_mode", "WAL")?;
        Ok(store)
    }

    pub fn open_peer(&self) -> Result<Self, StorageError> {
        let path = self
            .connection
            .path()
            .filter(|path| !path.is_empty())
            .ok_or_else(|| StorageError::InvalidData("database has no file path".into()))?;
        let mut peer = Self::open(path)?;
        peer.writer = Arc::clone(&self.writer);
        Ok(peer)
    }

    fn from_connection(mut connection: Connection) -> Result<Self, StorageError> {
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.execute_batch("PRAGMA foreign_keys = ON;")?;
        apply_migrations(&mut connection)?;
        Ok(Self {
            connection,
            writer: Arc::new(Mutex::new(())),
        })
    }

    fn lock_writer(&self) -> Result<MutexGuard<'_, ()>, StorageError> {
        self.writer
            .lock()
            .map_err(|_| StorageError::InvalidData("database writer mutex is poisoned".into()))
    }

    fn begin_write(&self) -> Result<(MutexGuard<'_, ()>, Transaction<'_>), StorageError> {
        let guard = self.lock_writer()?;
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)?;
        Ok((guard, transaction))
    }

    pub fn replace_collection(
        &mut self,
        collection: &CollectionSnapshot,
    ) -> Result<(), StorageError> {
        let (_writer, transaction) = self.begin_write()?;
        transaction.execute(
            "INSERT OR IGNORE INTO collections (id, name, selected) VALUES (1, 'Ma collection', 1)",
            [],
        )?;
        ensure_collection_exists(&transaction, 1)?;
        replace_collection_in_transaction(&transaction, 1, collection)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn create_collection(
        &mut self,
        name: &str,
        collection: &CollectionSnapshot,
    ) -> Result<i64, StorageError> {
        let name = valid_collection_name(name)?;
        let (_writer, transaction) = self.begin_write()?;
        ensure_unique_collection_name(&transaction, name, None)?;
        transaction.execute(
            "INSERT INTO collections (name, selected) VALUES (?1, 1)",
            [name],
        )?;
        let id = transaction.last_insert_rowid();
        replace_collection_in_transaction(&transaction, id, collection)?;
        transaction.commit()?;
        Ok(id)
    }

    pub fn replace_collection_for(
        &mut self,
        collection_id: i64,
        collection: &CollectionSnapshot,
    ) -> Result<(), StorageError> {
        let (_writer, transaction) = self.begin_write()?;
        ensure_collection_exists(&transaction, collection_id)?;
        replace_collection_in_transaction(&transaction, collection_id, collection)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn replace_collection_if_unchanged(
        &mut self,
        collection_id: i64,
        expected: &CollectionSnapshot,
        replacement: &CollectionSnapshot,
    ) -> Result<bool, StorageError> {
        let (_writer, transaction) = self.begin_write()?;
        ensure_collection_exists(&transaction, collection_id)?;
        if self.load_collection_rows(collection_id)? != *expected {
            return Ok(false);
        }
        replace_collection_in_transaction(&transaction, collection_id, replacement)?;
        transaction.commit()?;
        Ok(true)
    }

    pub fn rename_collection(
        &mut self,
        collection_id: i64,
        name: &str,
    ) -> Result<(), StorageError> {
        let name = valid_collection_name(name)?;
        let (_writer, transaction) = self.begin_write()?;
        ensure_collection_exists(&transaction, collection_id)?;
        ensure_unique_collection_name(&transaction, name, Some(collection_id))?;
        transaction.execute(
            "UPDATE collections SET name = ?1 WHERE id = ?2",
            params![name, collection_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn set_collection_selected(
        &mut self,
        collection_id: i64,
        selected: bool,
    ) -> Result<(), StorageError> {
        let _writer = self.lock_writer()?;
        let changed = self.connection.execute(
            "UPDATE collections SET selected = ?1 WHERE id = ?2",
            params![selected, collection_id],
        )?;
        if changed == 0 {
            return Err(StorageError::InvalidData(format!(
                "unknown collection: {collection_id}"
            )));
        }
        Ok(())
    }

    pub fn collections(&self) -> Result<Vec<StoredCollection>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT collections.id, collections.name, collections.selected,
                    COALESCE(items.quantity, 0) + COALESCE(unresolved.quantity, 0)
             FROM collections
             LEFT JOIN (
                 SELECT collection_id, SUM(quantity) AS quantity
                 FROM collection_items GROUP BY collection_id
             ) items ON items.collection_id = collections.id
             LEFT JOIN (
                 SELECT collection_id, SUM(quantity) AS quantity
                 FROM unresolved_cards GROUP BY collection_id
             ) unresolved ON unresolved.collection_id = collections.id
             ORDER BY collections.id",
        )?;
        let mut rows = statement.query([])?;
        let mut collections = Vec::new();
        while let Some(row) = rows.next()? {
            let stored_total = row.get::<_, i64>(3)?;
            let total_quantity = u64::try_from(stored_total).map_err(|_| {
                StorageError::InvalidData(format!("invalid collection quantity: {stored_total}"))
            })?;
            collections.push(StoredCollection {
                id: row.get(0)?,
                name: row.get(1)?,
                selected: row.get::<_, i64>(2)? != 0,
                total_quantity,
            });
        }
        Ok(collections)
    }

    pub fn load_collection_by_id(
        &self,
        collection_id: i64,
    ) -> Result<CollectionSnapshot, StorageError> {
        let transaction = self.connection.unchecked_transaction()?;
        ensure_collection_exists(&transaction, collection_id)?;
        let collection = self.load_collection_rows(collection_id)?;
        transaction.commit()?;
        Ok(collection)
    }

    fn load_collection_rows(&self, collection_id: i64) -> Result<CollectionSnapshot, StorageError> {
        let items = self.load_items(collection_id)?;
        let unresolved = self.load_unresolved(collection_id)?;
        let import_issues = self.load_import_issues(collection_id)?;
        Ok(CollectionSnapshot::new(items, unresolved, import_issues))
    }

    pub fn load_collection(&self) -> Result<CollectionSnapshot, StorageError> {
        let selected_ids = self
            .connection
            .prepare_cached("SELECT id FROM collections WHERE selected = 1 ORDER BY id")?
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<i64>, _>>()?;
        let collections = selected_ids
            .into_iter()
            .map(|id| self.load_collection_rows(id))
            .collect::<Result<Vec<_>, _>>()?;
        merge_collections(collections).map_err(|error| StorageError::InvalidData(error.to_string()))
    }

    pub fn load_collection_with_references(
        &self,
    ) -> Result<(CollectionSnapshot, Vec<CatalogCard>, Vec<ReferencePrice>), StorageError> {
        // The reference updater writes through a separate connection.
        let transaction = self.connection.unchecked_transaction()?;
        let collection = self.load_collection()?;
        let catalog = self.load_catalog_scoped(true)?;
        let prices = self.load_prices_scoped(true)?;
        transaction.commit()?;
        Ok((collection, catalog, prices))
    }

    pub fn reference_counts(&self) -> Result<(usize, usize), StorageError> {
        Ok(self.connection.query_row(
            "SELECT (SELECT COUNT(*) FROM catalog_cards),
                    (SELECT COUNT(*) FROM reference_prices)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?)
    }

    fn load_items(&self, collection_id: i64) -> Result<Vec<OwnedPrinting>, StorageError> {
        let mut statement = self.connection.prepare_cached(
            "SELECT uuid, finish, quantity FROM collection_items
             WHERE collection_id = ?1 ORDER BY position",
        )?;
        let mut rows = statement.query([collection_id])?;
        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            let uuid = row.get::<_, String>(0)?;
            let stored_finish = row.get::<_, String>(1)?;
            let finish = CardFinish::from_storage(&stored_finish).ok_or_else(|| {
                StorageError::InvalidData(format!("invalid card finish: {stored_finish}"))
            })?;
            let quantity = stored_quantity(row.get(2)?)?;
            items.push(OwnedPrinting::new(uuid, finish, quantity));
        }
        Ok(items)
    }

    fn load_unresolved(&self, collection_id: i64) -> Result<Vec<UnresolvedCard>, StorageError> {
        let mut card_statement = self.connection.prepare_cached(
            "SELECT id, name, set_code, collector_number, quantity, match_status, language, printing
             FROM unresolved_cards WHERE collection_id = ?1 ORDER BY position",
        )?;
        let mut candidate_statement = self.connection.prepare_cached(
            "SELECT uuid FROM unresolved_candidates
             WHERE unresolved_id = ?1 ORDER BY position",
        )?;
        let mut rows = card_statement.query([collection_id])?;
        let mut cards = Vec::new();

        while let Some(row) = rows.next()? {
            let id = row.get::<_, i64>(0)?;
            let name = row.get::<_, String>(1)?;
            let set_code = row.get::<_, String>(2)?;
            let collector_number = row.get::<_, String>(3)?;
            let quantity = stored_quantity(row.get(4)?)?;
            let status = row.get::<_, String>(5)?;
            let language = row.get::<_, Option<String>>(6)?;
            let printing = row.get::<_, Option<String>>(7)?;
            let match_result = match status.as_str() {
                "unmatched" => MatchResult::Unmatched,
                "ambiguous" => {
                    let candidates = candidate_statement
                        .query_map([id], |candidate| candidate.get::<_, String>(0))?
                        .collect::<Result<Vec<_>, _>>()?;
                    MatchResult::Ambiguous {
                        candidate_uuids: candidates,
                    }
                }
                value => {
                    return Err(StorageError::InvalidData(format!(
                        "unknown match status: {value}"
                    )));
                }
            };
            cards.push(UnresolvedCard::new(
                ImportedCard::new(
                    quantity,
                    CollectionCard::new_with_details(
                        &name,
                        &set_code,
                        &collector_number,
                        language.as_deref(),
                        printing.as_deref(),
                    ),
                ),
                match_result,
            ));
        }
        Ok(cards)
    }

    fn load_import_issues(&self, collection_id: i64) -> Result<Vec<ImportIssue>, StorageError> {
        let mut statement = self.connection.prepare_cached(
            "SELECT source_line, reason_code, detail FROM import_issues
             WHERE collection_id = ?1 ORDER BY position",
        )?;
        let mut rows = statement.query([collection_id])?;
        let mut issues = Vec::new();
        while let Some(row) = rows.next()? {
            let line = stored_line(row.get(0)?)?;
            let reason_code = row.get::<_, String>(1)?;
            let detail = row.get::<_, String>(2)?;
            let reason = match reason_code.as_str() {
                "missing_field" => RowIssueReason::MissingField {
                    field: required_field(&detail)?,
                },
                "invalid_quantity" => RowIssueReason::InvalidQuantity { value: detail },
                value => {
                    return Err(StorageError::InvalidData(format!(
                        "unknown import issue: {value}"
                    )));
                }
            };
            issues.push(ImportIssue::new(line, reason));
        }
        Ok(issues)
    }

    pub fn replace_catalog(&mut self, catalog: &CatalogImport) -> Result<(), StorageError> {
        let (_writer, transaction) = self.begin_write()?;
        transaction.execute("DELETE FROM catalog_cards", [])?;

        {
            let mut card_statement = transaction.prepare(
                "INSERT INTO catalog_cards
                 (position, uuid, name, set_code, collector_number, rarity, mana_value,
                  language, mana_cost, power, toughness, type_line, rules_text, scryfall_id,
                  has_back_face, cardmarket_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                         ?15, ?16)",
            )?;
            let mut facet_statement = transaction.prepare(
                "INSERT INTO catalog_facets (card_uuid, kind, position, value)
                 VALUES (?1, ?2, ?3, ?4)",
            )?;

            for (position, card) in catalog.cards().iter().enumerate() {
                card_statement.execute(params![
                    position as i64,
                    card.uuid,
                    card.name,
                    card.key.set_code(),
                    card.key.collector_number(),
                    card.rarity,
                    card.mana_value,
                    card.language,
                    card.mana_cost,
                    card.power,
                    card.toughness,
                    card.type_line,
                    card.rules_text,
                    card.scryfall_id,
                    card.has_back_face,
                    card.cardmarket_id,
                ])?;
                insert_facets(&mut facet_statement, &card.uuid, "color", &card.colors)?;
                insert_facets(
                    &mut facet_statement,
                    &card.uuid,
                    "color_identity",
                    &card.color_identity,
                )?;
                insert_facets(&mut facet_statement, &card.uuid, "type", &card.types)?;
                insert_facets(&mut facet_statement, &card.uuid, "keyword", &card.keywords)?;
                insert_facets(&mut facet_statement, &card.uuid, "finish", &card.finishes)?;
            }
        }

        transaction.commit()?;
        Ok(())
    }

    pub fn load_catalog(&self) -> Result<Vec<CatalogCard>, StorageError> {
        let transaction = self.connection.unchecked_transaction()?;
        let catalog = self.load_catalog_scoped(false)?;
        transaction.commit()?;
        Ok(catalog)
    }

    fn load_catalog_scoped(&self, selected_only: bool) -> Result<Vec<CatalogCard>, StorageError> {
        let mut facets = self.load_catalog_facets(selected_only)?;
        let scope = reference_scope("uuid", selected_only);
        let mut statement = self.connection.prepare(&format!(
            "SELECT uuid, name, set_code, collector_number, rarity, mana_value, language,
                    mana_cost, power, toughness, type_line, rules_text, scryfall_id,
                    has_back_face, cardmarket_id
             FROM catalog_cards {scope} ORDER BY position",
        ))?;
        let mut rows = statement.query([])?;
        let mut cards = Vec::new();

        while let Some(row) = rows.next()? {
            let uuid = row.get::<_, String>(0)?;
            let card_facets = facets.remove(&uuid).unwrap_or_default();
            cards.push(CatalogCard {
                uuid,
                scryfall_id: row.get(12)?,
                has_back_face: row.get(13)?,
                cardmarket_id: row.get(14)?,
                name: row.get(1)?,
                key: PrintingKey::new(&row.get::<_, String>(2)?, &row.get::<_, String>(3)?),
                rarity: row.get(4)?,
                mana_value: stored_mana_value(row.get(5)?)?,
                colors: card_facets.colors,
                color_identity: card_facets.color_identity,
                types: card_facets.types,
                keywords: card_facets.keywords,
                finishes: card_facets.finishes,
                language: row.get(6)?,
                mana_cost: row.get(7)?,
                power: row.get(8)?,
                toughness: row.get(9)?,
                type_line: row.get(10)?,
                rules_text: row.get(11)?,
            });
        }
        if !facets.is_empty() {
            return Err(StorageError::InvalidData(
                "catalog facets reference missing cards".into(),
            ));
        }
        Ok(cards)
    }

    pub fn replace_prices(&mut self, prices: &PriceImport) -> Result<(), StorageError> {
        let (_writer, transaction) = self.begin_write()?;
        transaction.execute("DELETE FROM reference_prices", [])?;

        {
            let mut statement = transaction.prepare(
                "INSERT INTO reference_prices (uuid, finish, amount_cents, currency, as_of_date)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for price in prices.prices() {
                let amount_cents = i64::try_from(price.amount_cents()).map_err(|_| {
                    StorageError::InvalidData(format!(
                        "price is too large for {}: {}",
                        price.uuid(),
                        price.amount_cents()
                    ))
                })?;
                statement.execute(params![
                    price.uuid(),
                    price.finish().as_str(),
                    amount_cents,
                    price.currency(),
                    price.as_of_date(),
                ])?;
            }
        }

        transaction.commit()?;
        Ok(())
    }

    pub fn load_prices(&self) -> Result<Vec<ReferencePrice>, StorageError> {
        self.load_prices_scoped(false)
    }

    fn load_prices_scoped(&self, selected_only: bool) -> Result<Vec<ReferencePrice>, StorageError> {
        let scope = reference_scope("uuid", selected_only);
        let mut statement = self.connection.prepare(&format!(
            "SELECT uuid, finish, amount_cents, currency, as_of_date
             FROM reference_prices {scope}
             ORDER BY uuid,
                      CASE finish WHEN 'normal' THEN 0 WHEN 'foil' THEN 1 ELSE 2 END",
        ))?;
        let mut rows = statement.query([])?;
        let mut prices = Vec::new();

        while let Some(row) = rows.next()? {
            let uuid = row.get::<_, String>(0)?;
            let stored_finish = row.get::<_, String>(1)?;
            let stored_amount = row.get::<_, i64>(2)?;
            let currency = row.get::<_, String>(3)?;
            let as_of_date = row.get::<_, String>(4)?;
            let amount_cents = u64::try_from(stored_amount).map_err(|_| {
                StorageError::InvalidData(format!("invalid price for {uuid}: {stored_amount}"))
            })?;
            let finish = CardFinish::from_storage(&stored_finish).ok_or_else(|| {
                StorageError::InvalidData(format!("invalid card finish: {stored_finish}"))
            })?;
            if uuid.is_empty() || currency != "EUR" || !valid_iso_date(&as_of_date) {
                return Err(StorageError::InvalidData(format!(
                    "invalid reference price for {uuid}"
                )));
            }
            prices.push(ReferencePrice {
                uuid,
                finish,
                amount_cents,
                currency,
                as_of_date,
            });
        }
        Ok(prices)
    }

    pub fn last_reference_update_date(&self) -> Result<Option<String>, StorageError> {
        self.connection
            .query_row(
                "SELECT last_successful_date FROM reference_update_state WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(StorageError::from)
    }

    pub fn set_last_reference_update_date(&mut self, date: &str) -> Result<(), StorageError> {
        if !valid_iso_date(date) {
            return Err(StorageError::InvalidData(format!(
                "invalid reference update date: {date}"
            )));
        }
        let _writer = self.lock_writer()?;
        self.connection.execute(
            "INSERT INTO reference_update_state (singleton, last_successful_date)
             VALUES (1, ?1)
             ON CONFLICT(singleton) DO UPDATE SET last_successful_date = excluded.last_successful_date",
            [date],
        )?;
        Ok(())
    }

    fn load_catalog_facets(
        &self,
        selected_only: bool,
    ) -> Result<HashMap<String, CatalogFacets>, StorageError> {
        let scope = reference_scope("card_uuid", selected_only);
        let mut statement = self.connection.prepare(&format!(
            "SELECT card_uuid, kind, value
             FROM catalog_facets {scope} ORDER BY card_uuid, kind, position",
        ))?;
        let mut rows = statement.query([])?;
        let mut facets = HashMap::<String, CatalogFacets>::new();
        while let Some(row) = rows.next()? {
            let uuid = row.get::<_, String>(0)?;
            let kind = row.get::<_, String>(1)?;
            let value = row.get::<_, String>(2)?;
            let card = facets.entry(uuid).or_default();
            match kind.as_str() {
                "color" => card.colors.push(value),
                "color_identity" => card.color_identity.push(value),
                "type" => card.types.push(value),
                "keyword" => card.keywords.push(value),
                "finish" => card.finishes.push(value),
                value => {
                    return Err(StorageError::InvalidData(format!(
                        "unknown catalog facet: {value}"
                    )));
                }
            }
        }
        Ok(facets)
    }
}

fn replace_collection_in_transaction(
    transaction: &Transaction<'_>,
    collection_id: i64,
    collection: &CollectionSnapshot,
) -> Result<(), StorageError> {
    transaction.execute(
        "DELETE FROM collection_items WHERE collection_id = ?1",
        [collection_id],
    )?;
    transaction.execute(
        "DELETE FROM unresolved_cards WHERE collection_id = ?1",
        [collection_id],
    )?;
    transaction.execute(
        "DELETE FROM import_issues WHERE collection_id = ?1",
        [collection_id],
    )?;

    {
        let mut statement = transaction.prepare(
            "INSERT INTO collection_items (collection_id, position, uuid, finish, quantity)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        for (position, item) in collection.items().iter().enumerate() {
            statement.execute(params![
                collection_id,
                position as i64,
                item.uuid(),
                item.finish().as_str(),
                item.quantity()
            ])?;
        }
    }

    {
        let mut unresolved_statement = transaction.prepare(
            "INSERT INTO unresolved_cards
             (collection_id, position, name, set_code, collector_number, quantity, match_status,
              language, printing)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?;
        let mut candidate_statement = transaction.prepare(
            "INSERT INTO unresolved_candidates (unresolved_id, position, uuid)
             VALUES (?1, ?2, ?3)",
        )?;

        for (position, card) in collection.unresolved().iter().enumerate() {
            let (status, candidates) = match card.match_result() {
                MatchResult::Unmatched => ("unmatched", &[][..]),
                MatchResult::Ambiguous { candidate_uuids } => {
                    ("ambiguous", candidate_uuids.as_slice())
                }
                MatchResult::Matched { .. } => {
                    return Err(StorageError::InvalidData(
                        "resolved card found in unresolved collection".into(),
                    ));
                }
            };
            unresolved_statement.execute(params![
                collection_id,
                position as i64,
                card.name(),
                card.key().set_code(),
                card.key().collector_number(),
                card.quantity(),
                status,
                card.language(),
                card.printing(),
            ])?;
            let unresolved_id = transaction.last_insert_rowid();
            for (candidate_position, uuid) in candidates.iter().enumerate() {
                candidate_statement.execute(params![
                    unresolved_id,
                    candidate_position as i64,
                    uuid,
                ])?;
            }
        }
    }

    {
        let mut statement = transaction.prepare(
            "INSERT INTO import_issues
             (collection_id, position, source_line, reason_code, detail)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        for (position, issue) in collection.import_issues().iter().enumerate() {
            let (reason_code, detail) = issue_parts(issue.reason());
            statement.execute(params![
                collection_id,
                position as i64,
                issue.line() as i64,
                reason_code,
                detail,
            ])?;
        }
    }
    Ok(())
}

fn valid_collection_name(name: &str) -> Result<&str, StorageError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(StorageError::InvalidData(
            "collection name must contain between 1 and 80 characters".into(),
        ));
    }
    Ok(name)
}

fn ensure_collection_exists(
    connection: &Connection,
    collection_id: i64,
) -> Result<(), StorageError> {
    let exists = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM collections WHERE id = ?1)",
        [collection_id],
        |row| row.get::<_, bool>(0),
    )?;
    if !exists {
        return Err(StorageError::InvalidData(format!(
            "unknown collection: {collection_id}"
        )));
    }
    Ok(())
}

fn ensure_unique_collection_name(
    connection: &Connection,
    name: &str,
    ignored_id: Option<i64>,
) -> Result<(), StorageError> {
    let exists = connection.query_row(
        "SELECT EXISTS(
             SELECT 1 FROM collections
             WHERE name = ?1 COLLATE NOCASE AND (?2 IS NULL OR id != ?2)
         )",
        params![name, ignored_id],
        |row| row.get::<_, bool>(0),
    )?;
    if exists {
        return Err(StorageError::InvalidData(format!(
            "collection name already exists: {name}"
        )));
    }
    Ok(())
}

#[derive(Default)]
struct CatalogFacets {
    colors: Vec<String>,
    color_identity: Vec<String>,
    types: Vec<String>,
    keywords: Vec<String>,
    finishes: Vec<String>,
}

#[derive(Debug)]
pub enum StorageError {
    Sqlite(rusqlite::Error),
    InvalidData(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(error) => write!(formatter, "SQLite error: {error}"),
            Self::InvalidData(message) => write!(formatter, "invalid stored data: {message}"),
        }
    }
}

impl Error for StorageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Sqlite(error) => Some(error),
            Self::InvalidData(_) => None,
        }
    }
}

impl From<rusqlite::Error> for StorageError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        sync::{TryLockError, mpsc},
        thread,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn concurrent_writers_wait_before_attempting_sqlite() -> Result<(), Box<dyn Error>> {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!(
            "dicomtg-writer-{}-{unique}.sqlite",
            std::process::id()
        ));
        let mut primary = CollectionStore::open(&path)?;
        let id =
            primary.create_collection("Avant", &CollectionSnapshot::new(vec![], vec![], vec![]))?;
        let mut peer = primary.open_peer()?;
        assert_eq!(
            peer.connection
                .query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))?,
            "wal"
        );
        assert_eq!(
            peer.connection
                .query_row("PRAGMA busy_timeout", [], |row| row.get::<_, u32>(0))?,
            5_000
        );
        peer.connection.busy_timeout(Duration::ZERO)?;

        let (slot, transaction) = primary.begin_write()?;
        let (ready, contended) = mpsc::sync_channel(0);
        thread::scope(|scope| -> Result<(), Box<dyn Error>> {
            let waiting = scope.spawn(move || {
                let error = peer
                    .connection
                    .execute("UPDATE collections SET selected = 0", [])
                    .expect_err("an uncoordinated write must hit the active writer");
                assert_eq!(
                    error.sqlite_error_code(),
                    Some(rusqlite::ErrorCode::DatabaseBusy)
                );
                assert!(matches!(
                    peer.writer.try_lock(),
                    Err(TryLockError::WouldBlock)
                ));
                ready
                    .send(())
                    .expect("the first writer must still be running");
                peer.rename_collection(id, "Après")
            });
            contended.recv()?;
            transaction.commit()?;
            drop(slot);
            waiting.join().expect("writer thread must finish")?;
            Ok(())
        })?;
        assert_eq!(primary.collections()?[0].name(), "Après");
        drop(primary);
        fs::remove_file(path)?;
        Ok(())
    }
}

fn apply_migrations(connection: &mut Connection) -> Result<(), StorageError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
             version INTEGER PRIMARY KEY,
             applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
         );",
    )?;
    let current = connection.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get::<_, i64>(0),
    )?;
    let latest = MIGRATIONS.last().map_or(0, |migration| migration.0);
    if current > latest {
        return Err(StorageError::InvalidData(format!(
            "database schema {current} is newer than supported schema {latest}"
        )));
    }

    for (version, sql) in MIGRATIONS
        .iter()
        .copied()
        .filter(|(version, _)| *version > current)
    {
        let transaction = connection.transaction()?;
        transaction.execute_batch(sql)?;
        transaction.execute(
            "INSERT INTO schema_migrations (version) VALUES (?1)",
            [version],
        )?;
        transaction.commit()?;
    }
    Ok(())
}

fn issue_parts(reason: &RowIssueReason) -> (&'static str, &str) {
    match reason {
        RowIssueReason::MissingField { field } => ("missing_field", field),
        RowIssueReason::InvalidQuantity { value } => ("invalid_quantity", value),
    }
}

fn required_field(value: &str) -> Result<&'static str, StorageError> {
    match value {
        "Quantity" => Ok("Quantity"),
        "Card Name" => Ok("Card Name"),
        "Set Code" => Ok("Set Code"),
        "Card Number" => Ok("Card Number"),
        value => Err(StorageError::InvalidData(format!(
            "unknown required field: {value}"
        ))),
    }
}

fn stored_quantity(value: i64) -> Result<u32, StorageError> {
    u32::try_from(value)
        .ok()
        .filter(|quantity| *quantity > 0)
        .ok_or_else(|| StorageError::InvalidData(format!("invalid quantity: {value}")))
}

fn stored_line(value: i64) -> Result<usize, StorageError> {
    usize::try_from(value)
        .map_err(|_| StorageError::InvalidData(format!("invalid source line: {value}")))
}

fn stored_mana_value(value: f64) -> Result<f64, StorageError> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(StorageError::InvalidData(format!(
            "invalid mana value: {value}"
        )))
    }
}

fn insert_facets(
    statement: &mut rusqlite::Statement<'_>,
    uuid: &str,
    kind: &str,
    values: &[String],
) -> Result<(), rusqlite::Error> {
    for (position, value) in values.iter().enumerate() {
        statement.execute(params![uuid, kind, position as i64, value])?;
    }
    Ok(())
}
