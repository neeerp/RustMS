DROP TABLE IF EXISTS cash_sales;
ALTER TABLE accounts DROP COLUMN IF EXISTS storage_slots;
ALTER TABLE characters
    DROP COLUMN IF EXISTS equip_slots,
    DROP COLUMN IF EXISTS use_slots,
    DROP COLUMN IF EXISTS setup_slots,
    DROP COLUMN IF EXISTS etc_slots;
