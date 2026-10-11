CREATE TABLE character_quests (
    character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    quest_id SMALLINT NOT NULL,
    status SMALLINT NOT NULL CHECK (status IN (1, 2)),
    completed_at TIMESTAMP,
    PRIMARY KEY (character_id, quest_id),
    CHECK ((status = 2) = (completed_at IS NOT NULL))
);
