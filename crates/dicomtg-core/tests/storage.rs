use std::{
    error::Error,
    fs,
    io::Cursor,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use dicomtg_core::{
    CardCandidate, CardFinish, CollectionSnapshot, CollectionStore, build_collection,
    import_dragonshield_csv,
};

#[test]
fn col_006_replace_is_idempotent_and_round_trips_diagnostics() -> Result<(), Box<dyn Error>> {
    let mut store = CollectionStore::in_memory()?;
    let collection = diagnostic_collection()?;

    store.replace_collection(&collection)?;
    store.replace_collection(&collection)?;

    assert_eq!(store.load_collection()?, collection);
    Ok(())
}

#[test]
fn col_006_failed_replace_keeps_the_previous_collection() -> Result<(), Box<dyn Error>> {
    let path = temporary_database_path()?;
    let mut store = CollectionStore::open(&path)?;
    let initial = matched_collection("initial", "Abrade", "DOM", "117")?;
    store.replace_collection(&initial)?;

    let guard = rusqlite::Connection::open(&path)?;
    guard.execute_batch(
        "CREATE TRIGGER reject_blocked_uuid
         BEFORE INSERT ON collection_items
         WHEN NEW.uuid = 'blocked'
         BEGIN
             SELECT RAISE(ABORT, 'forced failure');
         END;",
    )?;
    drop(guard);

    let replacement = matched_collection("blocked", "Opt", "DOM", "60")?;
    assert!(store.replace_collection(&replacement).is_err());
    assert_eq!(store.load_collection()?, initial);

    drop(store);
    let reopened = CollectionStore::open(&path)?;
    assert_eq!(reopened.load_collection()?, initial);
    drop(reopened);
    fs::remove_file(path)?;
    Ok(())
}

#[test]
fn col_009_manages_and_merges_selected_named_collections() -> Result<(), Box<dyn Error>> {
    let mut store = CollectionStore::in_memory()?;
    let own = matched_collection("abrade", "Abrade", "DOM", "117")?;
    let friend = matched_collection_with_quantity("abrade", "Abrade", "DOM", "117", 3)?;
    store.replace_collection(&own)?;

    let friend_id = store.create_collection("Collection de Léa", &friend)?;

    assert_eq!(store.load_collection()?.items()[0].quantity(), 4);
    assert_eq!(store.collections()?.len(), 2);
    assert_eq!(store.collections()?[0].name(), "Ma collection");
    assert!(store.collections()?[0].selected());
    assert_eq!(store.collections()?[1].total_quantity(), 3);

    store.set_collection_selected(friend_id, false)?;
    assert_eq!(store.load_collection()?.items()[0].quantity(), 1);

    store.rename_collection(friend_id, "Collection de Prawse")?;
    let replacement = matched_collection_with_quantity("abrade", "Abrade", "DOM", "117", 7)?;
    store.replace_collection_for(friend_id, &replacement)?;
    store.set_collection_selected(friend_id, true)?;

    assert_eq!(store.load_collection()?.items()[0].quantity(), 8);
    assert_eq!(
        store.load_collection_by_id(friend_id)?.items()[0].quantity(),
        7
    );
    assert_eq!(store.collections()?[1].name(), "Collection de Prawse");
    Ok(())
}

#[test]
fn col_009_rejects_blank_and_duplicate_collection_names() -> Result<(), Box<dyn Error>> {
    let mut store = CollectionStore::in_memory()?;
    let collection = matched_collection("abrade", "Abrade", "DOM", "117")?;

    assert!(store.create_collection("   ", &collection).is_err());
    assert!(
        store
            .create_collection("ma COLLECTION", &collection)
            .is_err()
    );
    Ok(())
}

#[test]
fn col_004_migrates_an_existing_v3_database_without_losing_collection_prices_or_diagnostics()
-> Result<(), Box<dyn Error>> {
    let path = temporary_database_path()?;
    let connection = rusqlite::Connection::open(&path)?;
    connection.execute_batch(
        "CREATE TABLE schema_migrations (
             version INTEGER PRIMARY KEY,
             applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
         );",
    )?;
    connection.execute_batch(include_str!("../migrations/0001_collection.up.sql"))?;
    connection.execute_batch(include_str!("../migrations/0002_catalog.up.sql"))?;
    connection.execute_batch(include_str!("../migrations/0003_prices.up.sql"))?;
    connection.execute(
        "INSERT INTO schema_migrations (version) VALUES (1), (2), (3)",
        [],
    )?;
    connection.execute(
        "INSERT INTO collection_items (position, uuid, quantity)
         VALUES (0, 'owned', 2)",
        [],
    )?;
    connection.execute(
        "INSERT INTO reference_prices (uuid, amount_cents, currency, as_of_date)
         VALUES ('owned', 125, 'EUR', '2026-11-18')",
        [],
    )?;
    connection.execute(
        "INSERT INTO unresolved_cards
         (position, name, set_code, collector_number, quantity, match_status)
         VALUES (0, 'Old diagnostic', 'TST', '7', 1, 'ambiguous')",
        [],
    )?;
    connection.execute(
        "INSERT INTO unresolved_candidates (unresolved_id, position, uuid)
         VALUES (1, 0, 'candidate')",
        [],
    )?;
    drop(connection);

    let store = CollectionStore::open(&path)?;
    let collection = store.load_collection()?;

    assert_eq!(store.collections()?.len(), 1);
    assert_eq!(store.collections()?[0].name(), "Ma collection");
    assert!(store.collections()?[0].selected());
    assert_eq!(collection.items().len(), 1);
    assert_eq!(collection.items()[0].uuid(), "owned");
    assert_eq!(collection.items()[0].finish(), CardFinish::Normal);
    assert_eq!(collection.items()[0].quantity(), 2);
    let prices = store.load_prices()?;
    assert_eq!(prices.len(), 1);
    assert_eq!(prices[0].finish(), CardFinish::Normal);
    assert_eq!(prices[0].amount_cents(), 125);
    assert_eq!(collection.unresolved().len(), 1);
    assert_eq!(collection.unresolved()[0].name(), "Old diagnostic");
    assert_eq!(collection.unresolved()[0].language(), None);
    assert_eq!(collection.unresolved()[0].printing(), None);
    assert!(matches!(
        collection.unresolved()[0].match_result(),
        dicomtg_core::MatchResult::Ambiguous { candidate_uuids }
            if candidate_uuids == &["candidate"]
    ));
    drop(store);
    fs::remove_file(path)?;
    Ok(())
}

fn diagnostic_collection() -> Result<CollectionSnapshot, Box<dyn Error>> {
    let csv = concat!(
        "Quantity,Card Name,Set Code,Card Number,Language,Printing\n",
        "2,Abrade,DOM,117,French,Normal\n",
        "1,Unknown Card,TST,404,French,Galaxy Foil\n",
        "1,Shared Name,TST,12,French,Galaxy Foil\n",
        "0,Invalid Quantity,TST,1,French,Normal\n",
    );
    let catalog = [
        CardCandidate::with_language("abrade", "Abrade", "DOM", "117", "French"),
        CardCandidate::with_language("shared-a", "Shared Name", "TST", "12", "French"),
        CardCandidate::with_language("shared-b", "Shared Name", "TST", "12", "French"),
    ];
    let report = import_dragonshield_csv(Cursor::new(csv))?;
    Ok(build_collection(report, &catalog)?)
}

fn matched_collection(
    uuid: &str,
    name: &str,
    set_code: &str,
    collector_number: &str,
) -> Result<CollectionSnapshot, Box<dyn Error>> {
    matched_collection_with_quantity(uuid, name, set_code, collector_number, 1)
}

fn matched_collection_with_quantity(
    uuid: &str,
    name: &str,
    set_code: &str,
    collector_number: &str,
    quantity: u32,
) -> Result<CollectionSnapshot, Box<dyn Error>> {
    let csv = format!(
        "Quantity,Card Name,Set Code,Card Number\n{quantity},{name},{set_code},{collector_number}\n"
    );
    let catalog = [CardCandidate::new(uuid, name, set_code, collector_number)];
    let report = import_dragonshield_csv(Cursor::new(csv))?;
    Ok(build_collection(report, &catalog)?)
}

fn temporary_database_path() -> Result<PathBuf, Box<dyn Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(std::env::temp_dir().join(format!("dicomtg-{}-{unique}.sqlite", std::process::id())))
}
