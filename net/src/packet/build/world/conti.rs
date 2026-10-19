//! Continent ships, the station wall clock (`net::conti`) and the field
//! countdown.

use crate::conti::{State, Target};
use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// CONTI_MOVE: a ship leaves or docks on a route's map (v83 FUN_0054dca3).
pub fn build_conti_move(target: Target, state: State) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ContiMove as i16)?;
    p.write_byte(target as u8)?;
    p.write_byte(state as u8)?;
    Ok(p)
}

/// CONTI_STATE: where a trip is, for a client that has just entered one of
/// its maps (v83 FUN_0054dd08). The second byte says whether the Crimson
/// Balrog's ship is out, which only a deck reads.
pub fn build_conti_state(state: State) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::ContiState as i16)?;
    p.write_byte(state as u8)?;
    p.write_byte(0)?;
    Ok(p)
}

/// CLOCK kind 1: sets a map's wall clock to this time of day (v83
/// FUN_005361bd).
pub fn build_clock_time(hour: u8, minute: u8, second: u8) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::Clock as i16)?;
    p.write_byte(1)?;
    p.write_byte(hour)?;
    p.write_byte(minute)?;
    p.write_byte(second)?;
    Ok(p)
}

/// CLOCK kind 2: the field's countdown from this many seconds, started
/// (v83 FUN_005361bd).
pub fn build_countdown(seconds: u32) -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::Clock as i16)?;
    p.write_byte(2)?;
    p.write_int(seconds as i32)?;
    Ok(p)
}

/// STOP_CLOCK: takes the field's countdown down (v83 FUN_0053184a).
pub fn build_stop_clock() -> Result<Packet, NetworkError> {
    let mut p = Packet::new_empty();
    p.write_short(SendOpcode::StopClock as i16)?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ship_leaving_its_platform_names_the_platform_then_the_change() {
        assert_eq!(
            build_conti_move(Target::Platform, State::Start)
                .unwrap()
                .bytes,
            [0x94, 0, 8, 2]
        );
        assert_eq!(
            build_conti_move(Target::ArrivalPlatform, State::End)
                .unwrap()
                .bytes,
            [0x94, 0, 12, 6]
        );
    }

    #[test]
    fn the_state_comes_without_the_balrog() {
        assert_eq!(
            build_conti_state(State::Wait).unwrap().bytes,
            [0x95, 0, 1, 0]
        );
    }

    #[test]
    fn the_wall_clock_is_kind_one_with_the_time_of_day() {
        assert_eq!(
            build_clock_time(14, 5, 9).unwrap().bytes,
            [0x93, 0, 1, 14, 5, 9]
        );
    }

    #[test]
    fn the_countdown_is_kind_two_with_its_seconds() {
        assert_eq!(
            build_countdown(7200).unwrap().bytes,
            [0x93, 0, 2, 0x20, 0x1c, 0, 0]
        );
        assert_eq!(build_stop_clock().unwrap().bytes, [0x9a, 0]);
    }
}
