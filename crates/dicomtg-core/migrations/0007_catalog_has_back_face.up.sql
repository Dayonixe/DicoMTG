ALTER TABLE catalog_cards
ADD COLUMN has_back_face INTEGER NOT NULL DEFAULT 0
CHECK (has_back_face IN (0, 1));
