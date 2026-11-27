CREATE TABLE localized_card_names (
    card_name TEXT NOT NULL,
    language TEXT NOT NULL,
    position INTEGER NOT NULL,
    printed_name TEXT NOT NULL,
    search_name TEXT NOT NULL,
    PRIMARY KEY (card_name, language, position)
) WITHOUT ROWID;

CREATE TABLE localized_set_names (
    set_code TEXT NOT NULL,
    language TEXT NOT NULL,
    printed_name TEXT NOT NULL,
    PRIMARY KEY (set_code, language)
) WITHOUT ROWID;

CREATE TABLE game_localization_refresh (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    updated_at INTEGER NOT NULL
);
