-- The mesos a character of level 15 or below has sent by Duey today: the
-- client's "1 million mesos per day" for low levels. A row only once they
-- have sent any; `day` is the day it counts.
CREATE TABLE parcel_mesos_sent (
    character_id    INTEGER     PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    day             DATE        NOT NULL,
    mesos           INTEGER     NOT NULL DEFAULT 0
);
