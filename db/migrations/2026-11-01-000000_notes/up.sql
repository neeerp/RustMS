-- Notes (memos) waiting for their receiver, who sees each once: reading
-- them deletes them. A flag of 1 is a thank-you for a gift, which raises the
-- receiver's fame when read.
CREATE TABLE notes (
    id              SERIAL          PRIMARY KEY,
    receiver_id     INTEGER         NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    sender          VARCHAR(13)     NOT NULL,
    message         TEXT            NOT NULL,
    sent_at         TIMESTAMP       NOT NULL DEFAULT now(),
    flag            SMALLINT        NOT NULL DEFAULT 0
);
CREATE INDEX notes_receiver ON notes (receiver_id);

-- The gifts whose receivers have thanked their senders with a note.
CREATE TABLE gift_replies (
    cash_item_id    BIGINT          PRIMARY KEY REFERENCES cash_items(id) ON DELETE CASCADE
);
