-- Who raised or dropped whose fame, and when: a character may do so once a
-- day, and to the same character once a month.
CREATE TABLE fame_log (
    id              SERIAL          PRIMARY KEY,
    giver_id        INTEGER         NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    target_id       INTEGER         NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    given_at        TIMESTAMP       NOT NULL DEFAULT now()
);
CREATE INDEX fame_log_giver ON fame_log (giver_id, given_at);
