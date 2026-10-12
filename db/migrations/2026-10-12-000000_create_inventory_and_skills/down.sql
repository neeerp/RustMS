DROP INDEX IF EXISTS sessions_character_id_idx;
DROP INDEX IF EXISTS sessions_account_id_idx;
DROP INDEX IF EXISTS characters_accountid_idx;
ALTER TABLE characters DROP COLUMN IF EXISTS spawn_point, DROP COLUMN IF EXISTS sp;
ALTER TABLE accounts DROP COLUMN IF EXISTS gm_level;
DROP TABLE IF EXISTS skills;
DROP TABLE IF EXISTS equipment_stats;
DROP TABLE IF EXISTS inventory_items;
