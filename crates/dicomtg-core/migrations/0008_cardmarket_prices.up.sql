ALTER TABLE catalog_cards
    ADD COLUMN cardmarket_id INTEGER CHECK (cardmarket_id IS NULL OR cardmarket_id > 0);

ALTER TABLE collection_items RENAME TO collection_items_v7;

CREATE TABLE collection_items (
    position INTEGER NOT NULL UNIQUE CHECK (position >= 0),
    uuid TEXT NOT NULL,
    finish TEXT NOT NULL CHECK (finish IN ('normal', 'foil', 'etched')),
    quantity INTEGER NOT NULL CHECK (quantity > 0 AND quantity <= 4294967295),
    PRIMARY KEY (uuid, finish)
);

INSERT INTO collection_items (position, uuid, finish, quantity)
SELECT position, uuid, 'normal', quantity FROM collection_items_v7;

DROP TABLE collection_items_v7;

ALTER TABLE reference_prices RENAME TO reference_prices_v7;

CREATE TABLE reference_prices (
    uuid TEXT NOT NULL CHECK (length(uuid) > 0),
    finish TEXT NOT NULL CHECK (finish IN ('normal', 'foil', 'etched')),
    amount_cents INTEGER NOT NULL CHECK (amount_cents >= 0),
    currency TEXT NOT NULL CHECK (currency = 'EUR'),
    as_of_date TEXT NOT NULL CHECK (length(as_of_date) = 10),
    PRIMARY KEY (uuid, finish)
);

INSERT INTO reference_prices (uuid, finish, amount_cents, currency, as_of_date)
SELECT uuid, 'normal', amount_cents, currency, as_of_date FROM reference_prices_v7;

DROP TABLE reference_prices_v7;

CREATE TABLE reference_update_state (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    last_successful_date TEXT NOT NULL CHECK (length(last_successful_date) = 10)
);
