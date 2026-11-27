use std::{error::Error, io::Cursor};

use dicomtg_core::{
    CollectionStore, DeckFormat, build_collection, import_collection_csv, import_decks_csv,
    import_mtgjson_cards_csv,
};

#[test]
fn imports_the_two_deck_exports_and_preserves_sorted_sections() -> Result<(), Box<dyn Error>> {
    let dragonshield = import_decks_csv(
        "\u{feff}\"sep=,\"\r\nCard Type,Quantity,Card Name,Mana Cost,Min Price (MARKET)\r\ncreature,1,\"Healer's Hawk\",{W},0.17\r\nland,10,Forest,,0.09",
        "Deck DragonShield.csv",
    )?;
    assert_eq!(dragonshield[0].name, "Deck DragonShield");
    assert_eq!(dragonshield[0].format, DeckFormat::Freeform);
    assert_eq!(dragonshield[0].cards[1].quantity, 10);
    assert_eq!(dragonshield[0].cards[1].card_type, "land");
    assert!(dragonshield[0].cards[0].set_code.is_empty());

    let sorted = import_decks_csv(
        concat!(
            "\"sep=,\"\nList Type,List Name,Collection,Format,Board,Quantity,Card Name,Set Code,Card Number,Printing,Language\n",
            "Folder,MyCard,mtg,,,15,Abrade,DSK,149,Normal,fr\n",
            "Deck,Faldorn,mtg,mtg_commander,Commander,1,\"Faldorn, Dread Wolf Herald\",clb,647,Foil,\n",
            "Deck,Faldorn,mtg,mtg_commander,Main Deck,1,Abrade,DSK,149,Normal,fr\n",
            "Deck,Faldorn,mtg,mtg_commander,Sideboard,1,Forest,DSK,272,Normal,\n",
            "Deck,Kaito,mtg,mtg_standard,Main Deck,1,Island,DSK,270,Normal,\n",
            "Deck,Ignore,pokemon,,,1,Pikachu,SV,001,Normal,\n",
        ),
        "export_sorted.csv",
    )?;
    assert_eq!(
        sorted
            .iter()
            .map(|deck| deck.name.as_str())
            .collect::<Vec<_>>(),
        ["Faldorn", "Kaito"]
    );
    assert_eq!(sorted[0].format, DeckFormat::Commander);
    assert_eq!(
        sorted[0]
            .cards
            .iter()
            .map(|card| card.board.as_str())
            .collect::<Vec<_>>(),
        ["commander", "main", "sideboard"]
    );
    assert_eq!(sorted[0].cards[0].set_code, "CLB");
    assert_eq!(sorted[0].cards[0].printing, "Foil");
    assert_eq!(sorted[0].cards[1].language, "fr");
    Ok(())
}

#[test]
fn edits_decks_atomically_and_lists_stock_from_all_collections() -> Result<(), Box<dyn Error>> {
    let mut store = CollectionStore::in_memory()?;
    let catalog = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid\n",
        "R,R,nonfoil,,English,{R},2.0,Abrade,1,,common,TST,Destroy target artifact,,Instant,Instant,abrade\n",
    )))?;
    store.replace_catalog(&catalog)?;
    let report = import_collection_csv(
        "Quantity,Card Name,Set Code,Card Number\n2,Abrade,TST,1".as_bytes(),
    )?;
    let candidates = catalog
        .cards()
        .iter()
        .map(|card| card.matching_candidate())
        .collect::<Vec<_>>();
    let inventory = build_collection(report, &candidates)?;
    let id = store.create_collection("Ma collection", &inventory)?;
    store.set_collection_selected(id, false)?;
    let stock = serde_json::to_value(store.load_deck_workspace()?)?;
    assert_eq!(stock["holdings"][0]["quantity"], 2);
    assert_eq!(stock["candidates"][0]["colorIdentity"][0], "R");
    assert_eq!(store.find_deck_cards("abr", &[], false)?.len(), 1);
    assert!(store.find_deck_cards("abr", &[], true)?.is_empty());
    assert_eq!(
        store
            .find_deck_cards("", &["Abrade".to_owned()], false)?
            .len(),
        1
    );

    let original =
        import_decks_csv("Quantity,Card Name,Card Type\n1,Abrade,instant", "Test.csv")?.remove(0);
    let saved = store.save_deck(None, original.clone())?;
    let mut replacement = original.clone();
    replacement.cards[0].quantity = 2;
    let next = store.save_deck(Some((saved.id, saved.revision)), replacement)?;
    assert!(
        store
            .save_deck(Some((saved.id, saved.revision)), original)
            .is_err()
    );
    assert!(store.delete_deck(saved.id, saved.revision).is_err());
    let saved_decks = serde_json::to_value(store.list_saved_decks()?)?;
    assert_eq!(saved_decks[0]["cards"][0]["quantity"], 2);
    store.delete_deck(next.id, next.revision)?;
    assert!(store.list_saved_decks()?.is_empty());
    Ok(())
}

#[test]
fn finds_unique_front_face_aliases_without_guessing_ambiguous_names() -> Result<(), Box<dyn Error>>
{
    let mut store = CollectionStore::in_memory()?;
    let catalog = import_mtgjson_cards_csv(Cursor::new(concat!(
        "colorIdentity,colors,finishes,keywords,language,manaCost,manaValue,name,number,power,rarity,setCode,text,toughness,type,types,uuid\n",
        "U,U,nonfoil,,English,{2}{U},3.0,Behold the Unspeakable // Vision of the Unspeakable,48,,uncommon,NEO,,,Enchantment,Enchantment,behold\n",
        "B,B,nonfoil,,English,{1}{B},2.0,Life of Toshiro Umezawa // Memory of Toshiro,108,,uncommon,NEO,,,Enchantment,Enchantment,life\n",
        "U,U,nonfoil,,English,{1}{U},2.0,Riddle // First,1,,common,TST,,,Creature,Creature,riddle1\n",
        "U,U,nonfoil,,English,{1}{U},2.0,Riddle // Second,2,,common,TST,,,Creature,Creature,riddle2\n",
    )))?;
    store.replace_catalog(&catalog)?;
    let names = [
        "behold the unspeakable",
        "Life of Toshiro Umezawa",
        "Riddle",
    ]
    .map(str::to_owned);
    let cards = serde_json::to_value(store.find_deck_cards("", &names, false)?)?;
    let cards = cards.as_array().unwrap();
    assert_eq!(cards.len(), 2);
    assert!(cards.iter().any(|card| {
        card["name"] == "Behold the Unspeakable // Vision of the Unspeakable"
            && card["requestedName"] == "behold the unspeakable"
            && card["collectorNumber"] == "48"
    }));
    assert!(cards.iter().any(|card| {
        card["name"] == "Life of Toshiro Umezawa // Memory of Toshiro"
            && card["requestedName"] == "Life of Toshiro Umezawa"
    }));
    Ok(())
}
