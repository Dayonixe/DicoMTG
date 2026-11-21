ALTER TABLE unresolved_cards DROP COLUMN language;
DELETE FROM schema_migrations WHERE version = 4;
