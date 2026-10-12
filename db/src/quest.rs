use crate::{
    character::Character,
    schema::{character_quests, characters},
};
use diesel::prelude::*;
use game::QuestState;
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

/// The beginner EXP table (levels 1–9) from the client, enough for the quest
/// rewards reachable today. Full leveling lives with combat.
const BEGINNER_EXP: [i32; 10] = [0, 15, 34, 57, 92, 135, 372, 560, 840, 1242];

fn rejected() -> diesel::result::Error {
    diesel::result::Error::NotFound
}

/// Start a quest. `prerequisites` are `(quest id, required state)` pairs,
/// rechecked under the character lock; a quest can only be started once.
pub fn start(
    cid: i32,
    qid: i16,
    prerequisites: &[(i16, QuestState)],
) -> crate::Result<QuestProgress> {
    let mut conn = crate::connection()?;
    conn.transaction(|conn| -> diesel::QueryResult<_> {
        // Serialize every quest change for a character on its row.
        characters::table
            .find(cid)
            .for_update()
            .first::<Character>(conn)?;
        for &(required, state) in prerequisites {
            let current = character_quests::table
                .find((cid, required))
                .select(character_quests::status)
                .first::<i16>(conn)
                .optional()?
                .unwrap_or(0);
            if current != state as i16 {
                return Err(rejected());
            }
        }
        if character_quests::table
            .find((cid, qid))
            .first::<QuestProgress>(conn)
            .optional()?
            .is_some()
        {
            return Err(rejected());
        }
        diesel::insert_into(character_quests::table)
            .values((
                character_quests::character_id.eq(cid),
                character_quests::quest_id.eq(qid),
                character_quests::status.eq(1),
            ))
            .get_result(conn)
    })
    .map_err(Into::into)
}

/// Complete a started quest and grant its EXP in the same transaction, so a
/// replayed completion can never pay twice.
pub fn complete(cid: i32, qid: i16, exp: i32) -> crate::Result<(Character, QuestProgress)> {
    let mut conn = crate::connection()?;
    conn.transaction(|conn| -> diesel::QueryResult<_> {
        let mut character: Character = characters::table.find(cid).for_update().first(conn)?;
        let current = character_quests::table
            .find((cid, qid))
            .first::<QuestProgress>(conn)
            .optional()?;
        if current.map(|p| p.status) != Some(1) || exp < 0 {
            return Err(rejected());
        }
        if exp > 0 {
            character.exp = character
                .exp
                .checked_add(exp)
                .ok_or(diesel::result::Error::RollbackTransaction)?;
            while character.level > 0
                && character.level < 10
                && character.exp >= BEGINNER_EXP[character.level as usize]
            {
                character.exp -= BEGINNER_EXP[character.level as usize];
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
        }
        let progress = diesel::update(character_quests::table.find((cid, qid)))
            .set((
                character_quests::status.eq(2),
                character_quests::completed_at.eq(Some(SystemTime::now())),
            ))
            .get_result(conn)?;
        Ok((character, progress))
    })
    .map_err(Into::into)
}

/// Give up a started quest. Completed quests cannot be forfeited.
pub fn forfeit(cid: i32, qid: i16) -> crate::Result<()> {
    let mut conn = crate::connection()?;
    let deleted = diesel::delete(
        character_quests::table
            .find((cid, qid))
            .filter(character_quests::status.eq(1)),
    )
    .execute(&mut conn)?;
    if deleted == 0 {
        return Err(rejected().into());
    }
    Ok(())
}
