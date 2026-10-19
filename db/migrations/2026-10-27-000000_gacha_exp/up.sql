-- EXP a character has from a Writ of Solomon and not yet taken, by
-- double-clicking the EXP gauge. A row only while some is left; its own
-- table because characters is at diesel's 32-column limit.
CREATE TABLE gacha_exp (
    character_id    INTEGER         PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    exp             INTEGER         NOT NULL CHECK (exp > 0)
);
