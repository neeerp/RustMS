-- Parcels left with Duey for a character, who claims or deletes them in the
-- Duey window. One may carry mesos, an item (one stack), or both; the item
-- columns are null when it carries none. announced is false until the
-- receiver has been told the parcel arrived.
CREATE TABLE parcels (
    id              BIGSERIAL       PRIMARY KEY,
    receiver_id     INTEGER         NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    sender          VARCHAR(13)     NOT NULL,
    mesos           INTEGER         NOT NULL DEFAULT 0 CHECK (mesos >= 0),
    message         TEXT,
    quick           BOOLEAN         NOT NULL DEFAULT false,
    sent_at         TIMESTAMP       NOT NULL DEFAULT now(),
    announced       BOOLEAN         NOT NULL DEFAULT false,
    item_id         INTEGER,
    quantity        SMALLINT        NOT NULL DEFAULT 0 CHECK (quantity >= 0),
    owner           VARCHAR(13)     NOT NULL DEFAULT '',
    flags           SMALLINT        NOT NULL DEFAULT 0,
    expires_at      TIMESTAMP,
    cash_serial     BIGINT
);

CREATE INDEX parcels_receiver ON parcels (receiver_id);

-- Stats of the equipment in a parcel, as equipment_stats for carried items.
CREATE TABLE parcel_equipment_stats (
    parcel_id       BIGINT          PRIMARY KEY REFERENCES parcels(id) ON DELETE CASCADE,
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
