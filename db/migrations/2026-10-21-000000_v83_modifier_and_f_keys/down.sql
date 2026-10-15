UPDATE keybindings SET key = key - 18 WHERE key IN (87, 88);
INSERT INTO keybindings (character_id, key, bind_type, action)
SELECT character_id, CASE key WHEN 29 THEN 85 WHEN 42 THEN 86 ELSE 87 END, bind_type, action
FROM keybindings
WHERE key IN (29, 42, 56);
