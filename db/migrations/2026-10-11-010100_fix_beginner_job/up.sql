-- Job 1 is not a valid MapleStory job. It was an Explorer creation flag
-- incorrectly persisted as the job ID by the original character handler.
UPDATE characters SET job = 0 WHERE job = 1;
