-- The maps a character keeps for their teleport rocks, by place in the
-- list: five for the regular rock, ten for the VIP one.
CREATE TABLE teleport_rocks (
    character_id    INTEGER         NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    vip             BOOLEAN         NOT NULL,
    place           SMALLINT        NOT NULL,
    map_id          INTEGER         NOT NULL,
    PRIMARY KEY (character_id, vip, place)
);
