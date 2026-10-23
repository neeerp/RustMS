-- Gifts that wedding guests gave a groom or a bride from their wishlist
-- (report b085). The receiver takes them from Pila in the Wedding Gift
-- window, oldest first within each inventory type; they wait until then.
CREATE TABLE wedding_gifts (
    id              BIGSERIAL       PRIMARY KEY,
    receiver_id     INTEGER         NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    giver_id        INTEGER         NOT NULL,
    given_at        TIMESTAMP       NOT NULL DEFAULT now(),
    item_id         INTEGER         NOT NULL,
    quantity        SMALLINT        NOT NULL CHECK (quantity > 0),
    owner           VARCHAR(13)     NOT NULL DEFAULT '',
    flags           SMALLINT        NOT NULL DEFAULT 0,
    expires_at      TIMESTAMP,
    cash_serial     BIGINT
);

CREATE INDEX wedding_gifts_receiver ON wedding_gifts (receiver_id, id);

-- Stats of the equipment in a gift, as equipment_stats for carried items.
CREATE TABLE wedding_gift_equipment_stats (
    gift_id         BIGINT          PRIMARY KEY REFERENCES wedding_gifts(id) ON DELETE CASCADE,
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
