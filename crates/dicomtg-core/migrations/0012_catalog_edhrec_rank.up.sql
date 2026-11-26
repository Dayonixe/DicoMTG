ALTER TABLE catalog_cards ADD COLUMN edhrec_rank INTEGER;

CREATE INDEX catalog_cards_edhrec_rank
    ON catalog_cards (edhrec_rank)
    WHERE edhrec_rank IS NOT NULL;
