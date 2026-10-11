DROP INDEX characters_name_lower_unique;
ALTER TABLE characters ADD CONSTRAINT name_is_unique UNIQUE(name);
