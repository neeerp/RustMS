-- Mesos in the account's storage.
ALTER TABLE accounts
    ADD COLUMN storage_meso INTEGER NOT NULL DEFAULT 0 CHECK (storage_meso >= 0);

-- Items in the account's storage, shared by its characters. position is the
-- item's place in the storage, in the order stored or sorted.
CREATE TABLE storage_items (
    id              BIGSERIAL       PRIMARY KEY,
    account_id      INTEGER         NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    position        SMALLINT        NOT NULL CHECK (position >= 0),
    item_id         INTEGER         NOT NULL,
    quantity        SMALLINT        NOT NULL DEFAULT 1 CHECK (quantity > 0),
    owner           VARCHAR(13)     NOT NULL DEFAULT '',
    flags           SMALLINT        NOT NULL DEFAULT 0,
    expires_at      TIMESTAMP,
    cash_serial     BIGINT,
    CONSTRAINT storage_position_is_unique UNIQUE (account_id, position)
);

-- Stats of stored equipment, as equipment_stats for carried items.
CREATE TABLE storage_equipment_stats (
    storage_item_id BIGINT          PRIMARY KEY REFERENCES storage_items(id) ON DELETE CASCADE,
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
