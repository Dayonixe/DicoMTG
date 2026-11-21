use std::{
    error::Error,
    fs,
    io::Cursor,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use dicomtg_core::{CollectionStore, PriceImport, import_mtgjson_prices_json};

#[test]
fn price_003_price_round_trip_is_idempotent() -> Result<(), Box<dyn Error>> {
    let mut store = CollectionStore::in_memory()?;
    let prices =
        import_mtgjson_prices_json(Cursor::new(include_bytes!("fixtures/mtgjson_prices.json")))?;

    store.replace_prices(&prices)?;
    store.replace_prices(&prices)?;

    assert_eq!(store.load_prices()?, prices.prices());
    Ok(())
}

#[test]
fn price_003_migrates_an_existing_v2_database() -> Result<(), Box<dyn Error>> {
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
    connection.execute(
        "INSERT INTO schema_migrations (version) VALUES (1), (2)",
        [],
    )?;
    drop(connection);

    let mut store = CollectionStore::open(&path)?;
    let prices = single_price_import("migrated", "1.25")?;
    store.replace_prices(&prices)?;
    assert_eq!(store.load_prices()?, prices.prices());

    drop(store);
    fs::remove_file(path)?;
    Ok(())
}

#[test]
fn price_003_failed_replace_keeps_the_previous_prices() -> Result<(), Box<dyn Error>> {
    let path = temporary_database_path()?;
    let mut store = CollectionStore::open(&path)?;
    let initial = single_price_import("initial", "1.25")?;
    store.replace_prices(&initial)?;

    let guard = rusqlite::Connection::open(&path)?;
    guard.execute_batch(
        "CREATE TRIGGER reject_blocked_price_uuid
         BEFORE INSERT ON reference_prices
         WHEN NEW.uuid = 'blocked'
         BEGIN
             SELECT RAISE(ABORT, 'forced failure');
         END;",
    )?;
    drop(guard);

    let replacement = single_price_import("blocked", "2.50")?;
    assert!(store.replace_prices(&replacement).is_err());
    assert_eq!(store.load_prices()?, initial.prices());

    drop(store);
    fs::remove_file(path)?;
    Ok(())
}

fn single_price_import(uuid: &str, amount: &str) -> Result<PriceImport, Box<dyn Error>> {
    let json = r#"{"data":{"$UUID":{"paper":{"cardmarket":{"currency":"EUR","retail":{"normal":{"2026-11-15":$AMOUNT}}}}}}}"#
        .replace("$UUID", uuid)
        .replace("$AMOUNT", amount);
    Ok(import_mtgjson_prices_json(Cursor::new(json))?)
}

fn temporary_database_path() -> Result<PathBuf, Box<dyn Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(std::env::temp_dir().join(format!(
        "dicomtg-prices-{}-{unique}.sqlite",
        std::process::id()
    )))
}
