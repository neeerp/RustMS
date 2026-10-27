-- The records the client keeps beside the quest log, by quest: a
-- `k=v;k=v` string, as the party quests' tries, clears and best times.
CREATE TABLE quest_ex_records (
    character_id    INTEGER         NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    quest_id        SMALLINT        NOT NULL,
    data            TEXT            NOT NULL,
    PRIMARY KEY (character_id, quest_id)
);
