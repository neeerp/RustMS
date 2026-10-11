-- Key bindings point at skill and item ids (e.g. 1001004), which do not fit
-- in a SMALLINT.
ALTER TABLE keybindings ALTER COLUMN action TYPE INTEGER;
