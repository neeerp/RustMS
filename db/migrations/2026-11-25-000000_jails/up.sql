-- Until when a GM's `!jail` keeps a character in the jail map. A row once
-- they were ever jailed; a time past is free.
CREATE TABLE jails (
    character_id    INTEGER         PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    until           TIMESTAMP       NOT NULL
);
