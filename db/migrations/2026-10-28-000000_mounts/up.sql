-- A character's tamed monster: its level, EXP and tiredness (v83
-- SET_TAMING_MOB_INFO). A row only once it differs from a new mount's
-- 1 / 0 / 0; its own table because characters is at diesel's 32-column
-- limit.
CREATE TABLE character_mounts (
    character_id    INTEGER         PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    level           INTEGER         NOT NULL,
    exp             INTEGER         NOT NULL,
    tiredness       INTEGER         NOT NULL
);
