-- The guild bulletin board. Each guild numbers its own threads from 1; the
-- notice is the thread numbered 0, so a guild has at most one. Replies are
-- numbered across every guild.
CREATE TABLE guild_bbs_threads (
    id              SERIAL          PRIMARY KEY,
    guild_id        INTEGER         NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    local_id        INTEGER         NOT NULL,
    poster_id       INTEGER         NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    title           VARCHAR(25)     NOT NULL,
    body            TEXT            NOT NULL,
    icon            SMALLINT        NOT NULL,
    posted_at       TIMESTAMP       NOT NULL DEFAULT now(),
    UNIQUE (guild_id, local_id)
);

CREATE TABLE guild_bbs_replies (
    id              SERIAL          PRIMARY KEY,
    thread_id       INTEGER         NOT NULL REFERENCES guild_bbs_threads(id) ON DELETE CASCADE,
    poster_id       INTEGER         NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    text            VARCHAR(25)     NOT NULL,
    posted_at       TIMESTAMP       NOT NULL DEFAULT now()
);
CREATE INDEX guild_bbs_replies_thread ON guild_bbs_replies (thread_id);
