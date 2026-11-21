-- Additional collections cannot be represented by the previous schema; keep only Ma collection.
ALTER TABLE collection_items RENAME TO collection_items_v9;
ALTER TABLE unresolved_candidates RENAME TO unresolved_candidates_v9;
ALTER TABLE unresolved_cards RENAME TO unresolved_cards_v9;
ALTER TABLE import_issues RENAME TO import_issues_v9;

CREATE TABLE collection_items (
    position INTEGER NOT NULL UNIQUE CHECK (position >= 0),
    uuid TEXT NOT NULL,
    finish TEXT NOT NULL CHECK (finish IN ('normal', 'foil', 'etched')),
    quantity INTEGER NOT NULL CHECK (quantity > 0 AND quantity <= 4294967295),
    PRIMARY KEY (uuid, finish)
);
INSERT INTO collection_items (position, uuid, finish, quantity)
SELECT position, uuid, finish, quantity FROM collection_items_v9 WHERE collection_id = 1;

CREATE TABLE unresolved_cards (
    id INTEGER PRIMARY KEY,
    position INTEGER NOT NULL UNIQUE CHECK (position >= 0),
    name TEXT NOT NULL,
    set_code TEXT NOT NULL,
    collector_number TEXT NOT NULL,
    quantity INTEGER NOT NULL CHECK (quantity > 0 AND quantity <= 4294967295),
    match_status TEXT NOT NULL CHECK (match_status IN ('unmatched', 'ambiguous')),
    language TEXT,
    printing TEXT
);
CREATE TABLE unresolved_candidates (
    unresolved_id INTEGER NOT NULL REFERENCES unresolved_cards(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    uuid TEXT NOT NULL,
    PRIMARY KEY (unresolved_id, position)
);
INSERT INTO unresolved_cards
    (id, position, name, set_code, collector_number, quantity, match_status, language, printing)
SELECT id, position, name, set_code, collector_number, quantity, match_status, language, printing
FROM unresolved_cards_v9 WHERE collection_id = 1;
INSERT INTO unresolved_candidates (unresolved_id, position, uuid)
SELECT candidate.unresolved_id, candidate.position, candidate.uuid
FROM unresolved_candidates_v9 candidate
JOIN unresolved_cards_v9 card ON card.id = candidate.unresolved_id
WHERE card.collection_id = 1;

CREATE TABLE import_issues (
    position INTEGER PRIMARY KEY CHECK (position >= 0),
    source_line INTEGER NOT NULL CHECK (source_line >= 0),
    reason_code TEXT NOT NULL CHECK (reason_code IN ('missing_field', 'invalid_quantity')),
    detail TEXT NOT NULL
);
INSERT INTO import_issues (position, source_line, reason_code, detail)
SELECT position, source_line, reason_code, detail FROM import_issues_v9 WHERE collection_id = 1;

DROP TABLE unresolved_candidates_v9;
DROP TABLE unresolved_cards_v9;
DROP TABLE collection_items_v9;
DROP TABLE import_issues_v9;
DROP TABLE collections;
DELETE FROM schema_migrations WHERE version = 9;
