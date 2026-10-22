-- The family rate entitlement working for each character, until `until`.
-- A party's entitlement reaches members outside any family too.
CREATE TABLE family_buffs (
    character_id    INTEGER     PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    entitlement     SMALLINT    NOT NULL,
    until           TIMESTAMP   NOT NULL
);
