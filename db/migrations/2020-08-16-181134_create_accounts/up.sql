CREATE TABLE accounts (
    id serial PRIMARY KEY,
    user_name varchar(13) NOT NULL,
    password varchar(128) NOT NULL,
    pin varchar(4) NOT NULL DEFAULT '',
    pic varchar(26) NOT NULL DEFAULT '',

    logged_in BOOLEAN NOT NULL DEFAULT FALSE,
    last_login_at timestamp NULL DEFAULT NULL,
    created_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,

    character_slots SMALLINT NOT NULL DEFAULT 3,
    gender SMALLINT NOT NULL DEFAULT 10,

    banned BOOLEAN NOT NULL DEFAULT FALSE,
    ban_msg text,

    CONSTRAINT user_is_unique UNIQUE (user_name)
);
