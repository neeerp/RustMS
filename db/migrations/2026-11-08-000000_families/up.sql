-- Families: each character has at most one senior and a few juniors, and
-- the senior at the top leads the family, which goes by their name. A
-- character gets a row on first joining one; a row with neither a senior
-- nor juniors is in no family.
CREATE TABLE family_members (
    character_id        INTEGER         PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    senior_id           INTEGER         REFERENCES family_members(character_id) ON DELETE SET NULL,
    -- Rep to spend, and all ever earned.
    reputation          INTEGER         NOT NULL DEFAULT 0,
    total_reputation    INTEGER         NOT NULL DEFAULT 0,
    -- Rep earned today, and given to the senior.
    today_reputation    INTEGER         NOT NULL DEFAULT 0,
    senior_reputation   INTEGER         NOT NULL DEFAULT 0,
    -- The leader's message to the family (the precepts).
    precepts            VARCHAR(200)    NOT NULL DEFAULT ''
);
CREATE INDEX family_members_senior ON family_members (senior_id);
