use std::io::Cursor;

use dicomtg_core::{
    CardCandidate, CollectionItemStatus, CollectionQuery, CollectionQueryError, build_collection,
    import_dragonshield_csv, import_mtgjson_cards_csv, import_mtgjson_prices_json,
    most_expensive_card, query_collection,
};

const CATALOG_HEADER: &str = "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid\n";

#[test]
fn ui_001_search_is_partial_case_insensitive_and_empty_search_does_not_filter() {
    let catalog = catalog(concat!(
        "W,W,nonfoil,,English,,4.0,Serra Angel,1,4,rare,TST,,4,Creature — Angel,Creature,serra\n",
        "R,R,nonfoil,,English,,1.0,Lightning Bolt,2,,uncommon,TST,,,Instant,Instant,bolt\n",
    ));
    let collection = collection(
        "Quantity,Card Name,Set Code,Card Number\n1,Serra Angel,TST,1\n1,Lightning Bolt,TST,2\n",
        &catalog,
    );

    let searched = query_collection(
        &collection,
        catalog.cards(),
        &[],
        &CollectionQuery::new("ANG", vec![], vec![]),
    )
    .expect("query should be valid");
    let all = query_collection(
        &collection,
        catalog.cards(),
        &[],
        &CollectionQuery::default(),
    )
    .expect("empty query should be valid");

    assert_eq!(searched.len(), 1);
    assert_eq!(searched[0].name(), "Serra Angel");
    assert_eq!(all.len(), 2);
}

#[test]
fn acc_006_colors_are_or_and_rarity_is_and() {
    let catalog = catalog(concat!(
        "W,W,nonfoil,,English,,1.0,White Rare,1,,rare,TST,,,Creature,Creature,white-rare\n",
        "U,U,nonfoil,,English,,1.0,Blue Rare,2,,rare,TST,,,Creature,Creature,blue-rare\n",
        "W,W,nonfoil,,English,,1.0,White Common,3,,common,TST,,,Creature,Creature,white-common\n",
        "R,R,nonfoil,,English,,1.0,Red Rare,4,,rare,TST,,,Creature,Creature,red-rare\n",
    ));
    let collection = collection(
        concat!(
            "Quantity,Card Name,Set Code,Card Number\n",
            "1,White Rare,TST,1\n",
            "1,Blue Rare,TST,2\n",
            "1,White Common,TST,3\n",
            "1,Red Rare,TST,4\n",
            "1,Unknown Card,TST,404\n",
        ),
        &catalog,
    );
    let query = CollectionQuery::new(
        "",
        vec!["W".to_owned(), "U".to_owned()],
        vec!["rare".to_owned()],
    );

    let result =
        query_collection(&collection, catalog.cards(), &[], &query).expect("query should be valid");

    assert_eq!(
        result.iter().map(|item| item.uuid()).collect::<Vec<_>>(),
        [Some("white-rare"), Some("blue-rare")]
    );
}

#[test]
fn ui_006_type_filter_keeps_cards_with_the_selected_type() {
    let catalog = catalog(concat!(
        "W,W,nonfoil,,English,,4.0,Serra Angel,1,4,rare,TST,,4,Creature — Angel,\"Creature, Angel\",serra\n",
        "R,R,nonfoil,,English,,1.0,Lightning Bolt,2,,uncommon,TST,,,Instant,Instant,bolt\n",
    ));
    let collection = collection(
        "Quantity,Card Name,Set Code,Card Number\n1,Serra Angel,TST,1\n1,Lightning Bolt,TST,2\n",
        &catalog,
    );
    let query = CollectionQuery::default().with_types(vec!["Angel".to_owned()]);

    let result =
        query_collection(&collection, catalog.cards(), &[], &query).expect("query should be valid");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name(), "Serra Angel");
}

#[test]
fn ui_003_list_exposes_matched_unmatched_and_ambiguous_rows() {
    let catalog = catalog(concat!(
        "W,W,nonfoil,,English,,4.0,Serra Angel,1,4,rare,TST,,4,Creature — Angel,Creature,serra\n",
        "U,U,nonfoil,,English,,2.0,Variant One,9,,rare,TST,,,Instant,Instant,variant-one\n",
        "U,U,nonfoil,,English,,2.0,Variant Two,9,,rare,TST,,,Instant,Instant,variant-two\n",
    ));
    let collection = collection(
        concat!(
            "Quantity,Card Name,Set Code,Card Number\n",
            "2,Serra Angel,TST,1\n",
            "3,Missing Card,TST,404\n",
            "1,Unknown Variant,TST,9\n",
        ),
        &catalog,
    );
    let prices = import_mtgjson_prices_json(Cursor::new(price_json("serra", "1.50")))
        .expect("price fixture should be valid");

    let result = query_collection(
        &collection,
        catalog.cards(),
        prices.prices(),
        &CollectionQuery::default(),
    )
    .expect("query should be valid");

    assert_eq!(result.len(), 3);
    assert_eq!(result[0].status(), CollectionItemStatus::Matched);
    assert_eq!(result[0].quantity(), 2);
    assert_eq!(result[0].rarity(), Some("rare"));
    assert_eq!(result[0].mana_value(), Some(4.0));
    assert_eq!(result[0].price_cents(), Some(150));
    assert_eq!(result[0].price_date(), Some("2026-11-15"));
    assert_eq!(result[1].status(), CollectionItemStatus::Unmatched);
    assert_eq!(result[1].price_cents(), None);
    assert_eq!(result[2].status(), CollectionItemStatus::Ambiguous);
}

#[test]
fn price_006_uses_the_price_for_the_owned_finish() {
    let catalog = catalog(
        "W,W,\"foil, nonfoil\",,English,,4.0,Serra Angel,1,4,rare,TST,,4,Creature,Creature,serra\n",
    );
    let collection = collection(
        "Quantity,Card Name,Set Code,Card Number,Printing\n1,Serra Angel,TST,1,Foil\n",
        &catalog,
    );
    let prices = import_mtgjson_prices_json(Cursor::new(
        r#"{"data":{"serra":{"paper":{"cardmarket":{"currency":"EUR","retail":{"normal":{"2026-11-15":1.00},"foil":{"2026-11-15":2.50}}}}}}}"#,
    ))
    .expect("price fixture should be valid");

    let result = query_collection(
        &collection,
        catalog.cards(),
        prices.prices(),
        &CollectionQuery::default(),
    )
    .expect("query should be valid");

    assert_eq!(result[0].finish(), dicomtg_core::CardFinish::Foil);
    assert_eq!(result[0].price_cents(), Some(250));
    assert_eq!(result[0].normal_price_cents(), Some(100));
    assert_eq!(result[0].foil_price_cents(), Some(250));
}

#[test]
fn ui_004_detail_exposes_catalog_ownership_and_price_data() {
    let catalog = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,scryfallId,layout\n",
        "W,W,\"foil, nonfoil\",\"Flying, Vigilance\",English,{3}{W},4.0,",
        "Serra Angel,1,4,rare,TST,\"Flying, vigilance.\",4,",
        "Creature — Angel,\"Creature, Angel\",serra,5feddbd0-3d29-4428-b70d-107c1e26930d,transform\n",
    )))
    .expect("catalog fixture should be valid");
    let collection = collection(
        "Quantity,Card Name,Set Code,Card Number\n2,Serra Angel,TST,1\n",
        &catalog,
    );
    let prices = import_mtgjson_prices_json(Cursor::new(price_json("serra", "1.50")))
        .expect("price fixture should be valid");

    let result = query_collection(
        &collection,
        catalog.cards(),
        prices.prices(),
        &CollectionQuery::default(),
    )
    .expect("query should be valid");
    let detail = &result[0];

    assert_eq!(detail.quantity(), 2);
    assert_eq!(
        detail.scryfall_id(),
        Some("5feddbd0-3d29-4428-b70d-107c1e26930d")
    );
    assert_eq!(detail.color_identity(), ["W"]);
    assert_eq!(detail.types(), ["Creature", "Angel"]);
    assert_eq!(detail.keywords(), ["Flying", "Vigilance"]);
    assert_eq!(detail.finishes(), ["foil", "nonfoil"]);
    assert_eq!(detail.language(), Some("English"));
    assert_eq!(detail.mana_cost(), Some("{3}{W}"));
    assert_eq!(detail.power(), Some("4"));
    assert_eq!(detail.toughness(), Some("4"));
    assert_eq!(detail.type_line(), Some("Creature — Angel"));
    assert_eq!(detail.rules_text(), Some("Flying, vigilance."));
    assert!(detail.has_back_face());
    assert_eq!(detail.price_cents(), Some(150));
    assert_eq!(detail.price_date(), Some("2026-11-15"));
}

#[test]
fn ui_008_most_expensive_card_uses_the_unit_price() {
    let catalog = catalog(concat!(
        "W,W,nonfoil,,English,,4.0,Premium Card,1,,rare,TST,,,Creature,Creature,premium\n",
        "U,U,nonfoil,,English,,2.0,Large Stack,2,,rare,TST,,,Instant,Instant,stack\n",
    ));
    let collection = collection(
        concat!(
            "Quantity,Card Name,Set Code,Card Number\n",
            "1,Premium Card,TST,1\n",
            "10,Large Stack,TST,2\n",
        ),
        &catalog,
    );
    let prices = import_mtgjson_prices_json(Cursor::new(
        r#"{"data":{"premium":{"paper":{"cardmarket":{"currency":"EUR","retail":{"normal":{"2026-11-15":5.00}}}}},"stack":{"paper":{"cardmarket":{"currency":"EUR","retail":{"normal":{"2026-11-15":1.00}}}}}}}"#,
    ))
    .expect("price fixture should be valid");
    let items = query_collection(
        &collection,
        catalog.cards(),
        prices.prices(),
        &CollectionQuery::default(),
    )
    .expect("query should be valid");

    let featured = most_expensive_card(&items).expect("a priced card should be selected");

    assert_eq!(featured.name(), "Premium Card");
    assert_eq!(featured.price_cents(), Some(500));
}

#[test]
fn ui_003_missing_catalog_data_is_reported_instead_of_hiding_an_owned_card() {
    let report = import_dragonshield_csv(Cursor::new(
        "Quantity,Card Name,Set Code,Card Number\n1,Orphan,TST,1\n",
    ))
    .expect("collection fixture should be valid");
    let collection = build_collection(
        report,
        &[CardCandidate::new("orphan", "Orphan", "TST", "1")],
    )
    .expect("quantity should fit");

    let error = query_collection(&collection, &[], &[], &CollectionQuery::default())
        .expect_err("missing catalog data must be explicit");

    assert!(matches!(
        error,
        CollectionQueryError::MissingCatalogCard { uuid } if uuid == "orphan"
    ));
}

fn catalog(rows: &str) -> dicomtg_core::CatalogImport {
    import_mtgjson_cards_csv(Cursor::new(format!("{CATALOG_HEADER}{rows}")))
        .expect("catalog fixture should be valid")
}

fn collection(
    csv: &str,
    catalog: &dicomtg_core::CatalogImport,
) -> dicomtg_core::CollectionSnapshot {
    let report = import_dragonshield_csv(Cursor::new(csv)).expect("collection fixture is valid");
    let candidates = catalog
        .cards()
        .iter()
        .map(|card| card.matching_candidate())
        .collect::<Vec<_>>();
    build_collection(report, &candidates).expect("quantities should fit")
}

fn price_json(uuid: &str, amount: &str) -> String {
    r#"{"data":{"$UUID":{"paper":{"cardmarket":{"currency":"EUR","retail":{"normal":{"2026-11-15":$AMOUNT}}}}}}}"#
        .replace("$UUID", uuid)
        .replace("$AMOUNT", amount)
}
