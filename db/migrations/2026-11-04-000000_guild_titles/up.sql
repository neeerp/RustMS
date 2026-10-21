-- Every rank starts with a title, so members, who join at the lowest rank
-- with one, join at rank 5 and the client shows them a title.
ALTER TABLE guilds
    ALTER COLUMN title4 SET DEFAULT 'Member',
    ALTER COLUMN title5 SET DEFAULT 'Member';
UPDATE guilds SET title4 = 'Member' WHERE title4 = '';
UPDATE guilds SET title5 = 'Member' WHERE title5 = '';
