-- Engaged and married couples (report b052), while the couple lasts. The
-- groom is the man; a character is in at most one couple.
CREATE TABLE marriages (
    id          SERIAL      PRIMARY KEY,
    groom_id    INTEGER     NOT NULL UNIQUE REFERENCES characters(id) ON DELETE CASCADE,
    bride_id    INTEGER     NOT NULL UNIQUE REFERENCES characters(id) ON DELETE CASCADE,
    status      SMALLINT    NOT NULL,
    item_id     INTEGER     NOT NULL
);
