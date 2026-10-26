-- A GM's `!mute` of an account: why, as Etc/ChatBlockReason.img's number
-- (0 is not muted), and until when. The login result carries both, and
-- the client then refuses to chat.
ALTER TABLE accounts ADD COLUMN chat_block_reason SMALLINT NOT NULL DEFAULT 0;
ALTER TABLE accounts ADD COLUMN chat_block_until TIMESTAMP;
