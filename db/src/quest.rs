use crate::{
    character::Character,
    player::write_player,
    schema::{character_quests, characters},
};
use diesel::prelude::*;
use game::{Player, QuestState, QuestStatus};
use std::time::SystemTime;

#[derive(Clone, Debug, Queryable, Identifiable)]
#[diesel(table_name = character_quests, primary_key(character_id, quest_id))]
pub struct QuestProgress {
    pub character_id: i32,
    pub quest_id: i16,
    pub status: i16,
    pub completed_at: Option<SystemTime>,
    /// Mob kill counts, three digits per required mob (v83 format), or the
    /// info string of a quest in `game::quest::INFO_QUESTS`.
    pub progress: String,
}

pub fn get_quests(cid: i32) -> crate::Result<Vec<QuestProgress>> {
    Ok(character_quests::table
        .filter(character_quests::character_id.eq(cid))
        .order(character_quests::quest_id)
        .load(&mut crate::connection()?)?)
}

fn rejected() -> diesel::result::Error {
    diesel::result::Error::NotFound
}

/// Start a quest and save `player` (already updated with whatever starting
/// it changed, e.g. a given item, and holding the started quest) in one
/// transaction. `prerequisites` are
/// `(quest id, required state)` pairs, rechecked under the character lock. A
/// quest can only be started once, unless it is `repeatable` and was
/// completed, when its record starts over.
pub fn start(
    player: &Player,
    qid: i16,
    prerequisites: &[(i16, QuestState)],
    repeatable: bool,
) -> crate::Result<QuestProgress> {
    begin(player, qid, prerequisites, repeatable, false)
}

/// Start a quest and complete it at once, as a start script that completes
/// its quest does (the informants of quests 2216-2219): the checks of
/// [`start`], then the record is saved completed.
pub fn finish(
    player: &Player,
    qid: i16,
    prerequisites: &[(i16, QuestState)],
    repeatable: bool,
) -> crate::Result<QuestProgress> {
    begin(player, qid, prerequisites, repeatable, true)
}

fn begin(
    player: &Player,
    qid: i16,
    prerequisites: &[(i16, QuestState)],
    repeatable: bool,
    finished: bool,
) -> crate::Result<QuestProgress> {
    let (status, completed_at) = if finished {
        (2, Some(SystemTime::now()))
    } else {
        (1, None)
    };
    let cid = player.id();
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
        let current = character_quests::table
            .find((cid, qid))
            .select(character_quests::status)
            .first::<i16>(conn)
            .optional()?;
        match current {
            None => {}
            Some(2) if repeatable => {}
            Some(_) => return Err(rejected()),
        }
        let progress = (player.quests.status(qid))
            .and_then(QuestStatus::record_string)
            .unwrap_or_default();
        let progress = diesel::insert_into(character_quests::table)
            .values((
                character_quests::character_id.eq(cid),
                character_quests::quest_id.eq(qid),
                character_quests::status.eq(status),
                character_quests::completed_at.eq(completed_at),
                character_quests::progress.eq(&progress),
            ))
            .on_conflict((character_quests::character_id, character_quests::quest_id))
            .do_update()
            .set((
                character_quests::status.eq(status),
                character_quests::completed_at.eq(completed_at),
                character_quests::progress.eq(&progress),
            ))
            .get_result(conn)?;
        write_player(conn, player)?;
        Ok(progress)
    })
    .map_err(Into::into)
}

/// Complete a started quest and save `player` (already holding its rewards)
/// in one transaction, so a replayed completion can never pay twice.
pub fn complete(player: &Player, qid: i16) -> crate::Result<QuestProgress> {
    let cid = player.id();
    let mut conn = crate::connection()?;
    conn.transaction(|conn| -> diesel::QueryResult<_> {
        characters::table
            .find(cid)
            .for_update()
            .first::<Character>(conn)?;
        let current = character_quests::table
            .find((cid, qid))
            .first::<QuestProgress>(conn)
            .optional()?;
        if current.map(|p| p.status) != Some(1) {
            return Err(rejected());
        }
        let progress = diesel::update(character_quests::table.find((cid, qid)))
            .set((
                character_quests::status.eq(2),
                character_quests::completed_at.eq(Some(SystemTime::now())),
            ))
            .get_result(conn)?;
        write_player(conn, player)?;
        Ok(progress)
    })
    .map_err(Into::into)
}

/// Give up a started quest, with the `records` it reads that are no quest
/// (`game::quest::RECORDS`). Completed quests cannot be forfeited.
pub fn forfeit(cid: i32, qid: i16, records: &[i16]) -> crate::Result<()> {
    let mut conn = crate::connection()?;
    conn.transaction(|conn| -> diesel::QueryResult<()> {
        let deleted = diesel::delete(
            character_quests::table
                .find((cid, qid))
                .filter(character_quests::status.eq(1)),
        )
        .execute(conn)?;
        if deleted == 0 {
            return Err(rejected());
        }
        diesel::delete(
            character_quests::table
                .filter(character_quests::character_id.eq(cid))
                .filter(character_quests::quest_id.eq_any(records)),
        )
        .execute(conn)?;
        Ok(())
    })
    .map_err(Into::into)
}
