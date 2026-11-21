use std::io::Cursor;

use dicomtg_core::{ImportError, RowIssueReason, import_dragonshield_csv};

#[test]
fn col_001_and_col_002_import_dragonshield_rows_and_report_invalid_ones() {
    let csv = include_bytes!("fixtures/dragonshield.csv");

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
