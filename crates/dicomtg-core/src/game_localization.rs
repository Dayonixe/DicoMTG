use std::{
    collections::BTreeMap,
    fmt,
    fs::File,
    io::{self, BufReader, Read},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use flate2::read::GzDecoder;
use serde::{
    Deserialize, Deserializer,
    de::{MapAccess, Visitor},
};

use crate::{
    CatalogUpdateError, CollectionStore,
    mtgjson_update::{
        TemporaryArchive, download_archive, download_checksum, http_agent, verify_checksum,
    },
    storage::game_localization::LocalizationRefreshState,
};

const MTGJSON: &str = "https://mtgjson.com/api/v5";
const ATOMIC_CARDS: &str = "AtomicCards.json.gz";
const SET_LIST: &str = "SetList.json.gz";
const MAX_COMPRESSED_BYTES: u64 = 64 * 1024 * 1024;
const MAX_DECOMPRESSED_BYTES: u64 = 512 * 1024 * 1024;
const REFRESH_INTERVAL_SECONDS: i64 = 7 * 24 * 60 * 60;

pub type LocalizedCardNames = BTreeMap<String, Vec<String>>;
pub type LocalizedSetNames = BTreeMap<String, BTreeMap<String, String>>;

#[derive(Deserialize)]
struct Export<T> {
    data: T,
}

#[derive(Deserialize)]
struct AtomicCardNames(
    #[serde(deserialize_with = "deserialize_card_names")] BTreeMap<String, LocalizedCardNames>,
);

fn deserialize_card_names<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, LocalizedCardNames>, D::Error>
where
    D: Deserializer<'de>,
{
    struct CardsVisitor;

    impl<'de> Visitor<'de> for CardsVisitor {
        type Value = BTreeMap<String, LocalizedCardNames>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("MTGJSON atomic cards grouped by canonical name")
        }

        fn visit_map<M: MapAccess<'de>>(self, mut input: M) -> Result<Self::Value, M::Error> {
            let mut cards = BTreeMap::new();
            // Keep only the useful aliases between records, not the parsed faces of the whole export.
            while let Some((name, faces)) = input.next_entry::<String, Vec<AtomicFace>>()? {
                let names = card_names_for(&name, &faces);
                if !names.is_empty() {
                    cards.insert(name, names);
                }
            }
            Ok(cards)
        }
    }

    deserializer.deserialize_map(CardsVisitor)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AtomicFace {
    face_name: Option<String>,
    foreign_data: Option<Vec<ForeignName>>,
}

#[derive(Deserialize)]
struct ForeignName {
    language: String,
    name: Option<String>,
}

#[derive(Deserialize)]
struct Set {
    code: String,
    name: String,
    translations: Option<BTreeMap<String, Option<String>>>,
}

fn language_code(language: &str) -> Option<&'static str> {
    match language {
        "English" | "en" => Some("en"),
        "French" | "fr" => Some("fr"),
        "German" | "de" => Some("de"),
        "Spanish" | "es" => Some("es"),
        "Italian" | "it" => Some("it"),
        "Portuguese" | "Portuguese (Brazil)" | "pt" | "pt-BR" => Some("pt"),
        "Japanese" | "ja" => Some("ja"),
        _ => None,
    }
}

fn add_name(names: &mut Vec<String>, name: &str) {
    let name = name.trim();
    if !name.is_empty() && !names.iter().any(|existing| existing == name) {
        names.push(name.to_owned());
    }
}

fn card_names_for(name: &str, faces: &[AtomicFace]) -> LocalizedCardNames {
    let face_order: Vec<_> = name.split(" // ").collect();
    let mut languages = LocalizedCardNames::new();
    let mut face_names = BTreeMap::<_, BTreeMap<&str, &str>>::new();
    for face in faces {
        for foreign in face.foreign_data.as_deref().unwrap_or_default() {
            let (Some(language), Some(printed_name)) = (
                language_code(&foreign.language),
                foreign.name.as_deref().map(str::trim),
            ) else {
                continue;
            };
            if printed_name.is_empty() || language == "en" {
                continue;
            }
            let aliases = languages.entry(language.to_owned()).or_default();
            if let Some(face_name) = face.face_name.as_deref()
                && face_order.len() > 1
                && !printed_name.contains(" // ")
            {
                face_names
                    .entry(language)
                    .or_default()
                    .entry(face_name)
                    .or_insert(printed_name);
            }
            add_name(aliases, printed_name);
        }
    }
    for (language, aliases) in &mut languages {
        let combined = aliases
            .iter()
            .find(|alias| alias.contains(" // "))
            .cloned()
            .or_else(|| {
                let localized = face_names.get(language.as_str())?;
                let faces: Option<Vec<_>> = face_order
                    .iter()
                    .map(|face| localized.get(face).copied())
                    .collect();
                Some(faces?.join(" // "))
            });
        if let Some(combined) = combined {
            aliases.retain(|alias| alias != &combined);
            aliases.insert(0, combined);
        } else if face_order.len() > 1 {
            // A single translated face is searchable, but cannot label the whole card.
            aliases.insert(0, name.to_owned());
        }
    }
    languages
}

fn parse_set_names(data: Vec<Set>) -> LocalizedSetNames {
    data.into_iter()
        .filter(|set| !set.code.trim().is_empty() && !set.name.trim().is_empty())
        .map(|set| {
            let names =
                set.translations
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|(language, name)| {
                        let code = language_code(&language)?;
                        let name = name?.trim().to_owned();
                        (!name.is_empty() && code != "en").then(|| (code.to_owned(), name))
                    });
            let mut names: BTreeMap<_, _> = names.collect();
            names.insert("en".to_owned(), set.name);
            (set.code.to_ascii_uppercase(), names)
        })
        .collect()
}

fn download_export<T: serde::de::DeserializeOwned>(
    agent: &ureq::Agent,
    directory: &Path,
    file: &str,
    checksum: &str,
) -> Result<T, CatalogUpdateError> {
    let url = format!("{MTGJSON}/{file}");
    let (archive, mut target) = TemporaryArchive::create(directory, file)?;
    download_archive(agent, &url, &mut target, MAX_COMPRESSED_BYTES)?;
    drop(target);
    verify_checksum(archive.path(), checksum)?;
    let mut reader = BufReader::new(GzDecoder::new(File::open(archive.path())?))
        .take(MAX_DECOMPRESSED_BYTES + 1);
    let export = serde_json::from_reader::<_, Export<T>>(&mut reader);
    if reader.limit() == 0 {
        return Err(CatalogUpdateError::DecompressedTooLarge {
            limit: MAX_DECOMPRESSED_BYTES,
        });
    }
    Ok(export
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?
        .data)
}

fn changed_export<T>(
    previous: Option<&str>,
    checksum: &str,
    download: impl FnOnce() -> Result<T, CatalogUpdateError>,
) -> Result<Option<T>, CatalogUpdateError> {
    (previous != Some(checksum)).then(download).transpose()
}

/// Refresh the independent translation cache without holding the SQLite writer during downloads.
pub fn update_game_localization(
    store: &CollectionStore,
    directory: &Path,
    force: bool,
) -> Result<bool, CatalogUpdateError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let previous = store.game_localization_refresh_state()?;
    if !force
        && previous
            .updated_at
            .is_some_and(|date| (0..REFRESH_INTERVAL_SECONDS).contains(&now.saturating_sub(date)))
    {
        return Ok(false);
    }
    let agent = http_agent();
    let cards_checksum = download_checksum(&agent, &format!("{MTGJSON}/{ATOMIC_CARDS}.sha256"))?;
    let sets_checksum = download_checksum(&agent, &format!("{MTGJSON}/{SET_LIST}.sha256"))?;
    let cards = changed_export(previous.cards_checksum.as_deref(), &cards_checksum, || {
        std::fs::create_dir_all(directory)?;
        download_export(&agent, directory, ATOMIC_CARDS, &cards_checksum)
    })?
    .map(|AtomicCardNames(cards)| cards);
    let sets = changed_export(previous.sets_checksum.as_deref(), &sets_checksum, || {
        std::fs::create_dir_all(directory)?;
        download_export(&agent, directory, SET_LIST, &sets_checksum)
    })?
    .map(parse_set_names);
    if cards.as_ref().is_some_and(BTreeMap::is_empty)
        || sets.as_ref().is_some_and(BTreeMap::is_empty)
    {
        return Err(
            io::Error::new(io::ErrorKind::InvalidData, "Catalogue de localisation vide").into(),
        );
    }
    Ok(store.replace_game_localization(
        cards.as_ref(),
        sets.as_ref(),
        previous.revision,
        &LocalizationRefreshState {
            updated_at: Some(now),
            cards_checksum: Some(cards_checksum),
            sets_checksum: Some(sets_checksum),
            ..LocalizationRefreshState::default()
        },
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downloads_only_changed_archives() -> Result<(), CatalogUpdateError> {
        assert_eq!(
            changed_export::<u32>(Some("same"), "same", || panic!("unchanged export"))?,
            None
        );
        assert_eq!(
            changed_export(Some("before"), "after", || Ok(42))?,
            Some(42)
        );
        assert_eq!(changed_export(None, "new", || Ok(3))?, Some(3));
        Ok(())
    }

    #[test]
    fn keeps_official_names_and_joins_two_printed_faces() -> Result<(), Box<dyn std::error::Error>>
    {
        let export = serde_json::from_str::<Export<AtomicCardNames>>(
            r#"{"meta":{"version":"5.0.0"},"data":{
            "Abrade":[{"foreignData":[{"language":"French","name":"Fissuration"},{"language":"German","name":"Abrieb"}]}],
            "Front // Back":[
                {"faceName":"Front","foreignData":[{"language":"French","name":"Avant"},{"language":"Spanish","name":"Anverso"}]},
                {"faceName":"Back","foreignData":[{"language":"French","name":"Arrière"}]}
            ],
            "Only English":[{}]}}"#,
        )?;
        let AtomicCardNames(cards) = export.data;
        assert_eq!(cards["Abrade"]["fr"], ["Fissuration"]);
        assert_eq!(cards["Abrade"]["de"], ["Abrieb"]);
        assert_eq!(
            cards["Front // Back"]["fr"],
            ["Avant // Arrière", "Avant", "Arrière"]
        );
        assert_eq!(cards["Front // Back"]["es"], ["Front // Back", "Anverso"]);
        assert!(!cards.contains_key("Only English"));
        Ok(())
    }
}
