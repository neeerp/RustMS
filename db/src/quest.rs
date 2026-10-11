use crate::{
    character::Character,
    schema::{character_quests, characters},
};
use diesel::prelude::*;
use std::time::SystemTime;

#[derive(Clone, Debug, Queryable, Identifiable)]
#[diesel(table_name = character_quests, primary_key(character_id, quest_id))]
pub struct QuestProgress {
    pub character_id: i32,
    pub quest_id: i16,
    pub status: i16,
    pub completed_at: Option<SystemTime>,
}

pub fn get_quests(cid: i32) -> crate::Result<Vec<QuestProgress>> {
    Ok(character_quests::table
        .filter(character_quests::character_id.eq(cid))
        .order(character_quests::quest_id)
        .load(&mut crate::connection()?)?)
}

/// Serialize quest transitions and rewards with the character row. Completion
/// and EXP are committed together, so retries cannot award the reward twice.
pub fn transition(
    cid: i32,
    qid: i16,
    job: i16,
    prerequisite: Option<i16>,
    max_start_level: Option<i16>,
    reward: Option<i32>,
) -> crate::Result<(Character, QuestProgress)> {
    let mut conn = crate::connection()?;
    conn.transaction(|conn| -> diesel::QueryResult<_> {
        let mut character: Character = characters::table.find(cid).for_update().first(conn)?;
        if character.job != job {
            return Err(diesel::result::Error::NotFound);
        }
        if reward.is_none() && max_start_level.is_some_and(|max| character.level > max) {
            return Err(diesel::result::Error::NotFound);
        }
        if let Some(required) = prerequisite {
            let completed = character_quests::table
                .find((cid, required))
                .filter(character_quests::status.eq(2))
                .first::<QuestProgress>(conn)
                .optional()?;
            if completed.is_none() {
                return Err(diesel::result::Error::NotFound);
            }
        }
        let current = character_quests::table
            .find((cid, qid))
            .first::<QuestProgress>(conn)
            .optional()?;
        if let Some(gain) = reward {
            if current.as_ref().map(|p| p.status) != Some(1) || gain < 0 {
                return Err(diesel::result::Error::NotFound);
            }
            character.exp = character
                .exp
                .checked_add(gain)
                .ok_or(diesel::result::Error::RollbackTransaction)?;
            // The beginner levels reached by Rain's 62 EXP, using the client table.
            const EXP: [i32; 10] = [0, 15, 34, 57, 92, 135, 372, 560, 840, 1242];
            while character.level > 0
                && character.level < 10
                && character.exp >= EXP[character.level as usize]
            {
                character.exp -= EXP[character.level as usize];
                character.level += 1;
                character.ap = character
                    .ap
                    .checked_add(5)
                    .ok_or(diesel::result::Error::RollbackTransaction)?;
            }
            diesel::update(characters::table.find(cid))
                .set((
                    characters::exp.eq(character.exp),
                    characters::level.eq(character.level),
                    characters::ap.eq(character.ap),
                ))
                .execute(conn)?;
            let progress = diesel::update(character_quests::table.find((cid, qid)))
                .set((
                    character_quests::status.eq(2),
                    character_quests::completed_at.eq(Some(SystemTime::now())),
                ))
                .get_result(conn)?;
            Ok((character, progress))
        } else {
            if let Some(progress) = current {
                if progress.status != 1 {
                    return Err(diesel::result::Error::NotFound);
                }
                return Ok((character, progress));
            }
            let progress = diesel::insert_into(character_quests::table)
                .values((
                    character_quests::character_id.eq(cid),
                    character_quests::quest_id.eq(qid),
                    character_quests::status.eq(1),
                ))
                .get_result(conn)?;
            Ok((character, progress))
        }
    })
    .map_err(Into::into)
}
