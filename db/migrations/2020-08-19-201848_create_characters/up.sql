CREATE TABLE characters (
    id serial PRIMARY KEY,
    accountid INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    world SMALLINT NOT NULL,
    name varchar(13) NOT NULL,

    level SMALLINT NOT NULL DEFAULT 1,
    exp INTEGER NOT NULL DEFAULT 0,

    stre SMALLINT NOT NULL DEFAULT 12,
    dex SMALLINT NOT NULL DEFAULT 5,
    luk SMALLINT NOT NULL DEFAULT 4,
    int SMALLINT NOT NULL DEFAULT 4,
    hp SMALLINT NOT NULL DEFAULT 50,
    mp SMALLINT NOT NULL DEFAULT 5,
    maxhp SMALLINT NOT NULL DEFAULT 50,
    maxmp SMALLINT NOT NULL DEFAULT 5,
    ap SMALLINT NOT NULL DEFAULT 0,
    fame SMALLINT NOT NULL DEFAULT 0,

    meso INTEGER NOT NULL DEFAULT 0,

    job SMALLINT NOT NULL DEFAULT 0,

    face INTEGER NOT NULL,
    hair INTEGER NOT NULL,
    hair_color INTEGER NOT NULL,
    skin INTEGER NOT NULL,
    gender SMALLINT NOT NULL,

    created_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT name_is_unique UNIQUE(name)
);
