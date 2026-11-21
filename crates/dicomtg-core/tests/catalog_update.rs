use std::{
    error::Error,
    fs,
    io::{Cursor, Write},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use dicomtg_core::{
    CatalogUpdateError, CollectionStore, import_mtgjson_cards_csv, install_catalog_archive,
    install_catalog_archives, install_catalog_archives_with_identifiers,
};
use flate2::{Compression, write::GzEncoder};
use sha2::{Digest, Sha256};

#[test]
fn cat_002_verified_gzip_replaces_the_catalog() -> Result<(), Box<dyn Error>> {
    let csv = include_bytes!("fixtures/mtgjson_cards.csv");
    let archive = gzip(csv)?;
    let checksum = sha256(&archive);
    let path = temporary_archive_path()?;
    fs::write(&path, &archive)?;
    let expected = import_mtgjson_cards_csv(Cursor::new(csv))?;
    let mut store = CollectionStore::in_memory()?;

    let result = install_catalog_archive(&mut store, &path, &checksum)?;

    assert_eq!(result.card_count(), 2);
    assert_eq!(store.load_catalog()?, expected.cards());
    fs::remove_file(path)?;
    Ok(())
}

#[test]
fn cat_002_checksum_mismatch_keeps_the_previous_catalog() -> Result<(), Box<dyn Error>> {
    let csv = include_bytes!("fixtures/mtgjson_cards.csv");
    let archive = gzip(csv)?;
    let path = temporary_archive_path()?;
    fs::write(&path, archive)?;
    let initial = import_mtgjson_cards_csv(Cursor::new(csv))?;
    let mut store = CollectionStore::in_memory()?;
    store.replace_catalog(&initial)?;

    let error = install_catalog_archive(&mut store, &path, &"0".repeat(64))
        .expect_err("incorrect checksum must be rejected");

    assert!(matches!(error, CatalogUpdateError::ChecksumMismatch { .. }));
    assert_eq!(store.load_catalog()?, initial.cards());
    fs::remove_file(path)?;
    Ok(())
}

#[test]
fn cat_002_rejects_a_malformed_checksum() -> Result<(), Box<dyn Error>> {
    let path = temporary_archive_path()?;
    fs::write(&path, b"not an archive")?;
    let mut store = CollectionStore::in_memory()?;

    let error = install_catalog_archive(&mut store, &path, "not-a-sha256")
        .expect_err("malformed checksum must be rejected");

    assert!(matches!(error, CatalogUpdateError::InvalidChecksum { .. }));
    fs::remove_file(path)?;
    Ok(())
}

#[test]
fn cat_002_corrupt_gzip_keeps_the_previous_catalog() -> Result<(), Box<dyn Error>> {
    let csv = include_bytes!("fixtures/mtgjson_cards.csv");
    let corrupt_archive = b"not a gzip archive";
    let path = temporary_archive_path()?;
    fs::write(&path, corrupt_archive)?;
    let initial = import_mtgjson_cards_csv(Cursor::new(csv))?;
    let mut store = CollectionStore::in_memory()?;
    store.replace_catalog(&initial)?;

    let error = install_catalog_archive(&mut store, &path, &sha256(corrupt_archive))
        .expect_err("a corrupt archive must be rejected");

    assert!(matches!(error, CatalogUpdateError::Catalog(_)));
    assert_eq!(store.load_catalog()?, initial.cards());
    fs::remove_file(path)?;
    Ok(())
}

#[test]
fn cat_005_cards_and_tokens_are_installed_together() -> Result<(), Box<dyn Error>> {
    let cards = gzip(include_bytes!("fixtures/mtgjson_cards.csv"))?;
    let tokens = gzip(token_csv())?;
    let cards_path = temporary_archive_path()?;
    let tokens_path = temporary_archive_path()?;
    fs::write(&cards_path, &cards)?;
    fs::write(&tokens_path, &tokens)?;
    let mut store = CollectionStore::in_memory()?;

    let result = install_catalog_archives(
        &mut store,
        &cards_path,
        &sha256(&cards),
        &tokens_path,
        &sha256(&tokens),
    )?;

    assert_eq!(result.card_count(), 3);
    assert!(
        store
            .load_catalog()?
            .iter()
            .any(|card| card.uuid() == "angel-token")
    );
    fs::remove_file(cards_path)?;
    fs::remove_file(tokens_path)?;
    Ok(())
}

#[test]
fn cat_005_invalid_tokens_keep_the_previous_catalog() -> Result<(), Box<dyn Error>> {
    let csv = include_bytes!("fixtures/mtgjson_cards.csv");
    let cards = gzip(csv)?;
    let tokens = gzip(token_csv())?;
    let cards_path = temporary_archive_path()?;
    let tokens_path = temporary_archive_path()?;
    fs::write(&cards_path, &cards)?;
    fs::write(&tokens_path, &tokens)?;
    let initial = import_mtgjson_cards_csv(Cursor::new(csv))?;
    let mut store = CollectionStore::in_memory()?;
    store.replace_catalog(&initial)?;

    let error = install_catalog_archives(
        &mut store,
        &cards_path,
        &sha256(&cards),
        &tokens_path,
        &"0".repeat(64),
    )
    .expect_err("invalid tokens must reject the whole update");

    assert!(matches!(error, CatalogUpdateError::ChecksumMismatch { .. }));
    assert_eq!(store.load_catalog()?, initial.cards());
    fs::remove_file(cards_path)?;
    fs::remove_file(tokens_path)?;
    Ok(())
}

#[test]
fn cat_006_installs_scryfall_identifiers_with_cards_and_tokens() -> Result<(), Box<dyn Error>> {
    let cards = gzip(include_bytes!("fixtures/mtgjson_cards.csv"))?;
    let tokens = gzip(token_csv())?;
    let card_identifiers = gzip(
        b"uuid,scryfallId,mcmId\nadmonition-uuid,aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa,42\nsplit-uuid,bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb,43\n",
    )?;
    let token_identifiers =
        gzip(b"uuid,scryfallId,mcmId\nangel-token,cccccccc-cccc-4ccc-8ccc-cccccccccccc,44\n")?;
    let paths = [
        temporary_archive_path()?,
        temporary_archive_path()?,
        temporary_archive_path()?,
        temporary_archive_path()?,
    ];
    for (path, bytes) in paths
        .iter()
        .zip([&cards, &tokens, &card_identifiers, &token_identifiers])
    {
        fs::write(path, bytes)?;
    }
    let mut store = CollectionStore::in_memory()?;

    let result = install_catalog_archives_with_identifiers(
        &mut store,
        &paths[0],
        &sha256(&cards),
        &paths[1],
        &sha256(&tokens),
        &paths[2],
        &sha256(&card_identifiers),
        &paths[3],
        &sha256(&token_identifiers),
    )?;

    assert_eq!(result.card_count(), 3);
    let catalog = store.load_catalog()?;
    assert_eq!(
        catalog
            .iter()
            .find(|card| card.uuid() == "admonition-uuid")
            .and_then(|card| card.scryfall_id()),
        Some("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa")
    );
    assert_eq!(
        catalog
            .iter()
            .find(|card| card.uuid() == "angel-token")
            .and_then(|card| card.scryfall_id()),
        Some("cccccccc-cccc-4ccc-8ccc-cccccccccccc")
    );
    assert_eq!(
        catalog
            .iter()
            .find(|card| card.uuid() == "admonition-uuid")
            .and_then(|card| card.cardmarket_id()),
        Some(42)
    );
    assert_eq!(
        catalog
            .iter()
            .find(|card| card.uuid() == "angel-token")
            .and_then(|card| card.cardmarket_id()),
        Some(44)
    );
    for path in paths {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn token_csv() -> &'static [u8] {
    b"colorIdentity,colors,finishes,keywords,language,manaCost,name,number,power,setCode,side,text,toughness,type,types,uuid\nW,W,nonfoil,Flying,English,,Angel,2,4,TC15,,Flying,4,Token Creature - Angel,Token,angel-token\n"
}

fn gzip(bytes: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes)?;
    encoder.finish()
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn temporary_archive_path() -> Result<PathBuf, Box<dyn Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(std::env::temp_dir().join(format!(
        "dicomtg-cards-{}-{unique}.csv.gz",
        std::process::id()
    )))
}
