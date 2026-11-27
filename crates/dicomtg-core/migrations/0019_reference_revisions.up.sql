ALTER TABLE game_localization_refresh ADD COLUMN cards_checksum TEXT;
ALTER TABLE game_localization_refresh ADD COLUMN sets_checksum TEXT;
ALTER TABLE game_localization_refresh ADD COLUMN revision INTEGER NOT NULL DEFAULT 0;

CREATE TABLE deck_search_revision (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    revision INTEGER NOT NULL
);
INSERT INTO deck_search_revision VALUES (1, 0);
