use std::collections::{BTreeMap, BTreeSet};

use crate::{LocalizedCardNames, LocalizedSetNames};

use super::{CollectionStore, OptionalExtension, StorageError, params};

pub(super) fn localized_search_name(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len());
    for letter in name.chars().flat_map(char::to_lowercase) {
        normalized.push(match letter {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ý' | 'ÿ' => 'y',
            'ç' => 'c',
            'ñ' => 'n',
            '’' => '\'',
            'œ' => {
                normalized.push_str("oe");
                continue;
            }
            _ => letter,
        });
    }
    normalized
}

impl CollectionStore {
    pub fn localized_card_names<'a>(
        &self,
        names: impl IntoIterator<Item = &'a str>,
    ) -> Result<BTreeMap<String, LocalizedCardNames>, StorageError> {
        let names: BTreeSet<_> = names.into_iter().collect();
        if names.is_empty() {
            return Ok(BTreeMap::new());
        }
        let names = serde_json::to_string(&names)
            .map_err(|error| StorageError::InvalidData(error.to_string()))?;
        let mut stmt = self.connection.prepare_cached(
            "SELECT card.card_name, card.language, card.printed_name
             FROM json_each(?1) AS requested
             CROSS JOIN localized_card_names AS card ON card.card_name = requested.value
             ORDER BY card.card_name, card.language, card.position",
        )?;
        let mut result = BTreeMap::<String, LocalizedCardNames>::new();
        for row in stmt.query_map([names], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))? {
            let (name, language, printed_name): (String, String, String) = row?;
            result
                .entry(name)
                .or_default()
                .entry(language)
                .or_default()
                .push(printed_name);
        }
        Ok(result)
    }

    pub fn localized_set_names(&self) -> Result<LocalizedSetNames, StorageError> {
        let mut stmt = self
            .connection
            .prepare_cached("SELECT set_code, language, printed_name FROM localized_set_names")?;
        let mut sets = LocalizedSetNames::new();
        for row in stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))? {
            let (code, language, name): (String, String, String) = row?;
            sets.entry(code).or_default().insert(language, name);
        }
        Ok(sets)
    }

    pub(crate) fn game_localization_timestamp(&self) -> Result<Option<i64>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT updated_at FROM game_localization_refresh WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .optional()?)
    }

    pub(crate) fn replace_game_localization(
        &self,
        cards: &BTreeMap<String, LocalizedCardNames>,
        sets: &LocalizedSetNames,
        updated_at: i64,
    ) -> Result<(), StorageError> {
        let (_guard, transaction) = self.begin_write()?;
        transaction.execute("DELETE FROM localized_card_names", [])?;
        transaction.execute("DELETE FROM localized_set_names", [])?;
        {
            let mut card_stmt = transaction.prepare(
                "INSERT INTO localized_card_names (card_name, language, position, printed_name, search_name)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for (name, languages) in cards {
                for (language, printed_names) in languages {
                    for (position, printed_name) in printed_names.iter().enumerate() {
                        card_stmt.execute(params![
                            name,
                            language,
                            position,
                            printed_name,
                            localized_search_name(printed_name)
                        ])?;
                    }
                }
            }
            let mut set_stmt = transaction.prepare(
                "INSERT INTO localized_set_names (set_code, language, printed_name) VALUES (?1, ?2, ?3)",
            )?;
            for (code, languages) in sets {
                for (language, name) in languages {
                    set_stmt.execute(params![code, language, name])?;
                }
            }
        }
        transaction.execute(
            "INSERT INTO game_localization_refresh (singleton, updated_at) VALUES (1, ?1)
             ON CONFLICT (singleton) DO UPDATE SET updated_at = excluded.updated_at",
            [updated_at],
        )?;
        transaction.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::import_mtgjson_cards_csv;

    use super::*;

    #[test]
    fn finds_deck_cards_by_a_printed_name_and_keeps_the_canonical_card()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut store = CollectionStore::in_memory()?;
        let catalog = import_mtgjson_cards_csv(
            &include_bytes!("../../tests/fixtures/mtgjson_cards.csv")[..],
        )?;
        store.replace_catalog(&catalog)?;
        let cards = BTreeMap::from([(
            "Admonition Angel".to_owned(),
            BTreeMap::from([("fr".to_owned(), vec!["Ange d’admonition".to_owned()])]),
        )]);
        let sets = BTreeMap::from([(
            "WWK".to_owned(),
            BTreeMap::from([("fr".to_owned(), "Worldwake".to_owned())]),
        )]);
        store.replace_game_localization(&cards, &sets, 42)?;

        let results = store.find_deck_cards("ANGE D'ADMONITION", &[], false)?;
        assert_eq!(results.len(), 1);
        let result = serde_json::to_value(&results[0])?;
        assert_eq!(result["name"], "Admonition Angel");
        assert_eq!(result["localizedNames"]["fr"][0], "Ange d’admonition");
        assert_eq!(store.localized_set_names()?, sets);
        assert_eq!(store.game_localization_timestamp()?, Some(42));
        assert!(
            store
                .find_deck_cards("ange d’admonition", &[], true)?
                .is_empty()
        );
        assert_eq!(localized_search_name("Éphémère"), "ephemere");

        store.replace_catalog(&catalog)?;
        assert_eq!(store.localized_card_names(["Admonition Angel"])?, cards);
        Ok(())
    }
}
