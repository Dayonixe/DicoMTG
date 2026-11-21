use std::io::Cursor;

use dicomtg_core::{CardFinish, PriceImportError, import_mtgjson_prices_json};

#[test]
fn price_001_imports_latest_cardmarket_prices_by_finish_in_euros() {
    let prices =
        import_mtgjson_prices_json(Cursor::new(include_bytes!("fixtures/mtgjson_prices.json")))
            .expect("fixture should be valid");

    assert_eq!(prices.prices().len(), 4);
    assert_eq!(prices.prices()[0].uuid(), "cheap-uuid");
    assert_eq!(prices.prices()[0].amount_cents(), 50);
    assert_eq!(prices.prices()[0].currency(), "EUR");
    assert_eq!(prices.prices()[0].as_of_date(), "2026-11-15");
    assert_eq!(prices.prices()[1].uuid(), "expensive-uuid");
    assert_eq!(prices.prices()[1].finish(), CardFinish::Normal);
    assert_eq!(prices.prices()[1].amount_cents(), 1_345);
    assert_eq!(prices.prices()[1].as_of_date(), "2026-11-15");
    assert_eq!(prices.prices()[2].uuid(), "expensive-uuid");
    assert_eq!(prices.prices()[2].finish(), CardFinish::Foil);
    assert_eq!(prices.prices()[2].amount_cents(), 9_999);
    assert_eq!(prices.prices()[3].uuid(), "without-normal-price");
    assert_eq!(prices.prices()[3].finish(), CardFinish::Foil);
    assert_eq!(prices.prices()[3].amount_cents(), 125);
}

#[test]
fn price_002_rejects_negative_or_sub_cent_prices() {
    for value in ["-1.00", "1.001"] {
        let json = price_json("EUR", "2026-11-15", value);

        let error = import_mtgjson_prices_json(Cursor::new(json))
            .expect_err("invalid monetary values must be rejected");

        assert!(matches!(error, PriceImportError::InvalidAmount { .. }));
    }
}

#[test]
fn price_002_rejects_an_unexpected_currency() {
    let json = price_json("USD", "2026-11-15", "1.00");

    let error = import_mtgjson_prices_json(Cursor::new(json))
        .expect_err("the Cardmarket reference price must be in euros");

    assert!(matches!(
        error,
        PriceImportError::UnsupportedCurrency { .. }
    ));
}

#[test]
fn price_002_rejects_an_invalid_date() {
    let json = price_json("EUR", "2026-02-30", "1.00");

    let error = import_mtgjson_prices_json(Cursor::new(json))
        .expect_err("the price date must be a valid ISO calendar date");

    assert!(matches!(error, PriceImportError::InvalidDate { .. }));
}

#[test]
fn price_002_rejects_an_empty_reference_price_set() {
    let json = r#"{"data":{"uuid":{"paper":{"cardmarket":{"currency":"EUR","retail":{}}}}}}"#;

    let error = import_mtgjson_prices_json(Cursor::new(json))
        .expect_err("a schema drift must not erase all stored prices");

    assert!(matches!(error, PriceImportError::EmptyPrices));
}

#[test]
fn acc_011_imports_a_foil_only_price_as_foil() {
    let json = r#"{"data":{"foil-only":{"paper":{"cardmarket":{"currency":"EUR","retail":{"foil":{"2026-11-17":79.99}}}}}}}"#;

    let prices = import_mtgjson_prices_json(Cursor::new(json))
        .expect("a foil-only Cardmarket price should be imported");

    assert_eq!(prices.prices().len(), 1);
    assert_eq!(prices.prices()[0].uuid(), "foil-only");
    assert_eq!(prices.prices()[0].finish(), CardFinish::Foil);
    assert_eq!(prices.prices()[0].amount_cents(), 7_999);
}

fn price_json(currency: &str, date: &str, amount: &str) -> String {
    r#"{"data":{"uuid":{"paper":{"cardmarket":{"currency":"$CURRENCY","retail":{"normal":{"$DATE":$AMOUNT}}}}}}}"#
        .replace("$CURRENCY", currency)
        .replace("$DATE", date)
        .replace("$AMOUNT", amount)
}
