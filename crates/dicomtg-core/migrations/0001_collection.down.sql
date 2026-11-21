DROP TABLE import_issues;
DROP TABLE unresolved_candidates;
DROP TABLE unresolved_cards;
DROP TABLE collection_items;
DELETE FROM schema_migrations WHERE version = 1;
