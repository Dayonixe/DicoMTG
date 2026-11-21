CREATE TABLE collection_items (
    position INTEGER NOT NULL UNIQUE CHECK (position >= 0),
    uuid TEXT PRIMARY KEY NOT NULL,
    quantity INTEGER NOT NULL CHECK (quantity > 0 AND quantity <= 4294967295)
);

CREATE TABLE unresolved_cards (
    id INTEGER PRIMARY KEY,
    position INTEGER NOT NULL UNIQUE CHECK (position >= 0),
    name TEXT NOT NULL,
    set_code TEXT NOT NULL,
    collector_number TEXT NOT NULL,
    quantity INTEGER NOT NULL CHECK (quantity > 0 AND quantity <= 4294967295),
    match_status TEXT NOT NULL CHECK (match_status IN ('unmatched', 'ambiguous'))
);

CREATE TABLE unresolved_candidates (
    unresolved_id INTEGER NOT NULL REFERENCES unresolved_cards(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    uuid TEXT NOT NULL,
    PRIMARY KEY (unresolved_id, position)
);

CREATE TABLE import_issues (
    position INTEGER PRIMARY KEY CHECK (position >= 0),
    source_line INTEGER NOT NULL CHECK (source_line >= 0),
    reason_code TEXT NOT NULL CHECK (reason_code IN ('missing_field', 'invalid_quantity')),
    detail TEXT NOT NULL
);
