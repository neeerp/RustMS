-- The drops a pet leaves where they lie, by the cash serial of its item;
-- 2147483647 stands for mesos.
CREATE TABLE pet_ignores (
    serial          BIGINT          NOT NULL REFERENCES pets(serial) ON DELETE CASCADE,
    item_id         INTEGER         NOT NULL,
    PRIMARY KEY (serial, item_id)
);
