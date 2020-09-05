CREATE TYPE session_state AS ENUM (
    'before_login',
    'after_login',
    'transition',
    'in_game'
);

CREATE TABLE sessions (
    id              SERIAL              PRIMARY KEY,
    account_id      INTEGER             NOT NULL,
    character_id    INTEGER,
    ip              INET                NOT NULL,
    hwid            VARCHAR(12)         NOT NULL,
    state           SESSION_STATE       NOT NULL DEFAULT 'before_login',
    updated_at      TIMESTAMP           NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at      TIMESTAMP           NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT fk_account
        FOREIGN KEY(account_id)
            REFERENCES accounts(id) ON DELETE CASCADE,

    CONSTRAINT fk_character
        FOREIGN KEY(character_id)
            REFERENCES characters(id) ON DELETE SET NULL
);
