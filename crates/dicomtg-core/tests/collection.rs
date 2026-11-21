use std::io::Cursor;

use dicomtg_core::{
    CardCandidate, CardFinish, CollectionBuildError, MatchResult, build_collection,
    import_dragonshield_csv, import_mtgjson_cards_csv, rematch_collection,
};

#[test]
fn col_005_groups_quantities_by_matched_uuid_and_keeps_diagnostics() {
    let csv = concat!(
        "Quantity,Card Name,Set Code,Card Number\n",
        "2,Abrade,DOM,117\n",
        "3,Abrade,DOM,117\n",
        "1,Unknown Card,TST,404\n",
        "0,Invalid Quantity,TST,1\n",
    );
    let catalog = [CardCandidate::new("abrade-uuid", "Abrade", "DOM", "117")];
    let report =
        import_dragonshield_csv(Cursor::new(csv)).expect("CSV should be structurally valid");

    let collection = build_collection(report, &catalog).expect("quantities should not overflow");

    assert_eq!(collection.items().len(), 1);
    assert_eq!(collection.items()[0].uuid(), "abrade-uuid");
    assert_eq!(collection.items()[0].quantity(), 5);
    assert_eq!(collection.unresolved().len(), 1);
    assert_eq!(collection.unresolved()[0].name(), "Unknown Card");
    assert_eq!(
        collection.unresolved()[0].match_result(),
        &MatchResult::Unmatched
    );
    assert_eq!(collection.import_issues().len(), 1);
}

#[test]
fn col_005_rejects_a_quantity_overflow() {
    let csv = concat!(
        "Quantity,Card Name,Set Code,Card Number\n",
        "4294967295,Abrade,DOM,117\n",
        "1,Abrade,DOM,117\n",
    );
    let catalog = [CardCandidate::new("abrade-uuid", "Abrade", "DOM", "117")];
    let report = import_dragonshield_csv(Cursor::new(csv)).expect("CSV should be valid");

    let error = build_collection(report, &catalog).expect_err("quantity should overflow");

    assert_eq!(
        error,
        CollectionBuildError::QuantityOverflow {
            uuid: "abrade-uuid".into()
        }
    );
}

#[test]
fn col_005_keeps_quantities_separate_by_owned_finish() {
    let csv = concat!(
        "Quantity,Card Name,Set Code,Card Number,Printing\n",
        "1,Abrade,DOM,117,Normal\n",
        "2,Abrade,DOM,117,Foil\n",
        "3,Abrade,DOM,117,Halo Foil\n",
        "4,Abrade,DOM,117,Etched Foil\n",
    );
    let catalog = [CardCandidate::new("abrade-uuid", "Abrade", "DOM", "117")];
    let report = import_dragonshield_csv(Cursor::new(csv)).expect("CSV should be valid");

    let collection = build_collection(report, &catalog).expect("quantities should fit");

    assert_eq!(collection.items().len(), 3);
    assert_eq!(collection.items()[0].finish(), CardFinish::Normal);
    assert_eq!(collection.items()[0].quantity(), 1);
    assert_eq!(collection.items()[1].finish(), CardFinish::Foil);
    assert_eq!(collection.items()[1].quantity(), 5);
    assert_eq!(collection.items()[2].finish(), CardFinish::Etched);
    assert_eq!(collection.items()[2].quantity(), 4);
}

#[test]
fn col_005_infers_the_only_catalog_finish_when_dragonshield_leaves_it_blank() {
    let catalog = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid\n",
        ",,foil,,English,,0,Twisted Landscape,521,,common,MH3,,,Land,Land,twisted\n",
    )))
    .expect("catalog fixture should be valid");
    let candidates = catalog
        .cards()
        .iter()
        .map(|card| card.matching_candidate())
        .collect::<Vec<_>>();
    let report = import_dragonshield_csv(Cursor::new(concat!(
        "Quantity,Card Name,Set Code,Card Number,Printing\n",
        "1,Twisted Landscape,MH3,521,\n",
    )))
    .expect("DragonShield fixture should be valid");

    let collection = build_collection(report, &candidates).expect("card should match");

    assert_eq!(collection.items()[0].finish(), CardFinish::Foil);
}

#[test]
fn acc_012_rematches_unresolved_cards_after_a_catalog_refresh() {
    let report = import_dragonshield_csv(Cursor::new(concat!(
        "Quantity,Card Name,Set Code,Card Number\n",
        "2,Abrade,DOM,117\n",
        "1,Angel Token,TC15,2\n",
    )))
    .expect("CSV should be valid");
    let initial_catalog = [CardCandidate::new("abrade", "Abrade", "DOM", "117")];
    let collection =
        build_collection(report, &initial_catalog).expect("initial import should be valid");
    assert_eq!(collection.unresolved().len(), 1);

    let refreshed_catalog = [
        CardCandidate::new("abrade", "Abrade", "DOM", "117"),
        CardCandidate::new("angel-token", "Angel", "TC15", "2"),
    ];
    let collection = rematch_collection(collection, &refreshed_catalog)
        .expect("catalog refresh should rematch unresolved rows");

    assert_eq!(collection.unresolved().len(), 0);
    assert_eq!(collection.items()[0].uuid(), "abrade");
    assert_eq!(collection.items()[0].quantity(), 2);
    assert_eq!(collection.items()[1].uuid(), "angel-token");
    assert_eq!(collection.items()[1].quantity(), 1);
}

#[test]
fn acc_012_corrects_an_impossible_finish_after_a_catalog_refresh() {
    let report = import_dragonshield_csv(Cursor::new(concat!(
        "Quantity,Card Name,Set Code,Card Number,Printing\n",
        "1,Twisted Landscape,MH3,521,\n",
    )))
    .expect("CSV should be valid");
    let initial_catalog = [CardCandidate::new(
        "twisted",
        "Twisted Landscape",
        "MH3",
        "521",
    )];
    let collection = build_collection(report, &initial_catalog).expect("card should match");
    assert_eq!(collection.items()[0].finish(), CardFinish::Normal);
    let refreshed = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid\n",
        ",,foil,,English,,0,Twisted Landscape,521,,common,MH3,,,Land,Land,twisted\n",
    )))
    .expect("catalog fixture should be valid");
    let refreshed_candidates = refreshed
        .cards()
        .iter()
        .map(|card| card.matching_candidate())
        .collect::<Vec<_>>();

    let collection = rematch_collection(collection, &refreshed_candidates)
        .expect("catalog refresh should correct the impossible finish");

    assert_eq!(collection.items()[0].finish(), CardFinish::Foil);
}
