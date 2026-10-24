-- Players' reports of other players (v83 CLAIM, report b155): kind 1 is a
-- conversation, with the lines the reporter's client sent; kind 0 an
-- illegal program. Each reporter has a few a week.
CREATE TABLE reports (
    id              SERIAL          PRIMARY KEY,
    reporter_id     INTEGER         NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    victim_id       INTEGER         NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    kind            SMALLINT        NOT NULL,
    reason          SMALLINT        NOT NULL,
    description     TEXT            NOT NULL,
    chat_log        TEXT            NOT NULL,
    reported_at     TIMESTAMP       NOT NULL DEFAULT now()
);
CREATE INDEX reports_reporter ON reports (reporter_id, reported_at);
