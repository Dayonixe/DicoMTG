CREATE TABLE catalog_cards (
    position INTEGER NOT NULL UNIQUE CHECK (position >= 0),
    uuid TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    set_code TEXT NOT NULL,
    collector_number TEXT NOT NULL,
    rarity TEXT NOT NULL,
    mana_value REAL NOT NULL CHECK (mana_value >= 0),
    language TEXT NOT NULL,
    mana_cost TEXT,
    power TEXT,
    toughness TEXT,
    type_line TEXT NOT NULL,
    rules_text TEXT
);

CREATE INDEX catalog_cards_matching
    ON catalog_cards (set_code, collector_number);
CREATE INDEX catalog_cards_name
    ON catalog_cards (name COLLATE NOCASE);

CREATE TABLE catalog_facets (
    card_uuid TEXT NOT NULL REFERENCES catalog_cards(uuid) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('color', 'color_identity', 'type', 'keyword', 'finish')),
    position INTEGER NOT NULL CHECK (position >= 0),
    value TEXT NOT NULL,
    PRIMARY KEY (card_uuid, kind, position)
);

CREATE INDEX catalog_facets_lookup
    ON catalog_facets (kind, value, card_uuid);
