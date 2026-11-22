ALTER TABLE collections ADD COLUMN drive_url TEXT;

CREATE TABLE collection_id_sequence (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    last_id INTEGER NOT NULL
);
