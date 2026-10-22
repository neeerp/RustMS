-- How often each character used each family entitlement on `used_on`; a
-- row from another day counts as none.
CREATE TABLE family_entitlement_uses (
    character_id    INTEGER     NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    entitlement     SMALLINT    NOT NULL,
    used_on         DATE        NOT NULL DEFAULT CURRENT_DATE,
    times           INTEGER     NOT NULL DEFAULT 0,
    PRIMARY KEY (character_id, entitlement)
);
