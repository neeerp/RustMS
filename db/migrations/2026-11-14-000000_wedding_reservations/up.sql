-- Booked weddings (report b066): one per couple, on the channel it was
-- booked on, starting on the hour. Venue 0 is the Chapel, 1 the Cathedral.
-- The couple's record keeps its status; the client sends invitations only
-- while it says engaged.
CREATE TABLE wedding_reservations (
    marriage_id INTEGER     PRIMARY KEY REFERENCES marriages(id) ON DELETE CASCADE,
    venue       SMALLINT    NOT NULL CHECK (venue IN (0, 1)),
    premium     BOOLEAN     NOT NULL,
    channel     SMALLINT    NOT NULL,
    starts_at   TIMESTAMP   NOT NULL,
    UNIQUE (channel, venue, starts_at)
);

-- The guests a couple invited to their wedding.
CREATE TABLE wedding_guests (
    marriage_id INTEGER     NOT NULL REFERENCES wedding_reservations(marriage_id) ON DELETE CASCADE,
    guest_id    INTEGER     NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    PRIMARY KEY (marriage_id, guest_id)
);
