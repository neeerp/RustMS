-- The potions a character's pet gives them from the HP and MP pouches'
-- slots, 0 for none. A row only while one is set.
CREATE TABLE auto_potions (
    character_id    INTEGER         PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    hp_item         INTEGER         NOT NULL,
    mp_item         INTEGER         NOT NULL
);
