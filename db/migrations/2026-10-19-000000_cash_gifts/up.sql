-- A gift's message, kept until the receiving account next enters the Cash
-- Shop and is shown the gift.
ALTER TABLE cash_items ADD COLUMN gift_message VARCHAR(73);
