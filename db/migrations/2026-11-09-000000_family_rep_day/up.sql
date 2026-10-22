-- The day a row's Rep counters for today (earned, and given to the senior)
-- were last written. They count from 0 again on the next day.
ALTER TABLE family_members ADD COLUMN rep_day DATE NOT NULL DEFAULT CURRENT_DATE;
