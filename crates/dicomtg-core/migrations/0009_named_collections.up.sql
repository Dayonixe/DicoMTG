CREATE TABLE collections (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE CHECK (length(trim(name)) BETWEEN 1 AND 80),
    selected INTEGER NOT NULL DEFAULT 1 CHECK (selected IN (0, 1))
);

INSERT INTO collections (id, name, selected)
SELECT 1, 'Ma collection', 1
WHERE EXISTS (SELECT 1 FROM collection_items)
   OR EXISTS (SELECT 1 FROM unresolved_cards)
   OR EXISTS (SELECT 1 FROM import_issues);

ALTER TABLE collection_items RENAME TO collection_items_v8;

CREATE TABLE collection_items (
    collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    uuid TEXT NOT NULL,
    finish TEXT NOT NULL CHECK (finish IN ('normal', 'foil', 'etched')),
    quantity INTEGER NOT NULL CHECK (quantity > 0 AND quantity <= 4294967295),
    PRIMARY KEY (collection_id, uuid, finish),
    UNIQUE (collection_id, position)
);

INSERT INTO collection_items (collection_id, position, uuid, finish, quantity)
SELECT 1, position, uuid, finish, quantity FROM collection_items_v8;

DROP TABLE collection_items_v8;

ALTER TABLE unresolved_candidates RENAME TO unresolved_candidates_v8;
ALTER TABLE unresolved_cards RENAME TO unresolved_cards_v8;

CREATE TABLE unresolved_cards (
    id INTEGER PRIMARY KEY,
    collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    name TEXT NOT NULL,
    set_code TEXT NOT NULL,
    collector_number TEXT NOT NULL,
    quantity INTEGER NOT NULL CHECK (quantity > 0 AND quantity <= 4294967295),
    match_status TEXT NOT NULL CHECK (match_status IN ('unmatched', 'ambiguous')),
    language TEXT,
    printing TEXT,
    UNIQUE (collection_id, position)
);

CREATE TABLE unresolved_candidates (
    unresolved_id INTEGER NOT NULL REFERENCES unresolved_cards(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    uuid TEXT NOT NULL,
    PRIMARY KEY (unresolved_id, position)
);

INSERT INTO unresolved_cards
    (id, collection_id, position, name, set_code, collector_number, quantity, match_status,
     language, printing)
SELECT id, 1, position, name, set_code, collector_number, quantity, match_status,
       language, printing
FROM unresolved_cards_v8;

INSERT INTO unresolved_candidates (unresolved_id, position, uuid)
SELECT unresolved_id, position, uuid FROM unresolved_candidates_v8;

DROP TABLE unresolved_candidates_v8;
DROP TABLE unresolved_cards_v8;

ALTER TABLE import_issues RENAME TO import_issues_v8;

CREATE TABLE import_issues (
    collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    source_line INTEGER NOT NULL CHECK (source_line >= 0),
    reason_code TEXT NOT NULL CHECK (reason_code IN ('missing_field', 'invalid_quantity')),
    detail TEXT NOT NULL,
    PRIMARY KEY (collection_id, position)
);

INSERT INTO import_issues (collection_id, position, source_line, reason_code, detail)
SELECT 1, position, source_line, reason_code, detail FROM import_issues_v8;

DROP TABLE import_issues_v8;
