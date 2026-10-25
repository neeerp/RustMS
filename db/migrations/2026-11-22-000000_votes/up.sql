-- Votes for the server reported by a vote site's hook (RUSTMS_VOTE_HOOK_ADDR):
-- each gave the account `amount` NX Credit. The latest keeps the account
-- from voting again within the cooldown.
CREATE TABLE votes (
    id              SERIAL          PRIMARY KEY,
    account_id      INTEGER         NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    amount          INTEGER         NOT NULL,
    voted_at        TIMESTAMP       NOT NULL DEFAULT now()
);
CREATE INDEX votes_account ON votes (account_id, voted_at);
