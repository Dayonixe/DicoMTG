use std::io::Cursor;

use dicomtg_core::{
    AnalyticsError, CardCandidate, analyze_collection, analyze_collection_by_type,
    build_collection, import_dragonshield_csv, import_mtgjson_cards_csv,
    import_mtgjson_prices_json,
};

#[test]
fn an_001_an_003_an_005_compute_quantity_value_and_price_coverage() {
    let collection_csv = concat!(
        "Quantity,Card Name,Set Code,Card Number\n",
        "2,Priced Card,TST,1\n",
        "3,Unpriced Card,TST,2\n",
        "4,Unknown Card,TST,3\n",
        "bad,Invalid Card,TST,4\n",
    );
    let report = import_dragonshield_csv(Cursor::new(collection_csv))
        .expect("collection fixture should be valid");
    let catalog = [
        CardCandidate::new("priced", "Priced Card", "TST", "1"),
        CardCandidate::new("unpriced", "Unpriced Card", "TST", "2"),
    ];
    let collection = build_collection(report, &catalog).expect("quantities should fit");
    let prices = import_mtgjson_prices_json(Cursor::new(price_json("priced", "1.25")))
        .expect("price fixture should be valid");

    let analytics = analyze_collection(&collection, &[], prices.prices())
        .expect("collection value should fit in cents");

    assert_eq!(analytics.total_quantity(), 9);
    assert_eq!(analytics.total_value_cents(), 250);
    assert_eq!(analytics.matched_printings(), 2);
    assert_eq!(analytics.priced_printings(), 1);
    assert_eq!(analytics.unpriced_printings(), 1);
    assert_eq!(analytics.price_coverage_basis_points(), 5_000);
}

#[test]
fn an_004_counts_owned_quantities_across_catalog_distributions() {
    let catalog =
        import_mtgjson_cards_csv(Cursor::new(include_bytes!("fixtures/mtgjson_cards.csv")))
            .expect("catalog fixture should be valid");
    let candidates = catalog
        .cards()
        .iter()
        .map(|card| card.matching_candidate())
        .collect::<Vec<_>>();
    let report = import_dragonshield_csv(Cursor::new(concat!(
        "Quantity,Card Name,Set Code,Card Number\n",
        "2,Admonition Angel,WWK,1\n",
        "3,Who What,TST,001a\n",
    )))
    .expect("collection fixture should be valid");
    let collection = build_collection(report, &candidates).expect("quantities should fit");

    let analytics = analyze_collection(&collection, catalog.cards(), &[])
        .expect("collection analytics should fit");

    assert_eq!(
        labels_and_quantities(analytics.colors()),
        [("R", 3), ("U", 3), ("W", 2)]
    );
    assert_eq!(
        labels_and_quantities(analytics.types()),
        [("Creature", 5), ("Legendary", 3)]
    );
    assert_eq!(
        labels_and_quantities(analytics.keywords()),
        [("Flying", 5), ("Haste", 3)]
    );
    assert_eq!(
        labels_and_quantities(analytics.sets()),
        [("TST", 3), ("WWK", 2)]
    );
    assert_eq!(
        labels_and_quantities(analytics.rarities()),
        [("rare", 3), ("mythic", 2)]
    );
    assert_eq!(
        labels_and_quantities(analytics.mana_values()),
        [("3", 3), ("6", 2)]
    );
}

#[test]
fn an_006_filters_metrics_and_distributions_by_card_type() {
    let catalog =
        import_mtgjson_cards_csv(Cursor::new(include_bytes!("fixtures/mtgjson_cards.csv")))
            .expect("catalog fixture should be valid");
    let candidates = catalog
        .cards()
        .iter()
        .map(|card| card.matching_candidate())
        .collect::<Vec<_>>();
    let report = import_dragonshield_csv(Cursor::new(concat!(
        "Quantity,Card Name,Set Code,Card Number\n",
        "2,Admonition Angel,WWK,1\n",
        "3,Who What,TST,001a\n",
        "4,Unknown Card,TST,404\n",
    )))
    .expect("collection fixture should be valid");
    let collection = build_collection(report, &candidates).expect("quantities should fit");

    let analytics = analyze_collection_by_type(&collection, catalog.cards(), &[], "Legendary")
        .expect("filtered analytics should fit");

    assert_eq!(analytics.total_quantity(), 3);
    assert_eq!(analytics.distinct_names(), 1);
    assert_eq!(analytics.matched_printings(), 1);
    assert_eq!(
        labels_and_quantities(analytics.colors()),
        [("R", 3), ("U", 3)]
    );
    assert_eq!(labels_and_quantities(analytics.mana_values()), [("3", 3)]);
}

#[test]
fn an_002_counts_distinct_names_across_matched_and_unresolved_rows() {
    let catalog =
        import_mtgjson_cards_csv(Cursor::new(include_bytes!("fixtures/mtgjson_cards.csv")))
            .expect("catalog fixture should be valid");
    let candidates = catalog
        .cards()
        .iter()
        .map(|card| card.matching_candidate())
        .collect::<Vec<_>>();
    let report = import_dragonshield_csv(Cursor::new(concat!(
        "Quantity,Card Name,Set Code,Card Number\n",
        "1,Admonition Angel,WWK,1\n",
        "2,Who What,TST,001a\n",
        "3,\"Who, What\",TST,404\n",
    )))
    .expect("collection fixture should be valid");
    let collection = build_collection(report, &candidates).expect("quantities should fit");

    let analytics = analyze_collection(&collection, catalog.cards(), &[])
        .expect("collection analytics should fit");

    assert_eq!(analytics.distinct_names(), 2);
    assert_eq!(analytics.matched_printings(), 2);
}

#[test]
fn an_003_rejects_a_collection_value_overflow() {
    let collection_csv = "Quantity,Card Name,Set Code,Card Number\n3,Expensive Card,TST,1\n";
    let report = import_dragonshield_csv(Cursor::new(collection_csv))
        .expect("collection fixture should be valid");
    let catalog = [CardCandidate::new(
        "expensive",
        "Expensive Card",
        "TST",
        "1",
    )];
    let collection = build_collection(report, &catalog).expect("quantity should fit");
    let prices =
        import_mtgjson_prices_json(Cursor::new(price_json("expensive", "92233720368547758.07")))
            .expect("the largest storable price should parse");

    let error = analyze_collection(&collection, &[], prices.prices())
        .expect_err("overflow must not wrap the collection value");

    assert!(matches!(error, AnalyticsError::ValueOverflow { .. }));
}

fn labels_and_quantities(entries: &[dicomtg_core::DistributionEntry]) -> Vec<(&str, u64)> {
    entries
        .iter()
        .map(|entry| (entry.label(), entry.quantity()))
        .collect()
}

fn price_json(uuid: &str, amount: &str) -> String {
    r#"{"data":{"$UUID":{"paper":{"cardmarket":{"currency":"EUR","retail":{"normal":{"2026-11-15":$AMOUNT}}}}}}}"#
        .replace("$UUID", uuid)
        .replace("$AMOUNT", amount)
}
