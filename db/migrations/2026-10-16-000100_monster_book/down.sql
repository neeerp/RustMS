DELETE FROM keybindings WHERE key = 48 AND bind_type = 'menu' AND action = 59;

ALTER TABLE characters
    DROP COLUMN monster_book_cover;

DROP TABLE monster_book_cards;
