-- NPC chat (HeavenClient's INTERACT_HARVEST 55) is an action, as in v83, so
-- Keyboard Setting shows it on its key.
UPDATE keybindings SET bind_type = 'action' WHERE bind_type = 'menu' AND action = 55;
