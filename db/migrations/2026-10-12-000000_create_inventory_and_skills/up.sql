-- Item instances. inventory_type follows the client's numbering:
-- -1 equipped, 1 equip, 2 use, 3 setup, 4 etc, 5 cash. Equipped items use
-- negative slots (cash cosmetics at -100 and below).
CREATE TABLE inventory_items (
    id              BIGSERIAL       PRIMARY KEY,
    character_id    INTEGER         NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    inventory_type  SMALLINT        NOT NULL CHECK (inventory_type IN (-1, 1, 2, 3, 4, 5)),
    slot            SMALLINT        NOT NULL,
    item_id         INTEGER         NOT NULL,
    quantity        SMALLINT        NOT NULL DEFAULT 1 CHECK (quantity > 0),
    owner           VARCHAR(13)     NOT NULL DEFAULT '',
    flags           SMALLINT        NOT NULL DEFAULT 0,
    expires_at      TIMESTAMP,
    cash_serial     BIGINT,
    CONSTRAINT inventory_slot_is_unique UNIQUE (character_id, inventory_type, slot),
    CONSTRAINT equipped_slots_are_negative CHECK ((inventory_type = -1) = (slot < 0))
);

-- Per-instance equipment stats; one row per equipment item.
CREATE TABLE equipment_stats (
    item_row_id     BIGINT          PRIMARY KEY REFERENCES inventory_items(id) ON DELETE CASCADE,
    upgrade_slots   SMALLINT        NOT NULL DEFAULT 0,
    level           SMALLINT        NOT NULL DEFAULT 0,
    stre            SMALLINT        NOT NULL DEFAULT 0,
    dex             SMALLINT        NOT NULL DEFAULT 0,
    int             SMALLINT        NOT NULL DEFAULT 0,
    luk             SMALLINT        NOT NULL DEFAULT 0,
    hp              SMALLINT        NOT NULL DEFAULT 0,
    mp              SMALLINT        NOT NULL DEFAULT 0,
    watk            SMALLINT        NOT NULL DEFAULT 0,
    matk            SMALLINT        NOT NULL DEFAULT 0,
    wdef            SMALLINT        NOT NULL DEFAULT 0,
    mdef            SMALLINT        NOT NULL DEFAULT 0,
    acc             SMALLINT        NOT NULL DEFAULT 0,
    avoid           SMALLINT        NOT NULL DEFAULT 0,
    hands           SMALLINT        NOT NULL DEFAULT 0,
    speed           SMALLINT        NOT NULL DEFAULT 0,
    jump            SMALLINT        NOT NULL DEFAULT 0,
    vicious         INTEGER         NOT NULL DEFAULT 0,
    item_level      SMALLINT        NOT NULL DEFAULT 0,
    item_exp        INTEGER         NOT NULL DEFAULT 0
);

CREATE TABLE skills (
    character_id    INTEGER         NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    skill_id        INTEGER         NOT NULL,
    level           INTEGER         NOT NULL CHECK (level >= 0),
    master_level    INTEGER         NOT NULL DEFAULT 0,
    expires_at      TIMESTAMP,
    PRIMARY KEY (character_id, skill_id)
);

ALTER TABLE accounts
    ADD COLUMN gm_level SMALLINT NOT NULL DEFAULT 0;

ALTER TABLE characters
    ADD COLUMN sp SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN spawn_point SMALLINT NOT NULL DEFAULT 0;

CREATE INDEX characters_accountid_idx ON characters (accountid);
CREATE INDEX sessions_account_id_idx ON sessions (account_id);
CREATE INDEX sessions_character_id_idx ON sessions (character_id);
