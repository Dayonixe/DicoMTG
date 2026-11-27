use std::io::Cursor;

use dicomtg_core::{
    CardCandidate, CardFinish, ImportError, RowIssueReason, build_collection,
    import_collection_csv, import_dragonshield_csv,
};

#[test]
fn imports_sorted_magic_folders_with_language_and_finish() -> Result<(), Box<dyn std::error::Error>>
{
    let csv = concat!(
        "\"sep=,\"\r\n",
        "List Type,List Name,Collection,Board,Quantity,Card Name,Set Code,Card Number,Printing,Language\r\n",
        "Folder,Owned,mtg,,1,Counterspell,STA,15,Normal,fr\r\n",
        "Folder,Owned,mtg,,2,Counterspell,STA,15,Foil,fr\r\n",
        "Folder,Owned,mtg,,3,Counterspell,STA,15,Normal,en\r\n",
        "Deck,Sample,mtg,Main Deck,4,Counterspell,STA,15,Normal,\r\n",
        "Folder,Other,pok,,5,Squirtle,TST,15,Reverse Holo,fr\r\n",
    );
    let report = import_collection_csv(csv.as_bytes())?;
    assert_eq!(report.cards().len(), 3);
    assert_eq!(report.ignored_rows(), 2);
    assert_eq!(report.cards()[0].language(), Some("French"));
    assert_eq!(report.cards()[2].language(), Some("English"));
    assert!(report.issues().is_empty());

    let candidates = [
        CardCandidate::with_language("fr-printing", "Counterspell", "STA", "15", "French"),
        CardCandidate::with_language("en-printing", "Counterspell", "STA", "15", "English"),
    ];
    let collection = build_collection(report, &candidates)?;
    assert!(collection.unresolved().is_empty());
    let printings = collection
        .items()
        .iter()
        .map(|item| (item.uuid(), item.finish(), item.quantity()))
        .collect::<Vec<_>>();
    assert_eq!(
        printings,
        [
            ("fr-printing", CardFinish::Normal, 1),
            ("fr-printing", CardFinish::Foil, 2),
            ("en-printing", CardFinish::Normal, 3),
        ]
    );
    Ok(())
}

#[test]
fn rejects_sorted_exports_without_any_magic_folder() {
    let csv = concat!(
        "List Type,Collection,Quantity,Card Name,Set Code,Card Number\n",
        "Deck,mtg,1,Counterspell,STA,15\n",
        "Folder,pok,1,Squirtle,TST,15\n",
    );
    assert!(matches!(
        import_collection_csv(csv.as_bytes()),
        Err(ImportError::NoSortedMagicFolders)
    ));
}

#[test]
fn col_001_and_col_002_import_dragonshield_rows_and_report_invalid_ones() {
    let lf = include_str!("fixtures/dragonshield.csv").replace("\r\n", "\n");
    let crlf = lf.replace('\n', "\r\n");
    assert_dragonshield_import(lf.as_bytes());
    assert_dragonshield_import(crlf.as_bytes());
}

fn assert_dragonshield_import(csv: &[u8]) {
    let report = import_dragonshield_csv(Cursor::new(csv)).expect("fixture should be valid CSV");

    assert_eq!(report.cards().len(), 1);
    let card = &report.cards()[0];
    assert_eq!(card.quantity(), 2);
    assert_eq!(card.name(), "Who, What, When, Where, Why");
    assert_eq!(card.key().set_code(), "UNH");
    assert_eq!(card.key().collector_number(), "001a");

    assert_eq!(report.issues().len(), 3);
    assert_eq!(report.issues()[0].line(), 4);
    assert_eq!(
        report.issues()[0].reason(),
        &RowIssueReason::InvalidQuantity { value: "0".into() }
    );
    assert_eq!(report.issues()[1].line(), 5);
    assert_eq!(
        report.issues()[1].reason(),
        &RowIssueReason::InvalidQuantity {
            value: "not-a-number".into()
        }
    );
    assert_eq!(report.issues()[2].line(), 6);
    assert_eq!(
        report.issues()[2].reason(),
        &RowIssueReason::MissingField { field: "Set Code" }
    );
}

#[test]
fn col_001_rejects_a_file_without_required_headers() {
    let csv = "Quantity,Card Name\n1,Abrade\n";

    let error = import_dragonshield_csv(Cursor::new(csv)).expect_err("headers are incomplete");

    assert!(matches!(
        error,
        ImportError::MissingHeaders { names }
            if names == ["Set Code".to_owned(), "Card Number".to_owned()]
    ));
}

#[test]
fn col_001_accepts_an_utf8_byte_order_mark() {
    let csv = "\u{feff}Quantity,Card Name,Set Code,Card Number\n1,Abrade,DOM,117\n";

    let report = import_dragonshield_csv(Cursor::new(csv)).expect("UTF-8 BOM should be accepted");

    assert_eq!(report.cards().len(), 1);
    assert_eq!(report.cards()[0].name(), "Abrade");
}

#[test]
fn col_001_accepts_the_quoted_separator_directive_exported_by_dragonshield() {
    let csv = "\"sep=,\"\r\n\
Folder Name,Quantity,Trade Quantity,Card Name,Set Code,Set Name,Card Number,Condition,Printing,Language,Price Bought,Date Bought,LOW,MID,MARKET\r\n\
Tokens,1,0,A Mysterious Creature,TMKM,Murders at Karlov Manor Tokens,21,NearMint,Normal,French,0.20,2026-04-03,0.01,0.28,0.28\r\n";

    let report = import_dragonshield_csv(Cursor::new(csv))
        .expect("an unmodified DragonShield export should be accepted");

    assert_eq!(report.cards().len(), 1);
    assert_eq!(report.cards()[0].name(), "A Mysterious Creature");
    assert_eq!(report.cards()[0].key().set_code(), "TMKM");
    assert_eq!(report.cards()[0].key().collector_number(), "21");
    assert_eq!(report.cards()[0].language(), Some("French"));
    assert_eq!(report.cards()[0].printing(), Some("Normal"));
}
