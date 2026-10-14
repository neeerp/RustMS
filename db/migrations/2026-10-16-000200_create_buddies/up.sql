-- A row (owner, buddy) puts `buddy` on `owner`'s buddy list. A pending row
-- is a request: `buddy` asked to be added and `owner` hasn't answered yet.
CREATE TABLE buddies (
    character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    buddy_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    group_name VARCHAR(16) NOT NULL DEFAULT 'Default Group',
    pending BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (character_id, buddy_id),
    CHECK (character_id <> buddy_id)
);
CREATE INDEX buddies_buddy_id ON buddies (buddy_id);
