-- A guild alliance (the client's "Guild Union"), named uniquely (ignoring
-- case) within its world. Its guilds point at it through guilds.alliance_id,
-- which is 0 for a guild in none.
CREATE TABLE alliances (
    id              SERIAL          PRIMARY KEY,
    world_id        SMALLINT        NOT NULL,
    name            VARCHAR(12)     NOT NULL,
    title1          VARCHAR(12)     NOT NULL DEFAULT 'Master',
    title2          VARCHAR(12)     NOT NULL DEFAULT 'Jr. Master',
    title3          VARCHAR(12)     NOT NULL DEFAULT 'Member',
    title4          VARCHAR(12)     NOT NULL DEFAULT 'Member',
    title5          VARCHAR(12)     NOT NULL DEFAULT 'Member',
    capacity        SMALLINT        NOT NULL DEFAULT 2,
    notice          VARCHAR(100)    NOT NULL DEFAULT ''
);
CREATE UNIQUE INDEX alliances_world_name ON alliances (world_id, lower(name));

-- A member's rank in their guild's alliance: 1 for its master, 2 for the
-- other guilds' masters, 3 to 5 for everyone else.
ALTER TABLE guild_members
    ADD COLUMN alliance_rank SMALLINT NOT NULL DEFAULT 5 CHECK (alliance_rank BETWEEN 1 AND 5);
