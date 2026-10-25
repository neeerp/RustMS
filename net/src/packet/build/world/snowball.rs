//! The Snowball field (field type 1, report b168): its two balls and two
//! snowmen, the hits on them and the field's messages.

use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// A ball as SNOWBALL_STATE gives it: the step it is at and its stage,
/// the index of the client's speed (8, no speed, only catches up).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BallState {
    pub position: i16,
    pub stage: u8,
}

/// SNOWBALL_STATE (0x119, v83 0x5750a3): the field's `state` (1 the game
/// runs, 2 ball 0 fades out, 3 ball 1 fades out), each snowman's HP in
/// percent and each ball. The first a client is sent on the field must
/// carry the damage it shows for a hit on a ball, on a snowman, and a
/// strong one on a snowman: it reads them only then.
pub fn build_snowball_state(
    state: u8,
    snowmen: [i32; 2],
    balls: [BallState; 2],
    damages: Option<[u16; 3]>,
) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::SnowballState as i16)?;
    p.write_byte(state)?;
    for hp in snowmen {
        p.write_int(hp)?;
    }
    for ball in balls {
        p.write_short(ball.position)?;
        p.write_byte(ball.stage)?;
    }
    for damage in damages.into_iter().flatten() {
        p.write_short(damage as i16)?;
    }
    Ok(p)
}

/// HIT_SNOWBALL (0x11A): `what` (ball 0, ball 1, snowman 0, snowman 1)
/// shows `damage`, or a miss for 0, after `delay` ms.
pub fn build_hit_snowball(what: u8, damage: u16, delay: u16) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::HitSnowball as i16)?;
    p.write_byte(what)?;
    p.write_short(damage as i16)?;
    p.write_short(delay as i16)?;
    Ok(p)
}

/// SNOWBALL_MESSAGE (0x11B): the client's line `message` about `team`'s
/// ball: 1 to 3 it passed that stage, 4 the other team is attacking the
/// snowman and stopping it, 5 it is moving again.
pub fn build_snowball_message(team: u8, message: u8) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::SnowballMessage as i16)?;
    p.write_byte(team)?;
    p.write_byte(message)?;
    Ok(p)
}

/// LEFT_KNOCK_BACK (0x11C): the ball the player walked into throws them
/// back, left and up.
pub fn build_left_knock_back() -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::LeftKnockBack as i16)?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::testing::Expect;

    #[test]
    fn the_first_state_carries_the_damages() {
        let balls = [
            BallState {
                position: 45,
                stage: 8,
            },
            BallState {
                position: -2,
                stage: 0,
            },
        ];
        let packet = build_snowball_state(1, [100, 40], balls, Some([10, 15, 45])).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x119).byte(1).int(100).int(40);
        expect.short(45).byte(8).short(-2).byte(0);
        expect.short(10).short(15).short(45);
        expect.end();
        let packet = build_snowball_state(3, [0, 100], balls, None).unwrap();
        assert_eq!(packet.bytes.len(), 2 + 15);
    }

    #[test]
    fn hits_messages_and_the_knock_back() {
        let packet = build_hit_snowball(2, 45, 600).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x11A).byte(2).short(45).short(600);
        expect.end();
        let packet = build_snowball_message(1, 4).unwrap();
        let mut expect = Expect::new(&packet.bytes);
        expect.short(0x11B).byte(1).byte(4);
        expect.end();
        assert_eq!(build_left_knock_back().unwrap().bytes, [0x1C, 0x01]);
    }
}
