ALTER TABLE characters DROP CONSTRAINT name_is_unique;
CREATE UNIQUE INDEX characters_name_lower_unique ON characters (lower(name));
