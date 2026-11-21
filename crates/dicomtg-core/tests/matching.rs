use dicomtg_core::{CardCandidate, CollectionCard, MatchResult, PrintingKey, match_card};

fn candidate(uuid: &str, name: &str, set_code: &str, collector_number: &str) -> CardCandidate {
    CardCandidate::new(uuid, name, set_code, collector_number)
}

#[test]
fn acc_002_normalization_preserves_alphanumeric_number() {
    let key = PrintingKey::new(" mom ", " 001a ");

    assert_eq!(key.set_code(), "MOM");
    assert_eq!(key.collector_number(), "001a");
}

#[test]
fn acc_010_normalizes_dragonshield_legacy_set_codes() {
    assert_eq!(PrintingKey::new("LEGI", "54").set_code(), "LEG");
    assert_eq!(PrintingKey::new("DRKI", "72").set_code(), "DRK");
}

#[test]
fn acc_010_normalizes_the_arabian_nights_light_variant() {
    let key = PrintingKey::new("ARN", "37a");

    assert_eq!(key.collector_number(), "37†");
}

#[test]
fn acc_013_matches_the_dragonshield_number_for_burning_inquiry() {
    let owned = CollectionCard::new("Burning Inquiry", "PLST", "854");
    let catalog = [candidate(
        "burning-inquiry",
        "Burning Inquiry",
        "PLST",
        "M10-128",
    )];

    assert_eq!(
        match_card(&owned, &catalog),
        MatchResult::Matched {
            uuid: "burning-inquiry".into()
        }
    );
}

#[test]
fn acc_003_unique_exact_key_matches_uuid() {
    let owned = CollectionCard::new("Archangel Elspeth", " mom ", " 006 ");
    let catalog = [candidate("card-uuid", "Archangel Elspeth", "MOM", "006")];

    assert_eq!(
        match_card(&owned, &catalog),
        MatchResult::Matched {
            uuid: "card-uuid".into()
        }
    );
}

#[test]
fn col_004_name_disambiguates_candidates_with_the_same_key() {
    let owned = CollectionCard::new(" second name ", "TST", "12");
    let catalog = [
        candidate("first", "First Name", "TST", "12"),
        candidate("second", "Second Name", "TST", "12"),
    ];

    assert_eq!(
        match_card(&owned, &catalog),
        MatchResult::Matched {
            uuid: "second".into()
        }
    );
}

#[test]
fn col_004_missing_key_is_unmatched() {
    let owned = CollectionCard::new("Missing Card", "TST", "99");
    let catalog = [candidate("other", "Other Card", "TST", "12")];

    assert_eq!(match_card(&owned, &catalog), MatchResult::Unmatched);
}

#[test]
fn acc_004_multiple_candidates_without_discriminator_are_ambiguous() {
    let owned = CollectionCard::new("Shared Name", "TST", "12");
    let catalog = [
        candidate("first", "Shared Name", "TST", "12"),
        candidate("second", "Shared Name", "TST", "12"),
    ];

    assert_eq!(
        match_card(&owned, &catalog),
        MatchResult::Ambiguous {
            candidate_uuids: vec!["first".into(), "second".into()]
        }
    );
}

#[test]
fn col_004_language_disambiguates_candidates_with_the_same_name_and_key() {
    let owned = CollectionCard::with_language("Counterspell", "STA", "15", "French");
    let catalog = [
        CardCandidate::with_language("english", "Counterspell", "STA", "15", "English"),
        CardCandidate::with_language("french", "Counterspell", "STA", "15", "French"),
    ];

    assert_eq!(
        match_card(&owned, &catalog),
        MatchResult::Matched {
            uuid: "french".into()
        }
    );
}
