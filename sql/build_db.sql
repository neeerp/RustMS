CREATE TABLE IF NOT EXISTS accounts (
    id serial PRIMARY KEY,
    user_name varchar(13) NOT NULL,
    password varchar(128) NOT NULL,
    pin varchar(4) NOT NULL DEFAULT '',
    pic varchar(26) NOT NULL DEFAULT '',
    created_at timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);
