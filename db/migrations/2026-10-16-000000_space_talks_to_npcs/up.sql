-- Space talks to the nearest NPC (HeavenClient's INTERACT_HARVEST 55) for new
-- characters. Give it to earlier ones too, unless they already use Space or
-- have bound the action elsewhere.
INSERT INTO keybindings (character_id, key, bind_type, action)
SELECT c.id, 57, 'menu', 55
FROM characters c
WHERE NOT EXISTS (
    SELECT 1 FROM keybindings k
    WHERE k.character_id = c.id
      AND (k.key = 57 OR (k.bind_type = 'menu' AND k.action = 55))
);
