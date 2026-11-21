DROP TABLE reference_update_state;

ALTER TABLE reference_prices RENAME TO reference_prices_v8;

CREATE TABLE reference_prices (
    uuid TEXT PRIMARY KEY NOT NULL CHECK (length(uuid) > 0),
    amount_cents INTEGER NOT NULL CHECK (amount_cents >= 0),
    currency TEXT NOT NULL CHECK (currency = 'EUR'),
    as_of_date TEXT NOT NULL CHECK (length(as_of_date) = 10)
);

INSERT INTO reference_prices (uuid, amount_cents, currency, as_of_date)
SELECT uuid, amount_cents, currency, as_of_date
FROM reference_prices_v8
WHERE finish = 'normal';

DROP TABLE reference_prices_v8;

ALTER TABLE collection_items RENAME TO collection_items_v8;

CREATE TABLE collection_items (
    position INTEGER NOT NULL UNIQUE CHECK (position >= 0),
    uuid TEXT PRIMARY KEY NOT NULL,
    quantity INTEGER NOT NULL CHECK (quantity > 0 AND quantity <= 4294967295)
);

INSERT INTO collection_items (position, uuid, quantity)
SELECT MIN(position), uuid, SUM(quantity) FROM collection_items_v8 GROUP BY uuid;

DROP TABLE collection_items_v8;

ALTER TABLE catalog_cards DROP COLUMN cardmarket_id;
