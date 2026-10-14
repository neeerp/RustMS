-- The account's Cash Shop balances: NX Credit, Maple Points and NX Prepaid.
ALTER TABLE accounts
    ADD COLUMN nx_credit INTEGER NOT NULL DEFAULT 0 CHECK (nx_credit >= 0),
    ADD COLUMN maple_points INTEGER NOT NULL DEFAULT 0 CHECK (maple_points >= 0),
    ADD COLUMN nx_prepaid INTEGER NOT NULL DEFAULT 0 CHECK (nx_prepaid >= 0);

-- Items in the account's Cash Inventory, shared by its characters. The id is
-- the item's cash serial number.
CREATE TABLE cash_items (
    id              BIGSERIAL       PRIMARY KEY,
    account_id      INTEGER         NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    item_id         INTEGER         NOT NULL,
    sn              INTEGER         NOT NULL,
    quantity        SMALLINT        NOT NULL DEFAULT 1 CHECK (quantity > 0),
    gift_from       VARCHAR(13)     NOT NULL DEFAULT '',
    expires_at      TIMESTAMP
);

CREATE INDEX cash_items_account_id_idx ON cash_items (account_id);

-- A character's Cash Shop wish list: up to ten commodity serial numbers.
CREATE TABLE wish_lists (
    character_id    INTEGER         NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    slot            SMALLINT        NOT NULL CHECK (slot >= 0 AND slot < 10),
    sn              INTEGER         NOT NULL,
    PRIMARY KEY (character_id, slot)
);
