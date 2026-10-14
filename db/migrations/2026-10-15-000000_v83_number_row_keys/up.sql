-- The number row used to be keyed 0 to 9 from 2; v83 keys it by scan code,
-- 1 to 9 then 0 from 2. Step through 1000+ to keep (character_id, key)
-- unique while shifting.
UPDATE keybindings SET key = key + 1000 WHERE key BETWEEN 2 AND 11;
UPDATE keybindings SET key = CASE WHEN key = 1002 THEN 11 ELSE key - 1001 END WHERE key BETWEEN 1002 AND 1011;
