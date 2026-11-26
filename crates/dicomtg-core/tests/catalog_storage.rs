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

#[test]
fn edhrec_stats_use_the_full_catalog_and_preserve_zero() -> Result<(), Box<dyn Error>> {
    let header = "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,edhrecSaltiness,edhrecRank\n";
    let first = "U,U,nonfoil,,English,{U},1,Opt,60,,common,DOM,,,Instant,Instant,first,0,1\n";
    let csv = format!(
        "{header}{first}\
        U,U,nonfoil,,English,{{2}}{{U}},3,Rhystic Study,45,,common,PCY,,,Enchantment,Enchantment,original,2.38,1871.0\n\
        U,U,foil,,English,{{2}}{{U}},3,Rhystic Study,25,,mythic,WOT,,,Enchantment,Enchantment,reprint,,\n\
        U,U,nonfoil,,English,{{2}}{{U}},3,Abhorrent Oculus,42,5,mythic,DSK,,5,Creature,Creature,oculus,0.43,5679.0\n\
        U,U,nonfoil,,English,{{U}},1,Consider,44,,common,MID,,,Instant,Instant,last,,10001.00\n\
        U,U,nonfoil,,English,{{U}},1,Preordain,70,,common,M11,,,Sorcery,Sorcery,invalid,5,15.5\n"
    );
    let mut store = CollectionStore::in_memory()?;
    let catalog = import_mtgjson_cards_csv(Cursor::new(csv))?;
    store.replace_catalog(&catalog)?;

    assert_eq!(store.load_catalog()?, catalog.cards());
    let original = store.card_edhrec_stats("original")?;
    assert_eq!(original.salt_score, Some(2.38));
    assert_eq!(original.rank, Some(1871));
    assert!((original.rank_ratio.unwrap() - 0.813).abs() < 1e-12);
    assert_eq!(store.card_edhrec_stats("reprint")?, original);
    assert_eq!(store.card_edhrec_stats("first")?.salt_score, Some(0.0));
    assert_eq!(store.card_edhrec_stats("first")?.rank_ratio, Some(1.0));
    assert_eq!(store.card_edhrec_stats("last")?.rank_ratio, Some(0.0));
    assert_eq!(store.card_edhrec_stats("last")?.salt_score, None);
    assert_eq!(store.card_edhrec_stats("oculus")?.rank, Some(5679));
    assert_eq!(store.card_edhrec_stats("oculus")?.salt_score, Some(0.43));
    assert_eq!(store.card_edhrec_stats("invalid")?, Default::default());
    assert_eq!(store.card_edhrec_stats("unknown")?, Default::default());

    let single_rank = import_mtgjson_cards_csv(Cursor::new(format!("{header}{first}")))?;
    store.replace_catalog(&single_rank)?;
    assert_eq!(store.card_edhrec_stats("first")?.rank_ratio, Some(1.0));
    Ok(())
}

#[test]
fn edhrec_rank_upgrade_schedules_one_refresh_and_keeps_the_catalog() -> Result<(), Box<dyn Error>> {
    let path = temporary_database_path()?;
    let mut store = CollectionStore::open(&path)?;
    let catalog = single_card_catalog("oculus", "Abhorrent Oculus", "DSK", "42")?;
    store.replace_catalog(&catalog)?;
    drop(store);

    let connection = rusqlite::Connection::open(&path)?;
    connection.execute("DELETE FROM schema_migrations WHERE version = 13", [])?;
    connection.execute(
        "INSERT INTO reference_update_state (singleton, last_successful_date)
         VALUES (1, '2026-11-26')",
        [],
    )?;
    drop(connection);

    let mut store = CollectionStore::open(&path)?;
    assert_eq!(store.load_catalog()?, catalog.cards());
    assert_eq!(store.last_reference_update_date()?, None);
    store.set_last_reference_update_date("2026-11-26")?;
    drop(store);

    let store = CollectionStore::open(&path)?;
    assert_eq!(
        store.last_reference_update_date()?.as_deref(),
        Some("2026-11-26")
    );
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
