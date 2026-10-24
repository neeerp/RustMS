-- A character's standing at the Mu Lung Dojo (report b158): training
-- points, the rest floor whose score So Gong recorded (0 for none), the
-- first-time trial, and the medal tried for with its kills. A row only
-- once it differs from a new character's.
CREATE TABLE dojo_records (
    character_id        INTEGER     PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    points              INTEGER     NOT NULL DEFAULT 0,
    stage               SMALLINT    NOT NULL DEFAULT 0,
    tutorial_done       BOOLEAN     NOT NULL DEFAULT FALSE,
    vanquisher_stage    INTEGER     NOT NULL DEFAULT 0,
    vanquisher_kills    INTEGER     NOT NULL DEFAULT 0
);
