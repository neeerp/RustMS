-- Lossy: ids above 32767 cannot be narrowed back.
ALTER TABLE keybindings ALTER COLUMN action TYPE SMALLINT USING LEAST(action, 32767)::SMALLINT;
