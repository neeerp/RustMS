-- Slots in each of a character's bags, which the Cash Shop expands. The cash
-- bag always has 96.
ALTER TABLE characters
    ADD COLUMN equip_slots SMALLINT NOT NULL DEFAULT 24 CHECK (equip_slots BETWEEN 24 AND 96),
    ADD COLUMN use_slots SMALLINT NOT NULL DEFAULT 24 CHECK (use_slots BETWEEN 24 AND 96),
    ADD COLUMN setup_slots SMALLINT NOT NULL DEFAULT 24 CHECK (setup_slots BETWEEN 24 AND 96),
    ADD COLUMN etc_slots SMALLINT NOT NULL DEFAULT 24 CHECK (etc_slots BETWEEN 24 AND 96);

-- Slots in the account's storage, which the Cash Shop expands.
ALTER TABLE accounts
    ADD COLUMN storage_slots SMALLINT NOT NULL DEFAULT 4 CHECK (storage_slots BETWEEN 4 AND 48);

-- How many of each commodity the Cash Shop has sold, bought or gifted; the
-- best sellers are its Best Items.
CREATE TABLE cash_sales (
    sn      INTEGER     PRIMARY KEY,
    sold    BIGINT      NOT NULL DEFAULT 0
);
