-- A session row follows an account from login through its world-entry
-- ticket into the game, so an account has at most one. Enforcing that here
-- makes opening a session the atomic "is this account online?" check.
--
-- Sessions are ephemeral (both servers clear their own on startup), so
-- dropping existing rows is safe and removes any duplicates.
DELETE FROM sessions;
DROP INDEX IF EXISTS sessions_account_id_idx;
CREATE UNIQUE INDEX sessions_account_id_key ON sessions (account_id);
