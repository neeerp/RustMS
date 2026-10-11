use crate::error::NetworkError;
use crate::handler::{HandlerContext, HandlerResult, PacketHandler};
use crate::packet::build::login;
use db::character::NewCharacter;
use packet::{io::read::PktRead, Packet};
use std::io::BufReader;

pub struct CreateCharacterHandler;

impl CreateCharacterHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for CreateCharacterHandler {
    fn handle(
        &self,
        packet: &mut Packet,
        ctx: &mut HandlerContext,
    ) -> Result<HandlerResult, NetworkError> {
        let mut reader = BufReader::new(&**packet);
        reader.read_short()?;

        // Get account_id from session
        let accountid = ctx
            .session
            .session
            .as_ref()
            .map(|s| s.account_id)
            .ok_or(NetworkError::NotLoggedIn)?;

        let name = &reader.read_str_with_length()?;
        // The creation packet carries a class-family flag, not a job ID.
        let job = starting_job(reader.read_int()?)?;
        let face = reader.read_int()?;
        let hair = reader.read_int()?;
        let hair_color = reader.read_int()?;
        let skin = reader.read_int()?;
        let _top = reader.read_int()?; // Slot 5
        let _bot = reader.read_int()?; // Slot 6
        let _shoes = reader.read_int()?; // Slot 7
        let _weapon = reader.read_int()?; // Special
        let gender = reader.read_byte()? as i16;

        let world = ctx
            .session
            .session
            .as_ref()
            .and_then(|session| session.selected_world_id)
            .unwrap_or(0);

        let character = NewCharacter {
            accountid,
            world,
            name,
            job,
            face,
            hair,
            hair_color,
            skin,
            gender,
        };

        let char_packet = login::char::build_char_packet(character.create()?.character)?;
        Ok(HandlerResult::reply(char_packet))
    }
}

fn starting_job(class_family: i32) -> Result<i16, NetworkError> {
    match class_family {
        0 => Ok(1000), // Cygnus Noblesse
        1 => Ok(0),    // Explorer Beginner
        2 => Ok(2000), // Aran Legend
        _ => Err(NetworkError::PacketHandlerError(
            "Unsupported character class",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn creation_flags_are_converted_to_actual_starting_jobs() {
        assert_eq!(starting_job(1).unwrap(), 0);
        assert_eq!(starting_job(0).unwrap(), 1000);
        assert_eq!(starting_job(2).unwrap(), 2000);
        assert!(starting_job(-1).is_err());
        assert!(starting_job(100).is_err());
    }
}
