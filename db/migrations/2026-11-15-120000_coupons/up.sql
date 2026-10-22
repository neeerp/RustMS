-- Cash Shop coupons (report b072): a code, typed in the shop's Coupon
-- dialog, that one account may redeem once for what its rewards list.
-- Codes are kept in upper case; the server upper-cases what is typed.
CREATE TABLE coupons (
    id          SERIAL      PRIMARY KEY,
    code        VARCHAR(32) NOT NULL UNIQUE CHECK (code <> '' AND code = upper(code)),
    expires_at  TIMESTAMP,
    redeemed_by INTEGER     REFERENCES accounts(id) ON DELETE SET NULL,
    redeemed_at TIMESTAMP
);

-- What a coupon gives. Kind 0 is an item (cash items go to the Cash
-- Inventory, others to the bags), 1 NX Credit, 2 Maple Points, 3 NX
-- Prepaid, 4 mesos; item_id is 0 for all but items.
CREATE TABLE coupon_rewards (
    id        SERIAL   PRIMARY KEY,
    coupon_id INTEGER  NOT NULL REFERENCES coupons(id) ON DELETE CASCADE,
    kind      SMALLINT NOT NULL CHECK (kind BETWEEN 0 AND 4),
    item_id   INTEGER  NOT NULL DEFAULT 0,
    quantity  INTEGER  NOT NULL CHECK (quantity > 0)
);
CREATE INDEX coupon_rewards_coupon ON coupon_rewards (coupon_id);
