use crate::{error::NetworkError, packet::op::SendOpcode};
use packet::{io::write::PktWrite, Packet};

/// What GIVE_FAME came to, as FAME_RESPONSE tells it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FameResponse {
    /// To the giver: `target`'s fame went up (or down) to `fame`.
    Gave { target: String, up: bool, fame: i16 },
    /// "The user name is incorrectly entered."
    NotFound,
    /// "Users under level 15 are unable to toggle with fame."
    UnderLevel,
    /// "You can't raise or drop a level of fame anymore for today."
    NotToday,
    /// "...of that character anymore for this month."
    NotThisMonth,
    /// To the target: `giver` raised (or dropped) their fame.
    Received { giver: String, up: bool },
    /// "The level of fame has neither been raised or dropped due to an
    /// unexpected error."
    Failed,
}

/// FAME_RESPONSE (0x26, v83 FUN_00a223dc). The giver's chat names the
/// target; their open info window shows the new fame.
pub fn build_fame_response(response: &FameResponse) -> Result<Packet, NetworkError> {
    let mut packet = Packet::new_empty();
    packet.write_short(SendOpcode::FameResponse as i16)?;
    let mode = match response {
        FameResponse::Gave { .. } => 0,
        FameResponse::NotFound => 1,
        FameResponse::UnderLevel => 2,
        FameResponse::NotToday => 3,
        FameResponse::NotThisMonth => 4,
        FameResponse::Received { .. } => 5,
        FameResponse::Failed => 6,
    };
    packet.write_byte(mode)?;
    match response {
        FameResponse::Gave { target, up, fame } => {
            packet.write_str_with_length(target)?;
            packet.write_byte(u8::from(*up))?;
            packet.write_int(i32::from(*fame))?;
        }
        FameResponse::Received { giver, up } => {
            packet.write_str_with_length(giver)?;
            packet.write_byte(u8::from(*up))?;
        }
        _ => {}
    }
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_giver_hears_the_new_fame_and_the_target_who_gave_it() {
        let gave = FameResponse::Gave {
            target: "Bob".into(),
            up: false,
            fame: -3,
        };
        assert_eq!(
            build_fame_response(&gave).unwrap().bytes,
            [0x26, 0, 0, 3, 0, b'B', b'o', b'b', 0, 0xfd, 0xff, 0xff, 0xff]
        );
        let received = FameResponse::Received {
            giver: "Al".into(),
            up: true,
        };
        assert_eq!(
            build_fame_response(&received).unwrap().bytes,
            [0x26, 0, 5, 2, 0, b'A', b'l', 1]
        );
        assert_eq!(
            build_fame_response(&FameResponse::NotThisMonth)
                .unwrap()
                .bytes,
            [0x26, 0, 4]
        );
    }
}
