-- A character's quick slot keys, left to right and top to bottom, as key
-- map indices. A row only once the client has sent a layout.
CREATE TABLE quick_slots (
    character_id    INTEGER         PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    key0            SMALLINT        NOT NULL,
    key1            SMALLINT        NOT NULL,
    key2            SMALLINT        NOT NULL,
    key3            SMALLINT        NOT NULL,
    key4            SMALLINT        NOT NULL,
    key5            SMALLINT        NOT NULL,
    key6            SMALLINT        NOT NULL,
    key7            SMALLINT        NOT NULL
);
