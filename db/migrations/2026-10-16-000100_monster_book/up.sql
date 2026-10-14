-- Monster Book: how many copies of each card a character has collected
-- (1 to 5), and the card on the book's cover (0 for none).
CREATE TABLE monster_book_cards (
    character_id    INTEGER         NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    card_id         INTEGER         NOT NULL,
    level           SMALLINT        NOT NULL CHECK (level BETWEEN 1 AND 5),
    PRIMARY KEY (character_id, card_id)
);

ALTER TABLE characters
    ADD COLUMN monster_book_cover INTEGER NOT NULL DEFAULT 0;

-- B opens the Monster Book (HeavenClient's menu action 59).
INSERT INTO keybindings (character_id, key, bind_type, action)
SELECT id, 48, 'menu', 59 FROM characters
ON CONFLICT (character_id, key) DO NOTHING;
