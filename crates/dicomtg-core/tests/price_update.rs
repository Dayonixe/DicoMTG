use std::{
    error::Error,
    fs,
    io::Cursor,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use dicomtg_core::{
    CardFinish, CollectionStore, PriceUpdateError, import_mtgjson_cards_csv,
    import_mtgjson_prices_json, install_cardmarket_data, install_cardmarket_price_guide,
};

#[test]
fn price_004_cardmarket_guide_replaces_the_prices() -> Result<(), Box<dyn Error>> {
    let path = temporary_guide_path()?;
    let guide = guide("1.23", "4.56");
    fs::write(&path, guide.as_bytes())?;
    let mut store = store_with_catalog()?;

    let result = install_cardmarket_price_guide(&mut store, &path)?;

    assert_eq!(result.price_count(), 2);
    assert_eq!(result.downloaded_bytes(), guide.len() as u64);
    let prices = store.load_prices()?;
    assert_eq!(prices[0].finish(), CardFinish::Normal);
    assert_eq!(prices[0].amount_cents(), 123);
    assert_eq!(prices[1].finish(), CardFinish::Foil);
    assert_eq!(prices[1].amount_cents(), 456);
    fs::remove_file(path)?;
    Ok(())
}

#[test]
fn price_004_invalid_guide_keeps_the_previous_prices() -> Result<(), Box<dyn Error>> {
    let path = temporary_guide_path()?;
    fs::write(&path, b"not JSON")?;
    let mut store = store_with_catalog()?;
    let initial = import_mtgjson_prices_json(Cursor::new(
        r#"{"data":{"serra":{"paper":{"cardmarket":{"currency":"EUR","retail":{"normal":{"2026-11-15":1.00}}}}}}}"#,
    ))?;
    store.replace_prices(&initial)?;

    let error = install_cardmarket_price_guide(&mut store, &path)
        .expect_err("an invalid guide must be rejected");

    assert!(matches!(error, PriceUpdateError::Price(_)));
    assert_eq!(store.load_prices()?, initial.prices());
    fs::remove_file(path)?;
    Ok(())
}

#[test]
fn price_004_product_catalog_resolves_tokens_without_an_mtgjson_id() -> Result<(), Box<dyn Error>> {
    let guide_path = temporary_file_path("guide")?;
    let products_path = temporary_file_path("products")?;
    let sets_path = temporary_file_path("sets")?;
    let guide = r#"{
        "createdAt":"2026-11-19T02:48:23+0100",
        "priceGuides":[
            {"idProduct":42,"trend":1.23,"trend-foil":4.56},
            {"idProduct":99,"trend":0.28,"trend-foil":0}
        ]
    }"#;
    let products = r#"{
        "products":[
            {"idProduct":42,"name":"Serra Angel","idExpansion":7},
            {"idProduct":99,"name":"A Mysterious Creature Token (Colorless 2/2)","idExpansion":7}
        ]
    }"#;
    let sets = r#"{"data":[]}"#;
    fs::write(&guide_path, guide)?;
    fs::write(&products_path, products)?;
    fs::write(&sets_path, sets)?;
    let catalog = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,mcmId\n",
        "W,W,nonfoil,,English,,4.0,Serra Angel,1,4,rare,TST,,4,Creature,Creature,serra,42\n",
        ",,nonfoil,,English,,0.0,A Mysterious Creature,21,2,token,TST,,2,Token,Token,mysterious,\n",
    )))?;
    let mut store = CollectionStore::in_memory()?;
    store.replace_catalog(&catalog)?;

    let result = install_cardmarket_data(&mut store, &guide_path, &products_path, &sets_path)?;

    assert_eq!(result.price_count(), 2);
    assert_eq!(
        result.downloaded_bytes(),
        (guide.len() + products.len() + sets.len()) as u64
    );
    assert!(
        store
            .load_prices()?
            .iter()
            .any(|price| price.uuid() == "mysterious" && price.amount_cents() == 28)
    );
    fs::remove_file(guide_path)?;
    fs::remove_file(products_path)?;
    fs::remove_file(sets_path)?;
    Ok(())
}

fn store_with_catalog() -> Result<CollectionStore, Box<dyn Error>> {
    let catalog = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,mcmId\n",
        "W,W,\"foil, nonfoil\",,English,,4.0,Serra Angel,1,4,rare,TST,,4,Creature,Creature,serra,42\n",
    )))?;
    let mut store = CollectionStore::in_memory()?;
    store.replace_catalog(&catalog)?;
    Ok(store)
}

fn guide(normal: &str, foil: &str) -> String {
    format!(
        r#"{{"version":1,"createdAt":"2026-11-19T02:48:23+0100","priceGuides":[{{"idProduct":42,"trend":{normal},"trend-foil":{foil}}}]}}"#
    )
}

fn temporary_guide_path() -> Result<PathBuf, Box<dyn Error>> {
    temporary_file_path("prices")
}

fn temporary_file_path(label: &str) -> Result<PathBuf, Box<dyn Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(std::env::temp_dir().join(format!(
        "dicomtg-{label}-{}-{unique}.json",
        std::process::id(),
    )))
}
