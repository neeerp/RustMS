-- Where a character came from before a trip, to go back to when it ends.
-- kind 0: the town they entered the Free Market from; otherwise an event
-- invitation's returnMapQR, for the map it was used in. Its own table
-- because characters is at diesel's 32-column limit.
CREATE TABLE saved_locations (
    character_id    INTEGER         NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    kind            SMALLINT        NOT NULL,
    map_id          INTEGER         NOT NULL,
    PRIMARY KEY (character_id, kind)
);
