-- Hired merchants (report 165, plan c007): a character's one merchant on a
-- Free Market map, which sells while they are away. It is set up with its
-- owner in the room (state 0), sells for 24 hours once open (state 1), and
-- when the time runs out it closes (state 2) and keeps what is left, with
-- its mesos, for Fredrick to give back.
CREATE TABLE hired_merchants (
    owner_id        INTEGER         PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    channel_id      SMALLINT        NOT NULL,
    map_id          INTEGER         NOT NULL,
    x               SMALLINT        NOT NULL,
    y               SMALLINT        NOT NULL,
    foothold        SMALLINT        NOT NULL,
    -- The permit (503xxxx), which picks the merchant and the balloon.
    item_id         INTEGER         NOT NULL,
    title           VARCHAR(50)     NOT NULL,
    state           SMALLINT        NOT NULL DEFAULT 0 CHECK (state BETWEEN 0 AND 2),
    opened_at       TIMESTAMP,
    closed_at       TIMESTAMP,
    -- What it has made, after tax, and not yet handed to its owner.
    meso            INTEGER         NOT NULL DEFAULT 0 CHECK (meso >= 0)
);

CREATE INDEX hired_merchants_map ON hired_merchants (channel_id, map_id);

-- What a merchant lists, in the order listed: bundles left of one bundle
-- of the item, at price a bundle. Sold-out rows stay until organized.
CREATE TABLE hired_merchant_items (
    id              BIGSERIAL       PRIMARY KEY,
    owner_id        INTEGER         NOT NULL REFERENCES hired_merchants(owner_id) ON DELETE CASCADE,
    bundles         SMALLINT        NOT NULL CHECK (bundles >= 0),
    price           INTEGER         NOT NULL CHECK (price > 0),
    item_id         INTEGER         NOT NULL,
    quantity        SMALLINT        NOT NULL CHECK (quantity > 0),
    owner           VARCHAR(13)     NOT NULL DEFAULT '',
    flags           SMALLINT        NOT NULL DEFAULT 0,
    expires_at      TIMESTAMP,
    cash_serial     BIGINT
);

CREATE INDEX hired_merchant_items_owner ON hired_merchant_items (owner_id, id);
-- For searching merchants by item (the Owl of Minerva).
CREATE INDEX hired_merchant_items_item ON hired_merchant_items (item_id);

-- Stats of listed equipment, as equipment_stats for carried items.
CREATE TABLE hired_merchant_equipment_stats (
    merchant_item_id BIGINT         PRIMARY KEY REFERENCES hired_merchant_items(id) ON DELETE CASCADE,
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

-- What a merchant sold, for its owner's sold-items list: quantity items
-- for mesos before tax.
CREATE TABLE hired_merchant_sales (
    id              BIGSERIAL       PRIMARY KEY,
    owner_id        INTEGER         NOT NULL REFERENCES hired_merchants(owner_id) ON DELETE CASCADE,
    item_id         INTEGER         NOT NULL,
    quantity        SMALLINT        NOT NULL CHECK (quantity > 0),
    mesos           INTEGER         NOT NULL CHECK (mesos >= 0),
    buyer           VARCHAR(13)     NOT NULL,
    sold_at         TIMESTAMP       NOT NULL DEFAULT now()
);

CREATE INDEX hired_merchant_sales_owner ON hired_merchant_sales (owner_id, id);
