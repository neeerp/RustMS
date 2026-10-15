-- v83 keys F11 and F12 by scan code, 87 and 88, and keeps one binding each
-- for Ctrl, Shift and Alt. The client used to put F11 and F12 at 69 and 70
-- and give the right-hand Ctrl, Shift and Alt their own keys at 85 to 87
-- (and Scroll Lock 88), copies of the left ones.
DELETE FROM keybindings WHERE key BETWEEN 85 AND 88;
UPDATE keybindings SET key = key + 18 WHERE key IN (69, 70);
