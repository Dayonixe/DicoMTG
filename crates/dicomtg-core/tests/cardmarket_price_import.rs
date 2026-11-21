use std::io::Cursor;

use dicomtg_core::{
    CardFinish, import_cardmarket_price_guide, import_cardmarket_price_guide_with_products,
    import_cardmarket_price_guide_with_products_and_sets, import_mtgjson_cards_csv,
};

#[test]
fn price_001_maps_cardmarket_trends_to_the_exact_finish() {
    let catalog = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,mcmId\n",
        "W,W,\"foil, nonfoil\",,English,,4.0,Serra Angel,1,4,rare,TST,,4,Creature,Creature,serra,42\n",
    )))
    .expect("catalog fixture should be valid");
    let guide = r#"{
        "version": 1,
        "createdAt": "2026-11-19T02:48:23+0100",
        "priceGuides": [{
            "idProduct": 42,
            "trend": 1.23,
            "trend-foil": 4.56
        }]
    }"#;

    let prices = import_cardmarket_price_guide(Cursor::new(guide), catalog.cards())
        .expect("Cardmarket guide should be valid");

    assert_eq!(prices.prices().len(), 2);
    assert_eq!(prices.prices()[0].uuid(), "serra");
    assert_eq!(prices.prices()[0].finish(), CardFinish::Normal);
    assert_eq!(prices.prices()[0].amount_cents(), 123);
    assert_eq!(prices.prices()[0].as_of_date(), "2026-11-19");
    assert_eq!(prices.prices()[1].finish(), CardFinish::Foil);
    assert_eq!(prices.prices()[1].amount_cents(), 456);
}

#[test]
fn price_001_foil_falls_back_to_the_normal_trend() {
    let catalog = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,mcmId\n",
        ",,\"foil, nonfoil\",,English,,0.0,Powerstone,7,,token,TBRO,,,Token,Token,powerstone,42\n",
    )))
    .expect("catalog fixture should be valid");
    let guide = r#"{
        "createdAt": "2026-11-19T02:48:23+0100",
        "priceGuides": [{"idProduct": 42, "trend": 0.14, "trend-foil": 0}]
    }"#;

    let prices = import_cardmarket_price_guide(Cursor::new(guide), catalog.cards())
        .expect("Cardmarket guide should be valid");

    let foil = prices
        .prices()
        .iter()
        .find(|price| price.finish() == CardFinish::Foil)
        .expect("the foil price should fall back to the normal trend");
    assert_eq!(foil.amount_cents(), 14);
}

#[test]
fn price_001_exposes_the_foil_trend_for_an_etched_catalog_printing() {
    let catalog = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,mcmId\n",
        "W,W,etched,,English,,6.0,\"Isperia, Supreme Judge\",4,6,mythic,SCD,,4,Creature,Creature,isperia,42\n",
    )))
    .expect("catalog fixture should be valid");
    let guide = r#"{
        "createdAt": "2026-11-19T02:48:23+0100",
        "priceGuides": [{"idProduct": 42, "trend": 0, "trend-foil": 0.42}]
    }"#;

    let prices = import_cardmarket_price_guide(Cursor::new(guide), catalog.cards())
        .expect("Cardmarket guide should be valid");

    let foil = prices
        .prices()
        .iter()
        .find(|price| price.finish() == CardFinish::Foil)
        .expect("DragonShield foil imports need a foil price alias");
    assert_eq!(foil.amount_cents(), 42);
}

#[test]
fn price_001_resolves_a_missing_token_id_from_a_unique_cardmarket_product() {
    let catalog = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,mcmId\n",
        ",,nonfoil,,English,,1.0,Anchor Card,1,,common,TST,,,Sorcery,Sorcery,anchor,10\n",
        ",,nonfoil,,English,,0.0,A Mysterious Creature,21,2,token,TST,,2,Token,Token,mysterious,\n",
    )))
    .expect("catalog fixture should be valid");
    let products = r#"{
        "products": [
            {"idProduct": 10, "name": "Anchor Card", "idExpansion": 7},
            {"idProduct": 20, "name": "A Mysterious Creature Token (Colorless 2/2)", "idExpansion": 7}
        ]
    }"#;
    let guide = r#"{
        "createdAt": "2026-11-19T02:48:23+0100",
        "priceGuides": [
            {"idProduct": 10, "trend": 1.00, "trend-foil": 0},
            {"idProduct": 20, "trend": 0.28, "trend-foil": 0}
        ]
    }"#;

    let prices = import_cardmarket_price_guide_with_products(
        Cursor::new(guide),
        Cursor::new(products),
        catalog.cards(),
    )
    .expect("Cardmarket files should be valid");

    let token = prices
        .prices()
        .iter()
        .find(|price| price.uuid() == "mysterious")
        .expect("the unique token product should be resolved");
    assert_eq!(token.amount_cents(), 28);
}

#[test]
fn price_001_uses_the_median_trend_for_ambiguous_token_products() {
    let catalog = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,mcmId\n",
        ",,nonfoil,,English,,1.0,Anchor Card,1,,common,TST,,,Sorcery,Sorcery,anchor,10\n",
        ",,nonfoil,,English,,0.0,Angel,2,4,token,TST,,4,Token,Token,angel,\n",
    )))
    .expect("catalog fixture should be valid");
    let products = r#"{
        "products": [
            {"idProduct": 10, "name": "Anchor Card", "idExpansion": 7},
            {"idProduct": 20, "name": "Angel Token (White 4/4)", "idExpansion": 7},
            {"idProduct": 21, "name": "Angel Token (White 4/4)", "idExpansion": 7}
        ]
    }"#;
    let guide = r#"{
        "createdAt": "2026-11-19T02:48:23+0100",
        "priceGuides": [
            {"idProduct": 10, "trend": 1.00, "trend-foil": 0},
            {"idProduct": 20, "trend": 0.20, "trend-foil": 0},
            {"idProduct": 21, "trend": 0.22, "trend-foil": 0}
        ]
    }"#;

    let prices = import_cardmarket_price_guide_with_products(
        Cursor::new(guide),
        Cursor::new(products),
        catalog.cards(),
    )
    .expect("Cardmarket files should be valid");

    let angel = prices
        .prices()
        .iter()
        .find(|price| price.uuid() == "angel")
        .expect("ambiguous physical products should use their median TREND");
    assert_eq!(angel.amount_cents(), 21);
}

#[test]
fn price_001_matches_a_complete_double_faced_product_name() {
    let catalog = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,mcmId\n",
        ",,nonfoil,,English,,0.0,Day // Night,19,,token,TMID,,,Card,Token,day-night,\n",
        ",,nonfoil,,English,,0.0,Clue,20,,token,TMID,,,Token,Token,clue,\n",
    )))
    .expect("catalog fixture should be valid");
    let products = r#"{
        "products": [
            {"idProduct": 20, "name": "Day // Night", "idExpansion": 7},
            {"idProduct": 21, "name": "Clue Token", "idExpansion": 7}
        ]
    }"#;
    let guide = r#"{
        "createdAt": "2026-11-19T02:48:23+0100",
        "priceGuides": [
            {"idProduct": 20, "trend": 0.08, "trend-foil": 0},
            {"idProduct": 21, "trend": 0.10, "trend-foil": 0}
        ]
    }"#;

    let prices = import_cardmarket_price_guide_with_products(
        Cursor::new(guide),
        Cursor::new(products),
        catalog.cards(),
    )
    .expect("Cardmarket files should be valid");

    assert!(
        prices
            .prices()
            .iter()
            .any(|price| price.uuid() == "day-night" && price.amount_cents() == 8)
    );
}

#[test]
fn price_001_rejects_the_only_named_product_when_power_and_toughness_differ() {
    let catalog = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,mcmId\n",
        ",,nonfoil,,English,,0.0,Elephant,20,3,token,TSCD,,3,Token,Token,elephant,\n",
        ",,nonfoil,,English,,0.0,Clue,21,,token,TSCD,,,Token,Token,clue,\n",
    )))
    .expect("catalog fixture should be valid");
    let products = r#"{
        "products": [
            {"idProduct": 20, "name": "Elephant Token (G 1/1) // Thopter Token (A 1/1)", "idExpansion": 7},
            {"idProduct": 21, "name": "Clue Token", "idExpansion": 7}
        ]
    }"#;
    let guide = r#"{
        "createdAt": "2026-11-19T02:48:23+0100",
        "priceGuides": [
            {"idProduct": 20, "trend": 0.25, "trend-foil": 0},
            {"idProduct": 21, "trend": 0.10, "trend-foil": 0}
        ]
    }"#;

    let prices = import_cardmarket_price_guide_with_products(
        Cursor::new(guide),
        Cursor::new(products),
        catalog.cards(),
    )
    .expect("Cardmarket files should be valid");

    assert!(
        prices
            .prices()
            .iter()
            .all(|price| price.uuid() != "elephant")
    );
}

#[test]
fn price_001_matches_a_named_token_when_cardmarket_omits_its_stats() {
    let catalog = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,mcmId\n",
        ",,nonfoil,,English,,0.0,Named Warrior,1,2,token,TTST,,1,Token Creature — Zombie Warrior,Token,named-warrior,\n",
        ",,nonfoil,,English,,0.0,Clue,2,,token,TTST,,,Token,Token,clue,\n",
    )))
    .expect("catalog fixture should be valid");
    let products = r#"{
        "products": [
            {"idProduct": 20, "name": "Named Warrior Token", "idExpansion": 7},
            {"idProduct": 21, "name": "Clue Token", "idExpansion": 7}
        ]
    }"#;
    let guide = r#"{
        "createdAt": "2026-11-19T02:48:23+0100",
        "priceGuides": [
            {"idProduct": 20, "trend": 0.16, "trend-foil": 0},
            {"idProduct": 21, "trend": 0.10, "trend-foil": 0}
        ]
    }"#;

    let prices = import_cardmarket_price_guide_with_products(
        Cursor::new(guide),
        Cursor::new(products),
        catalog.cards(),
    )
    .expect("Cardmarket files should be valid");

    assert!(
        prices
            .prices()
            .iter()
            .any(|price| price.uuid() == "named-warrior" && price.amount_cents() == 16)
    );
}

#[test]
fn price_001_keeps_a_generic_token_without_characteristics_unpriced() {
    let catalog = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,mcmId\n",
        ",,nonfoil,,English,,0.0,Elephant,1,3,token,TTST,,3,Token Creature — Elephant,Token,elephant,\n",
        ",,nonfoil,,English,,0.0,Clue,2,,token,TTST,,,Token,Token,clue,\n",
    )))
    .expect("catalog fixture should be valid");
    let products = r#"{
        "products": [
            {"idProduct": 20, "name": "Elephant Token", "idExpansion": 7},
            {"idProduct": 21, "name": "Clue Token", "idExpansion": 7}
        ]
    }"#;
    let guide = r#"{
        "createdAt": "2026-11-19T02:48:23+0100",
        "priceGuides": [
            {"idProduct": 20, "trend": 0.25, "trend-foil": 0},
            {"idProduct": 21, "trend": 0.10, "trend-foil": 0}
        ]
    }"#;

    let prices = import_cardmarket_price_guide_with_products(
        Cursor::new(guide),
        Cursor::new(products),
        catalog.cards(),
    )
    .expect("Cardmarket files should be valid");

    assert!(
        prices
            .prices()
            .iter()
            .all(|price| price.uuid() != "elephant")
    );
}

#[test]
fn price_001_matches_supplementary_objects_on_a_physical_card_face() {
    let catalog = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,mcmId\n",
        ",,nonfoil,,English,,0.0,Chandra Emblem,1,,token,TTST,,,Emblem,Emblem,chandra,\n",
        ",,nonfoil,,English,,0.0,Enduring Story,2,,token,TTST,,,Card,Card,story,\n",
    )))
    .expect("catalog fixture should be valid");
    let products = r#"{
        "products": [
            {"idProduct": 20, "name": "Vehicle Token (A 3/2) // Chandra Emblem", "idExpansion": 7},
            {"idProduct": 21, "name": "Dwarf Token (R 2/2) // Enduring Story", "idExpansion": 7}
        ]
    }"#;
    let guide = r#"{
        "createdAt": "2026-11-19T02:48:23+0100",
        "priceGuides": [
            {"idProduct": 20, "trend": 0.11, "trend-foil": 0},
            {"idProduct": 21, "trend": 0.13, "trend-foil": 0}
        ]
    }"#;

    let prices = import_cardmarket_price_guide_with_products(
        Cursor::new(guide),
        Cursor::new(products),
        catalog.cards(),
    )
    .expect("Cardmarket files should be valid");

    assert!(
        prices
            .prices()
            .iter()
            .any(|price| price.uuid() == "chandra" && price.amount_cents() == 11)
    );
    assert!(
        prices
            .prices()
            .iter()
            .any(|price| price.uuid() == "story" && price.amount_cents() == 13)
    );
}

#[test]
fn price_001_uses_token_characteristics_to_select_the_product() {
    let catalog = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,mcmId\n",
        ",W,nonfoil,,English,,1.0,Anchor Card,1,,common,TST,,,Sorcery,Sorcery,anchor,10\n",
        "W,W,nonfoil,,English,,0.0,Angel,2,4,token,TST,,4,Token Creature — Angel,Token,angel,\n",
    )))
    .expect("catalog fixture should be valid");
    let products = r#"{
        "products": [
            {"idProduct": 10, "name": "Anchor Card", "idExpansion": 7},
            {"idProduct": 20, "name": "Angel Token (White 3/3)", "idExpansion": 7},
            {"idProduct": 21, "name": "Angel Token (White 4/4)", "idExpansion": 7}
        ]
    }"#;
    let guide = r#"{
        "createdAt": "2026-11-19T02:48:23+0100",
        "priceGuides": [
            {"idProduct": 10, "trend": 1.00, "trend-foil": 0},
            {"idProduct": 20, "trend": 0.20, "trend-foil": 0},
            {"idProduct": 21, "trend": 0.42, "trend-foil": 0}
        ]
    }"#;

    let prices = import_cardmarket_price_guide_with_products(
        Cursor::new(guide),
        Cursor::new(products),
        catalog.cards(),
    )
    .expect("Cardmarket files should be valid");

    let token = prices
        .prices()
        .iter()
        .find(|price| price.uuid() == "angel")
        .expect("power and toughness should identify the token product");
    assert_eq!(token.amount_cents(), 42);
}

#[test]
fn price_001_infers_a_token_expansion_from_multiple_matching_names() {
    let catalog = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,mcmId\n",
        ",,nonfoil,,English,,0.0,A Mysterious Creature,21,2,token,TST,,2,Token,Token,mysterious,\n",
        ",,nonfoil,,English,,0.0,Clue,22,,token,TST,,,Token,Token,clue,\n",
    )))
    .expect("catalog fixture should be valid");
    let products = r#"{
        "products": [
            {"idProduct": 20, "name": "A Mysterious Creature Token (Colorless 2/2)", "idExpansion": 7},
            {"idProduct": 21, "name": "Clue Token", "idExpansion": 7},
            {"idProduct": 30, "name": "Clue Token", "idExpansion": 8}
        ]
    }"#;
    let guide = r#"{
        "createdAt": "2026-11-19T02:48:23+0100",
        "priceGuides": [
            {"idProduct": 20, "trend": 0.28, "trend-foil": 0},
            {"idProduct": 21, "trend": 0.10, "trend-foil": 0},
            {"idProduct": 30, "trend": 0.30, "trend-foil": 0}
        ]
    }"#;

    let prices = import_cardmarket_price_guide_with_products(
        Cursor::new(guide),
        Cursor::new(products),
        catalog.cards(),
    )
    .expect("the uniquely best expansion should be inferred");

    assert_eq!(prices.prices().len(), 2);
    assert!(
        prices
            .prices()
            .iter()
            .any(|price| price.uuid() == "mysterious" && price.amount_cents() == 28)
    );
}

#[test]
fn price_001_does_not_infer_an_expansion_when_the_best_scores_are_tied() {
    let catalog = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,mcmId\n",
        ",,nonfoil,,English,,1.0,Anchor Card,1,,common,KNOWN,,,Sorcery,Sorcery,anchor,10\n",
        ",,nonfoil,,English,,0.0,Angel,2,4,token,TST,,4,Token,Token,angel,\n",
        ",,nonfoil,,English,,0.0,Clue,3,,token,TST,,,Token,Token,clue,\n",
    )))
    .expect("catalog fixture should be valid");
    let products = r#"{
        "products": [
            {"idProduct": 10, "name": "Anchor Card", "idExpansion": 1},
            {"idProduct": 20, "name": "Angel Token (White 4/4)", "idExpansion": 7},
            {"idProduct": 21, "name": "Clue Token", "idExpansion": 7},
            {"idProduct": 30, "name": "Angel Token (White 4/4)", "idExpansion": 8},
            {"idProduct": 31, "name": "Clue Token", "idExpansion": 8}
        ]
    }"#;
    let guide = r#"{
        "createdAt": "2026-11-19T02:48:23+0100",
        "priceGuides": [
            {"idProduct": 10, "trend": 1.00, "trend-foil": 0},
            {"idProduct": 20, "trend": 0.20, "trend-foil": 0},
            {"idProduct": 21, "trend": 0.21, "trend-foil": 0},
            {"idProduct": 30, "trend": 0.30, "trend-foil": 0},
            {"idProduct": 31, "trend": 0.31, "trend-foil": 0}
        ]
    }"#;

    let prices = import_cardmarket_price_guide_with_products(
        Cursor::new(guide),
        Cursor::new(products),
        catalog.cards(),
    )
    .expect("the known card should still be priced");

    assert_eq!(prices.prices().len(), 1);
    assert_eq!(prices.prices()[0].uuid(), "anchor");
}

#[test]
fn price_001_uses_mtgjson_set_ids_to_resolve_a_tied_token_expansion() {
    let catalog = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,mcmId\n",
        ",,nonfoil,,English,,0.0,Angel,2,4,token,TTST,,4,Token,Token,angel,\n",
        ",,nonfoil,,English,,0.0,Clue,3,,token,TTST,,,Token,Token,clue,\n",
    )))
    .expect("catalog fixture should be valid");
    let products = r#"{
        "products": [
            {"idProduct": 20, "name": "Angel Token (White 4/4)", "idExpansion": 7},
            {"idProduct": 21, "name": "Clue Token", "idExpansion": 7},
            {"idProduct": 30, "name": "Angel Token (White 4/4)", "idExpansion": 8},
            {"idProduct": 31, "name": "Clue Token", "idExpansion": 8}
        ]
    }"#;
    let sets = r#"{"data":[{
        "code":"TST",
        "tokenSetCode":"TTST",
        "mcmId":7,
        "releaseDate":"2026-11-01"
    }]}"#;
    let guide = r#"{
        "createdAt": "2026-11-19T02:48:23+0100",
        "priceGuides": [
            {"idProduct": 20, "trend": 0.20, "trend-foil": 0},
            {"idProduct": 21, "trend": 0.21, "trend-foil": 0},
            {"idProduct": 30, "trend": 0.30, "trend-foil": 0},
            {"idProduct": 31, "trend": 0.31, "trend-foil": 0}
        ]
    }"#;

    let prices = import_cardmarket_price_guide_with_products_and_sets(
        Cursor::new(guide),
        Cursor::new(products),
        Cursor::new(sets),
        catalog.cards(),
    )
    .expect("the MTGJSON Cardmarket expansion should break the tie");

    assert!(
        prices
            .prices()
            .iter()
            .any(|price| price.uuid() == "angel" && price.amount_cents() == 20)
    );
}

#[test]
fn price_001_uses_the_release_date_to_resolve_a_tied_token_expansion() {
    let catalog = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,mcmId\n",
        ",,nonfoil,,English,,0.0,Angel,2,4,token,TTST,,4,Token,Token,angel,\n",
        ",,nonfoil,,English,,0.0,Clue,3,,token,TTST,,,Token,Token,clue,\n",
    )))
    .expect("catalog fixture should be valid");
    let products = r#"{
        "products": [
            {"idProduct": 20, "name": "Angel Token (White 4/4)", "idExpansion": 7, "dateAdded":"2020-01-01 00:00:00"},
            {"idProduct": 21, "name": "Clue Token", "idExpansion": 7, "dateAdded":"2020-01-01 00:00:00"},
            {"idProduct": 30, "name": "Angel Token (White 4/4)", "idExpansion": 8, "dateAdded":"2026-11-02 00:00:00"},
            {"idProduct": 31, "name": "Clue Token", "idExpansion": 8, "dateAdded":"2026-11-02 00:00:00"}
        ]
    }"#;
    let sets = r#"{"data":[{
        "code":"TST",
        "tokenSetCode":"TTST",
        "releaseDate":"2026-11-01"
    }]}"#;
    let guide = r#"{
        "createdAt": "2026-11-19T02:48:23+0100",
        "priceGuides": [
            {"idProduct": 20, "trend": 0.20, "trend-foil": 0},
            {"idProduct": 21, "trend": 0.21, "trend-foil": 0},
            {"idProduct": 30, "trend": 0.30, "trend-foil": 0},
            {"idProduct": 31, "trend": 0.31, "trend-foil": 0}
        ]
    }"#;

    let prices = import_cardmarket_price_guide_with_products_and_sets(
        Cursor::new(guide),
        Cursor::new(products),
        Cursor::new(sets),
        catalog.cards(),
    )
    .expect("the closest product date should break the tie");

    assert!(
        prices
            .prices()
            .iter()
            .any(|price| price.uuid() == "angel" && price.amount_cents() == 30)
    );
}
