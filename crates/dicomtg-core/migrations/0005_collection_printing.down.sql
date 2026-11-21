ALTER TABLE unresolved_cards DROP COLUMN printing;
DELETE FROM schema_migrations WHERE version = 5;
