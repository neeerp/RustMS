-- What a character has won, tied and lost at the mini games (report 164
-- §3): game 1 is Omok, 2 match cards. A row appears with the first game
-- played to its end.
CREATE TABLE minigame_records (
    character_id    INTEGER         NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    game            SMALLINT        NOT NULL CHECK (game BETWEEN 1 AND 2),
    wins            INTEGER         NOT NULL DEFAULT 0,
    ties            INTEGER         NOT NULL DEFAULT 0,
    losses          INTEGER         NOT NULL DEFAULT 0,
    PRIMARY KEY (character_id, game)
);
