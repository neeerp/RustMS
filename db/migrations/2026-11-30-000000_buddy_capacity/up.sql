-- A character's buddy list capacity once Robert Holly has raised it past
-- the 20 every character starts with. A row only once raised.
CREATE TABLE buddy_capacities (
    character_id    INTEGER     PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    capacity        SMALLINT    NOT NULL
);
