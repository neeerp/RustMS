//! A character's records at the mini games, Omok (1) and match cards (2).

use crate::schema::minigame_records;
use diesel::prelude::*;
use game::{GameOutcome, GameRecord};

/// Both records, Omok's first; none played is all zeros.
pub fn records(character_id: i32) -> crate::Result<[GameRecord; 2]> {
    let mut conn = crate::connection()?;
    let rows: Vec<(i16, i32, i32, i32)> = minigame_records::table
        .filter(minigame_records::character_id.eq(character_id))
        .select((
            minigame_records::game,
            minigame_records::wins,
            minigame_records::ties,
            minigame_records::losses,
        ))
        .load(&mut conn)?;
    let mut records = [GameRecord::default(); 2];
    for (game, wins, ties, losses) in rows {
        if let Some(record) = records.get_mut((game - 1) as usize) {
            *record = GameRecord { wins, ties, losses };
        }
    }
    Ok(records)
}

/// Count one game of `game` that ended in `outcome`.
pub fn add(character_id: i32, game: u8, outcome: GameOutcome) -> crate::Result<()> {
    let mut conn = crate::connection()?;
    let (wins, ties, losses) = match outcome {
        GameOutcome::Win => (1, 0, 0),
        GameOutcome::Tie => (0, 1, 0),
        GameOutcome::Loss => (0, 0, 1),
    };
    diesel::insert_into(minigame_records::table)
        .values((
            minigame_records::character_id.eq(character_id),
            minigame_records::game.eq(i16::from(game)),
            minigame_records::wins.eq(wins),
            minigame_records::ties.eq(ties),
            minigame_records::losses.eq(losses),
        ))
        .on_conflict((minigame_records::character_id, minigame_records::game))
        .do_update()
        .set((
            minigame_records::wins.eq(minigame_records::wins + wins),
            minigame_records::ties.eq(minigame_records::ties + ties),
            minigame_records::losses.eq(minigame_records::losses + losses),
        ))
        .execute(&mut conn)?;
    Ok(())
}
