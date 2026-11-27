CREATE TABLE catalog_refresh_state (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    fingerprint TEXT NOT NULL,
    rematch_pending INTEGER NOT NULL CHECK (rematch_pending IN (0, 1))
);
