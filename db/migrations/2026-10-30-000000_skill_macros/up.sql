-- A character's skill macros, by place in the list: a type-8 key runs the
-- macro at its place.
CREATE TABLE skill_macros (
    character_id    INTEGER         NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    place           SMALLINT        NOT NULL,
    name            TEXT            NOT NULL,
    silent          BOOLEAN         NOT NULL,
    skill1          INTEGER         NOT NULL,
    skill2          INTEGER         NOT NULL,
    skill3          INTEGER         NOT NULL,
    PRIMARY KEY (character_id, place)
);
