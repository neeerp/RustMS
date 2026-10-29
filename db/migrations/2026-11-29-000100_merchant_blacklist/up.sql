-- The names a hired merchant keeps out, as its owner's client sent them
-- when it opened and as the owner changed them in maintenance. They are
-- read again with the merchant when its field starts.
ALTER TABLE hired_merchants ADD COLUMN blacklist TEXT[] NOT NULL DEFAULT '{}';
