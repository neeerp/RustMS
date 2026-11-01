//! The Monster Carnival's packets (`net::carnival`; v83ref b163, the
//! client's `CField_MonsterCarnival::OnPacket`, FUN_005652ee).

use crate::carnival::{Points, SpendRefusal, Start};
use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// The client reads the points as 16-bit numbers.
fn points(p: &mut Packet, points: Points) -> Result<(), NetworkError> {
    let short = |n: i32| n.clamp(0, i32::from(u16::MAX)) as u16 as i16;
    p.write_short(short(points.cp))?;
    p.write_short(short(points.total))?;
    Ok(())
}

/// MONSTER_CARNIVAL_START: the player's side and the CP of the player and
/// both sides, then for each mob the field lets a side summon, how many
/// are out (`summoned`). Cosmic writes a short and a long of zeros there
/// instead.
pub fn build_carnival_start(start: Start, summoned: &[u8]) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::CarnivalStart as i16)?;
    p.write_byte(start.side)?;
    points(&mut p, start.mine)?;
    points(&mut p, start.ours)?;
    points(&mut p, start.theirs)?;
    for &count in summoned {
        p.write_byte(count)?;
    }
    Ok(p)
}

/// MONSTER_CARNIVAL_OBTAINED_CP: the player's own CP.
pub fn build_obtained_cp(mine: Points) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::CarnivalObtainedCp as i16)?;
    points(&mut p, mine)?;
    Ok(p)
}

/// MONSTER_CARNIVAL_PARTY_CP: side `side`'s CP.
pub fn build_party_cp(side: u8, team: Points) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::CarnivalPartyCp as i16)?;
    p.write_byte(side)?;
    points(&mut p, team)?;
    Ok(p)
}

/// MONSTER_CARNIVAL_SUMMON: `name` spent CP on `index` of the window's
/// tab `tab` (0 mobs, 1 skills, 2 guardians), which the client's chat log
/// shows. It frees the window for the next request.
pub fn build_carnival_summon(tab: u8, index: u8, name: &str) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::CarnivalSummon as i16)?;
    p.write_byte(tab)?;
    p.write_byte(index)?;
    p.write_str_with_length(name)?;
    Ok(p)
}

/// MONSTER_CARNIVAL_MESSAGE: why the player's spend failed. It frees the
/// window for the next request.
pub fn build_carnival_message(why: SpendRefusal) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::CarnivalMessage as i16)?;
    p.write_byte(why as u8)?;
    Ok(p)
}

/// MONSTER_CARNIVAL_DIED: `name` of side `side` died, and their side lost
/// `lost` CP, which the client's chat log shows.
pub fn build_carnival_died(side: u8, name: &str, lost: u8) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::CarnivalDied as i16)?;
    p.write_byte(side)?;
    p.write_str_with_length(name)?;
    p.write_byte(lost)?;
    Ok(p)
}

/// MONSTER_CARNIVAL_LEAVE: `name` of side `side` quit the battle
/// (`crate::carnival::LEAVE_QUIT`), or leads it now that its leader did
/// (`LEAVE_LEADER`), which the battle field's chat log shows.
pub fn build_carnival_leave(kind: u8, side: u8, name: &str) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::CarnivalLeave as i16)?;
    p.write_byte(kind)?;
    p.write_byte(side)?;
    p.write_str_with_length(name)?;
    Ok(p)
}

/// MONSTER_CARNIVAL_RESULT: how the battle ended for the player's side
/// (`crate::carnival::RESULT_WON` to `RESULT_LEFT`), which the battle field
/// and the revive room print in the chat log.
pub fn build_carnival_result(code: u8) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::CarnivalResult as i16)?;
    p.write_byte(code)?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    #[test]
    fn the_start_carries_every_score_and_the_summoned_counts() {
        let start = Start {
            side: 1,
            mine: Points { cp: 2, total: 5 },
            ours: Points { cp: 7, total: 9 },
            theirs: Points { cp: 3, total: 4 },
        };
        let packet = build_carnival_start(start, &[0, 0, 1]).unwrap();
        Expect::new(&packet.bytes)
            .short(0x121)
            .byte(1)
            .short(2)
            .short(5)
            .short(7)
            .short(9)
            .short(3)
            .short(4)
            .byte(0)
            .byte(0)
            .byte(1)
            .end();
    }

    #[test]
    fn cp_updates_carry_the_points() {
        let mine = Points {
            cp: 1,
            total: 70000,
        };
        Expect::new(&build_obtained_cp(mine).unwrap().bytes)
            .short(0x122)
            .short(1)
            .short(-1)
            .end();
        Expect::new(&build_party_cp(0, Points { cp: 4, total: 6 }).unwrap().bytes)
            .short(0x123)
            .byte(0)
            .short(4)
            .short(6)
            .end();
    }

    #[test]
    fn spending_is_answered_with_a_summon_or_a_message() {
        Expect::new(&build_carnival_summon(0, 3, "Red").unwrap().bytes)
            .short(0x124)
            .byte(0)
            .byte(3)
            .string("Red")
            .end();
        let limit = build_carnival_message(SpendRefusal::Limit).unwrap();
        Expect::new(&limit.bytes).short(0x125).byte(2).end();
    }

    #[test]
    fn a_death_names_the_dead_and_the_cp_lost() {
        Expect::new(&build_carnival_died(1, "Blue", 10).unwrap().bytes)
            .short(0x126)
            .byte(1)
            .string("Blue")
            .byte(10)
            .end();
        Expect::new(&build_carnival_leave(6, 0, "Red").unwrap().bytes)
            .short(0x127)
            .byte(6)
            .byte(0)
            .string("Red")
            .end();
        Expect::new(&build_carnival_result(8).unwrap().bytes)
            .short(0x128)
            .byte(8)
            .end();
    }
}
