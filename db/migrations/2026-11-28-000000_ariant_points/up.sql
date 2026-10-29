-- A character's Battle Arena points from the Ariant Coliseum, which
-- Abdullah VIII trades for a chair (Cosmic `ariantPoints`). A row only
-- once they hold any.
CREATE TABLE ariant_records (
    character_id    INTEGER     PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    points          INTEGER     NOT NULL DEFAULT 0
);
