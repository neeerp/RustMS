DROP INDEX IF EXISTS sessions_account_id_key;
CREATE INDEX sessions_account_id_idx ON sessions (account_id);
