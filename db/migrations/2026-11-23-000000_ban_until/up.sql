-- A GM's block of an account (`banned`): why, as Etc/BlockReason.img's
-- number (0 for none given), and until when (none is for good). The login
-- refusal shows both.
ALTER TABLE accounts ADD COLUMN ban_reason SMALLINT NOT NULL DEFAULT 0;
ALTER TABLE accounts ADD COLUMN ban_until TIMESTAMP;
