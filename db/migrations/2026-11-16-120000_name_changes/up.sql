-- Name Change (5400000, report c060): a character's request, bought in the
-- Cash Shop, to take a new name at its next logout. `cash_id` is the
-- receipt the purchase put in the Cash Inventory and `price` the NX
-- Prepaid it cost, given back if the name is taken before the change. A
-- done request keeps its row, with `completed_at`, for the 30-day
-- cooldown; a cancelled one is deleted.
CREATE TABLE name_changes (
    id           SERIAL      PRIMARY KEY,
    character_id INTEGER     NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    cash_id      BIGINT      NOT NULL,
    price        INTEGER     NOT NULL,
    old_name     VARCHAR(13) NOT NULL,
    new_name     VARCHAR(13) NOT NULL,
    requested_at TIMESTAMP   NOT NULL DEFAULT now(),
    completed_at TIMESTAMP,
    failed       BOOLEAN     NOT NULL DEFAULT false
);
-- One open request per character, and per new name.
CREATE UNIQUE INDEX name_changes_open ON name_changes (character_id) WHERE completed_at IS NULL;
CREATE UNIQUE INDEX name_changes_open_name ON name_changes (lower(new_name)) WHERE completed_at IS NULL;
