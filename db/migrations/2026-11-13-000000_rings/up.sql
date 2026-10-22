-- Crush and friendship rings (report b061): two rows per pair, one for each
-- ring. `id` is the ring's cash serial, and `partner_ring_id` the other's.
CREATE TABLE rings (
    id              BIGINT      PRIMARY KEY,
    partner_ring_id BIGINT      NOT NULL,
    partner_id      INTEGER     NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    item_id         INTEGER     NOT NULL
);
