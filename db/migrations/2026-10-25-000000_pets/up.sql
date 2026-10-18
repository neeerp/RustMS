-- A pet, by the cash serial of its item, wherever the item is: carried or
-- back in the Cash Inventory. `summoned` is its place in the owner's list
-- of pets out, or NULL when it rests.
CREATE TABLE pets (
    serial          BIGINT          PRIMARY KEY,
    name            VARCHAR(13)     NOT NULL,
    level           SMALLINT        NOT NULL DEFAULT 1,
    closeness       SMALLINT        NOT NULL DEFAULT 0,
    fullness        SMALLINT        NOT NULL DEFAULT 100,
    skill           INTEGER         NOT NULL DEFAULT 0,
    attribute       SMALLINT        NOT NULL DEFAULT 0,
    summoned        SMALLINT
);
