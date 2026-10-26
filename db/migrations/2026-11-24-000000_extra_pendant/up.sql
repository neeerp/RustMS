-- Until when a character's second pendant slot (part 51), bought with
-- 5550000, is open. A row once it was ever bought.
CREATE TABLE extra_pendant_slots (
    character_id    INTEGER         PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    open_until      TIMESTAMP       NOT NULL
);
