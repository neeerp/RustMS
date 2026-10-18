-- A guild, named uniquely (ignoring case) within its world. The emblem is
-- all zeros until one is made.
CREATE TABLE guilds (
    id              SERIAL          PRIMARY KEY,
    world_id        SMALLINT        NOT NULL,
    name            VARCHAR(12)     NOT NULL,
    title1          VARCHAR(12)     NOT NULL DEFAULT 'Master',
    title2          VARCHAR(12)     NOT NULL DEFAULT 'Jr. Master',
    title3          VARCHAR(12)     NOT NULL DEFAULT 'Member',
    title4          VARCHAR(12)     NOT NULL DEFAULT '',
    title5          VARCHAR(12)     NOT NULL DEFAULT '',
    capacity        SMALLINT        NOT NULL DEFAULT 10,
    mark_bg         SMALLINT        NOT NULL DEFAULT 0,
    mark_bg_color   SMALLINT        NOT NULL DEFAULT 0,
    mark            SMALLINT        NOT NULL DEFAULT 0,
    mark_color      SMALLINT        NOT NULL DEFAULT 0,
    notice          TEXT            NOT NULL DEFAULT '',
    points          INTEGER         NOT NULL DEFAULT 0,
    alliance_id     INTEGER         NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX guilds_world_name ON guilds (world_id, lower(name));

-- A character's place in their guild; a character has at most one. Rank 1
-- is the master.
CREATE TABLE guild_members (
    character_id    INTEGER         PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    guild_id        INTEGER         NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    rank            SMALLINT        NOT NULL CHECK (rank BETWEEN 1 AND 5),
    joined_at       TIMESTAMP       NOT NULL DEFAULT now()
);
CREATE INDEX guild_members_guild_id ON guild_members (guild_id);
