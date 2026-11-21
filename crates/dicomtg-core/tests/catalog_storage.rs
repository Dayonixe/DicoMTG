use std::{
    error::Error,
    fs,
    io::Cursor,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use dicomtg_core::{
    CollectionStore, build_collection, import_dragonshield_csv, import_mtgjson_cards_csv,
};

#[test]
fn cat_001_catalog_round_trip_feeds_matching_and_preserves_collection() -> Result<(), Box<dyn Error>>
{
    let mut store = CollectionStore::in_memory()?;
    let catalog =
        import_mtgjson_cards_csv(Cursor::new(include_bytes!("fixtures/mtgjson_cards.csv")))?;

    store.replace_catalog(&catalog)?;
    store.replace_catalog(&catalog)?;
    let loaded = store.load_catalog()?;
    assert_eq!(loaded, catalog.cards());

    let candidates = loaded
        .iter()
        .map(|card| card.matching_candidate())
        .collect::<Vec<_>>();
    let collection_csv = "Quantity,Card Name,Set Code,Card Number\n2,Admonition Angel,WWK,1\n";
    let report = import_dragonshield_csv(Cursor::new(collection_csv))?;
    let collection = build_collection(report, &candidates)?;
    assert_eq!(collection.items()[0].uuid(), "admonition-uuid");
    store.replace_collection(&collection)?;

    store.replace_catalog(&catalog)?;
    assert_eq!(store.load_collection()?, collection);
    Ok(())
}

#[test]
fn cat_002_migrates_an_existing_v1_database() -> Result<(), Box<dyn Error>> {
    let path = temporary_database_path()?;
    let connection = rusqlite::Connection::open(&path)?;
    connection.execute_batch(
        "CREATE TABLE schema_migrations (
             version INTEGER PRIMARY KEY,
             applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
         );",
    )?;
    connection.execute_batch(include_str!("../migrations/0001_collection.up.sql"))?;
    connection.execute("INSERT INTO schema_migrations (version) VALUES (1)", [])?;
    drop(connection);

    let mut store = CollectionStore::open(&path)?;
    let catalog = single_card_catalog("migrated", "Abrade", "DOM", "117")?;
    store.replace_catalog(&catalog)?;
    assert_eq!(store.load_catalog()?, catalog.cards());

    drop(store);
    fs::remove_file(path)?;
    Ok(())
}

#[test]
fn cat_002_failed_catalog_replace_keeps_the_previous_catalog() -> Result<(), Box<dyn Error>> {
    let path = temporary_database_path()?;
    let mut store = CollectionStore::open(&path)?;
    let initial = single_card_catalog("initial", "Abrade", "DOM", "117")?;
    store.replace_catalog(&initial)?;

    let guard = rusqlite::Connection::open(&path)?;
    guard.execute_batch(
        "CREATE TRIGGER reject_blocked_catalog_uuid
         BEFORE INSERT ON catalog_cards
         WHEN NEW.uuid = 'blocked'
         BEGIN
             SELECT RAISE(ABORT, 'forced failure');
         END;",
    )?;
    drop(guard);

    let replacement = single_card_catalog("blocked", "Opt", "DOM", "60")?;
    assert!(store.replace_catalog(&replacement).is_err());
    assert_eq!(store.load_catalog()?, initial.cards());

    drop(store);
    fs::remove_file(path)?;
    Ok(())
}

fn single_card_catalog(
    uuid: &str,
    name: &str,
    set_code: &str,
    number: &str,
) -> Result<dicomtg_core::CatalogImport, Box<dyn Error>> {
    let csv = format!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid\n,,nonfoil,,English,,1.0,{name},{number},,common,{set_code},,,Instant,Instant,{uuid}\n"
    );
    Ok(import_mtgjson_cards_csv(Cursor::new(csv))?)
}

fn temporary_database_path() -> Result<PathBuf, Box<dyn Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(std::env::temp_dir().join(format!(
        "dicomtg-catalog-{}-{unique}.sqlite",
        std::process::id()
    )))
}
