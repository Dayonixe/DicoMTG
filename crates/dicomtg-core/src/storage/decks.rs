use serde::Serialize;

use crate::{Deck, SavedDeck};

use super::{CollectionStore, OptionalExtension, StorageError, params};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckCardInfo {
    uuid: String,
    scryfall_id: Option<String>,
    has_back_face: bool,
    name: String,
    set_code: String,
    collector_number: String,
    mana_value: f64,
    type_line: String,
    rules_text: Option<String>,
    rarity: String,
    colors: Vec<String>,
    color_identity: Vec<String>,
    keywords: Vec<String>,
    mana_cost: Option<String>,
    power: Option<String>,
    toughness: Option<String>,
    types: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    requested_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeckCollection {
    id: i64,
    name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeckHolding {
    collection_id: i64,
    name: String,
    quantity: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckWorkspace {
    collections: Vec<DeckCollection>,
    holdings: Vec<DeckHolding>,
    candidates: Vec<DeckCardInfo>,
}

// Join facets by card UUID before partitioning them by kind.
const CARD_DETAILS: &str = "
    SELECT card.name, card.set_code, card.collector_number, card.mana_value,
           card.type_line, card.rules_text, card.rarity,
           group_concat(facet.value, char(31) ORDER BY facet.position) FILTER (WHERE facet.kind = 'color'),
           group_concat(facet.value, char(31) ORDER BY facet.position) FILTER (WHERE facet.kind = 'color_identity'),
           group_concat(facet.value, char(31) ORDER BY facet.position) FILTER (WHERE facet.kind = 'keyword'),
           card.uuid, card.scryfall_id, card.has_back_face, card.mana_cost, card.power, card.toughness,
           group_concat(facet.value, char(31) ORDER BY facet.position) FILTER (WHERE facet.kind = 'type'),
           candidates.requested_name
    FROM candidates
    CROSS JOIN catalog_cards card ON card.uuid = candidates.uuid
    LEFT JOIN catalog_facets facet ON facet.card_uuid = card.uuid
    GROUP BY card.uuid, candidates.requested_name";

fn read_card(row: &rusqlite::Row<'_>) -> rusqlite::Result<DeckCardInfo> {
    let list = |index| -> rusqlite::Result<Vec<String>> {
        let value: Option<String> = row.get(index)?;
        Ok(value.map_or_else(Vec::new, |text| {
            text.split('\x1f').map(str::to_owned).collect()
        }))
    };
    Ok(DeckCardInfo {
        name: row.get(0)?,
        set_code: row.get(1)?,
        collector_number: row.get(2)?,
        mana_value: row.get(3)?,
        type_line: row.get(4)?,
        rules_text: row.get(5)?,
        rarity: row.get(6)?,
        colors: list(7)?,
        color_identity: list(8)?,
        keywords: list(9)?,
        uuid: row.get(10)?,
        scryfall_id: row.get(11)?,
        has_back_face: row.get(12)?,
        mana_cost: row.get(13)?,
        power: row.get(14)?,
        toughness: row.get(15)?,
        types: list(16)?,
        requested_name: row.get(17)?,
    })
}

fn deck_data_error(error: serde_json::Error) -> StorageError {
    StorageError::InvalidData(error.to_string())
}

impl CollectionStore {
    pub fn list_saved_decks(&self) -> Result<Vec<SavedDeck>, StorageError> {
        self.connection
            .prepare_cached("SELECT id, revision, document FROM decks ORDER BY id")?
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .map(|row| -> Result<SavedDeck, StorageError> {
                let (id, revision, document) = row?;
                Ok(SavedDeck {
                    id,
                    revision,
                    deck: serde_json::from_str(&document).map_err(deck_data_error)?,
                })
            })
            .collect()
    }

    pub fn load_deck_workspace(&self) -> Result<DeckWorkspace, StorageError> {
        let transaction = self.connection.unchecked_transaction()?;
        let collections = transaction
            .prepare("SELECT id, name FROM collections ORDER BY id")?
            .query_map([], |row| {
                Ok(DeckCollection {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let holdings = transaction
            .prepare(
                "SELECT collection_id, name, SUM(quantity) FROM (
                SELECT item.collection_id, card.name, item.quantity
                FROM collection_items item JOIN catalog_cards card ON card.uuid = item.uuid
                UNION ALL
                SELECT collection_id, name, quantity FROM unresolved_cards
            ) GROUP BY collection_id, name COLLATE NOCASE",
            )?
            .query_map([], |row| {
                Ok(DeckHolding {
                    collection_id: row.get(0)?,
                    name: row.get(1)?,
                    quantity: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        // Drive the catalog lookup from the owned UUIDs.
        let candidates = transaction.prepare(&format!(
            "WITH owned AS (
                SELECT uuid FROM collection_items
                UNION SELECT uuid FROM catalog_cards
                WHERE name COLLATE NOCASE IN ('Plains', 'Island', 'Swamp', 'Mountain', 'Forest', 'Wastes')
            ), candidates AS (
                SELECT MIN(card.uuid) AS uuid, NULL AS requested_name
                FROM owned CROSS JOIN catalog_cards card ON card.uuid = owned.uuid
                WHERE card.rarity <> 'token'
                GROUP BY card.name COLLATE NOCASE
            ) {CARD_DETAILS} ORDER BY card.name COLLATE NOCASE",
        ))?
            .query_map([], read_card)?
            .collect::<Result<Vec<_>, _>>()?;
        transaction.commit()?;
        Ok(DeckWorkspace {
            collections,
            holdings,
            candidates,
        })
    }

    pub fn find_deck_cards(
        &self,
        search: &str,
        names: &[String],
        commander: bool,
    ) -> Result<Vec<DeckCardInfo>, StorageError> {
        let search = search.trim();
        if search.is_empty() {
            if names.is_empty() {
                return Ok(Vec::new());
            }
            let names = serde_json::to_string(names).map_err(deck_data_error)?;
            // Indexed prefix bounds cover every printing whose name starts with the supplied front face.
            // Prefer an exact name, and leave ambiguous front names unresolved.
            let sql = format!(
                "WITH requested(name) AS (SELECT DISTINCT value FROM json_each(?1)),
                 exact AS (
                   SELECT MIN(card.uuid) AS uuid, requested.name AS requested_name
                   FROM requested JOIN catalog_cards card ON card.name COLLATE NOCASE = requested.name
                   WHERE card.rarity <> 'token' GROUP BY requested.name COLLATE NOCASE
                 ), candidates AS (
                   SELECT * FROM exact
                   UNION ALL
                   SELECT MIN(card.uuid), requested.name
                   FROM requested JOIN catalog_cards card
                     ON card.name COLLATE NOCASE >= (requested.name || ' // ')
                    AND card.name COLLATE NOCASE < (requested.name || ' //!')
                   WHERE card.rarity <> 'token' AND NOT EXISTS (
                     SELECT 1 FROM exact WHERE requested_name COLLATE NOCASE = requested.name
                   )
                   GROUP BY requested.name COLLATE NOCASE
                   HAVING COUNT(DISTINCT card.name COLLATE NOCASE) = 1
                 ) {CARD_DETAILS}",
            );
            let mut statement = self.connection.prepare_cached(&sql)?;
            return Ok(statement
                .query_map([names], read_card)?
                .collect::<Result<_, _>>()?);
        }
        if search.chars().count() < 2 {
            return Ok(Vec::new());
        }
        let mut stmt = self.connection.prepare(&format!(
            "WITH candidates AS (
                 SELECT MIN(card.uuid) AS uuid, NULL AS requested_name
                 FROM catalog_cards card
                 WHERE card.rarity <> 'token'
                   AND instr(lower(card.name), lower(?1)) > 0
                   AND (NOT ?2 OR (instr(card.type_line, 'Legendary') > 0
                                   AND instr(card.type_line, 'Creature') > 0)
                               OR instr(lower(card.rules_text), 'can be your commander') > 0)
                 GROUP BY card.name COLLATE NOCASE
                 ORDER BY instr(lower(card.name), lower(?1)), card.name COLLATE NOCASE
                 LIMIT 24
             ) {CARD_DETAILS}
             ORDER BY instr(lower(card.name), lower(?1)), card.name COLLATE NOCASE",
        ))?;
        let cards = stmt.query_map(params![search, commander], read_card)?;
        Ok(cards.collect::<Result<_, _>>()?)
    }

    pub fn save_deck(
        &mut self,
        current: Option<(i64, i64)>,
        mut deck: Deck,
    ) -> Result<SavedDeck, StorageError> {
        deck.validate().map_err(StorageError::InvalidData)?;
        let document = serde_json::to_string(&deck).map_err(deck_data_error)?;
        let (_writer, transaction) = self.begin_write()?;
        let (id, revision) = if let Some((id, revision)) = current {
            let revision = transaction
                .query_row(
                    "UPDATE decks SET document = ?1, revision = revision + 1
                 WHERE id = ?2 AND revision = ?3 RETURNING revision",
                    params![document, id, revision],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or_else(deck_changed)?;
            (id, revision)
        } else {
            transaction.execute("INSERT INTO decks (document) VALUES (?1)", [document])?;
            (transaction.last_insert_rowid(), 1)
        };
        transaction.commit()?;
        Ok(SavedDeck { id, revision, deck })
    }

    pub fn delete_deck(&mut self, id: i64, revision: i64) -> Result<(), StorageError> {
        let (_writer, transaction) = self.begin_write()?;
        if transaction.execute(
            "DELETE FROM decks WHERE id = ?1 AND revision = ?2",
            [id, revision],
        )? == 0
        {
            return Err(deck_changed());
        }
        transaction.commit()?;
        Ok(())
    }
}

fn deck_changed() -> StorageError {
    StorageError::InvalidData(
        "Ce deck a été modifié ou supprimé. Rechargez la page Deck avant de continuer.".to_owned(),
    )
}
