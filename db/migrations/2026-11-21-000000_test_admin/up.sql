-- An account the client treats as v83's admin account (login grade 0x11):
-- it can't drop items or mesos, trade, or open a store. Opt-in, set by SQL.
ALTER TABLE accounts ADD COLUMN test_admin BOOLEAN NOT NULL DEFAULT FALSE;
