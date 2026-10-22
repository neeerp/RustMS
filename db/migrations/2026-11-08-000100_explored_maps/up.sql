-- The maps a character has visited toward each explorer medal, keyed by
-- the info record (27010 to 27019) that shows the medal's count.
CREATE TABLE explored_maps (
    character_id    INTEGER         NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    record_id       SMALLINT        NOT NULL,
    map_id          INTEGER         NOT NULL,
    PRIMARY KEY (character_id, record_id, map_id)
);
