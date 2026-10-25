//! The Monster Carnival's packets (`net::carnival`; v83ref b163, the
//! client's `CField_MonsterCarnival::OnPacket`, FUN_005652ee).

use crate::carnival::{Points, Start};
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
}
