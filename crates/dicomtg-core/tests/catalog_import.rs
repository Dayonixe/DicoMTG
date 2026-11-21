use std::io::Cursor;

use dicomtg_core::{
    CatalogImportError, CatalogRowError, import_mtgjson_cards_csv, import_mtgjson_tokens_csv,
};

#[test]
fn cat_003_imports_the_required_mtgjson_fields_without_a_fixed_column_count() {
    let csv = include_bytes!("fixtures/mtgjson_cards.csv");

    let catalog = import_mtgjson_cards_csv(Cursor::new(csv)).expect("fixture should be valid");

    assert_eq!(catalog.cards().len(), 2);
    let card = &catalog.cards()[1];
    assert_eq!(card.uuid(), "split-uuid");
    assert_eq!(
        card.scryfall_id(),
        Some("11111111-1111-4111-8111-111111111111")
    );
    assert_eq!(card.name(), "Who, What");
    assert_eq!(card.key().set_code(), "TST");
    assert_eq!(card.key().collector_number(), "001a");
    assert_eq!(card.mana_value(), 3.0);
    assert_eq!(card.colors(), ["R", "U"]);
    assert_eq!(card.color_identity(), ["R", "U"]);
    assert_eq!(card.types(), ["Legendary", "Creature"]);
    assert_eq!(card.keywords(), ["Flying", "Haste"]);
    assert_eq!(card.finishes(), ["nonfoil"]);
    assert_eq!(card.language(), "English");
    assert_eq!(card.power(), Some("3"));
    assert_eq!(card.toughness(), Some("2"));
    assert_eq!(card.rules_text(), Some("Rules text, with comma."));
    assert!(!card.has_back_face());
}

#[test]
fn cat_004_ignores_secondary_faces_of_the_same_physical_card() {
    let header = "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid,side,layout\n";
    let front = ",,nonfoil,,English,,2.0,Alluring Suitor // Deadly Dancer,141,2,uncommon,VOW,,3,Creature,Creature,front-uuid,a,transform\n";
    let back = ",,nonfoil,,English,,2.0,Alluring Suitor // Deadly Dancer,141,2,uncommon,VOW,,3,Creature,Creature,back-uuid,b,transform\n";

    let catalog = import_mtgjson_cards_csv(Cursor::new(format!("{header}{front}{back}")))
        .expect("two faces should describe one physical card");

    assert_eq!(catalog.cards().len(), 1);
    assert_eq!(catalog.cards()[0].uuid(), "front-uuid");
    assert!(catalog.cards()[0].has_back_face());
}

#[test]
fn cat_005_imports_tokens_without_card_only_fields() {
    let csv = "colorIdentity,colors,finishes,keywords,language,manaCost,name,number,power,setCode,side,text,toughness,type,types,uuid\nW,W,nonfoil,Flying,English,,Angel,2,4,TC15,,Flying,4,Token Creature — Angel,Token,angel-token\n";

    let catalog =
        import_mtgjson_tokens_csv(Cursor::new(csv)).expect("MTGJSON token should be valid");

    assert_eq!(catalog.cards().len(), 1);
    let token = &catalog.cards()[0];
    assert_eq!(token.uuid(), "angel-token");
    assert_eq!(token.name(), "Angel");
    assert_eq!(token.key().set_code(), "TC15");
    assert_eq!(token.key().collector_number(), "2");
    assert_eq!(token.rarity(), "token");
    assert_eq!(token.mana_value(), 0.0);
}

#[test]
fn cat_003_rejects_missing_headers() {
    let csv = "uuid,name,setCode,number\ncard,Abrade,DOM,117\n";

    let error = import_mtgjson_cards_csv(Cursor::new(csv)).expect_err("headers are incomplete");

    assert!(matches!(
        error,
        CatalogImportError::MissingHeaders { names } if names.contains(&"manaValue".to_owned())
    ));
}

#[test]
fn cat_003_rejects_an_invalid_row_instead_of_publishing_a_partial_catalog() {
    let csv = concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid\n",
        ",,nonfoil,,English,,not-a-number,Abrade,117,3,uncommon,DOM,,2,Instant,Instant,abrade\n",
    );

    let error = import_mtgjson_cards_csv(Cursor::new(csv)).expect_err("mana value is invalid");

    assert!(matches!(
        error,
        CatalogImportError::InvalidRow {
            line: 2,
            reason: CatalogRowError::InvalidManaValue { value }
        } if value == "not-a-number"
    ));
}

#[test]
fn cat_003_rejects_duplicate_uuids() {
    let header = "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid\n";
    let row = ",,nonfoil,,English,,1.0,Abrade,117,3,uncommon,DOM,,2,Instant,Instant,duplicate\n";
    let csv = format!("{header}{row}{row}");

    let error = import_mtgjson_cards_csv(Cursor::new(csv)).expect_err("UUID must be unique");

    assert!(matches!(
        error,
        CatalogImportError::InvalidRow {
            line: 3,
            reason: CatalogRowError::DuplicateUuid { uuid }
        } if uuid == "duplicate"
    ));
}

#[test]
fn cat_002_rejects_an_empty_catalog() {
    let csv = "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid\n";

    let error = import_mtgjson_cards_csv(Cursor::new(csv)).expect_err("catalog must not be empty");

    assert!(matches!(error, CatalogImportError::EmptyCatalog));
}
