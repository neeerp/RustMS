DROP TABLE IF EXISTS wish_lists;
DROP TABLE IF EXISTS cash_items;
ALTER TABLE accounts
    DROP COLUMN IF EXISTS nx_prepaid,
    DROP COLUMN IF EXISTS maple_points,
    DROP COLUMN IF EXISTS nx_credit;
