CREATE TYPE keybind_type AS ENUM (
    'nil',
    'skill',
    'item',
    'cash',
    'menu',
    'action',
    'face',
    'macro',
    'text'
);

-- TODO: action and bind_type could be enums but they aren't really consistent in
-- TODO: numbering... we'd either need filler or another mapping...


-- TODO: Action constraint is not needed! Only key constraint!
CREATE TABLE keybindings (
    id              SERIAL          PRIMARY KEY,
    character_id    INTEGER         NOT NULL,
    key             SMALLINT        NOT NULL DEFAULT 0, 
    bind_type       KEYBIND_TYPE    NOT NULL DEFAULT 'nil',
    action          SMALLINT        NOT NULL DEFAULT 0,

    CONSTRAINT fk_character
        FOREIGN KEY(character_id)
            REFERENCES characters(id),

    CONSTRAINT key_is_unique_per_character UNIQUE(character_id, key)
);
