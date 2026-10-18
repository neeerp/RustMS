use crate::handler::{
    GuildRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// `GUILD_OPERATION`: ask for the guild again, or name a new guild in
/// answer to the name prompt. Other modes aren't supported yet.
pub struct GuildOperationHandler;

impl WorldHandler for GuildOperationHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let request = match reader.read_byte()? {
            0x00 => GuildRequest::Reload,
            0x02 => GuildRequest::Create {
                name: reader.read_str_with_length()?,
            },
            _ => return Ok(HandlerResult::empty()),
        };
        Ok(HandlerResult::empty().with(WorldAction::Guild(request)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn request(bytes: &[u8]) -> Option<GuildRequest> {
        let mut packet = Packet::new(bytes);
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let result = GuildOperationHandler.handle(&mut packet, &mut ctx);
        match result.unwrap().actions.pop() {
            Some(WorldAction::Guild(request)) => Some(request),
            None => None,
            other => panic!("unexpected {other:?}"),
        }
    }

    fn operation(mode: u8) -> Packet {
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::GuildOperation as i16)
            .unwrap();
        packet.write_byte(mode).unwrap();
        packet
    }

    #[test]
    fn reads_reload_and_create() {
        assert_eq!(request(&operation(0).bytes), Some(GuildRequest::Reload));

        let mut packet = operation(2);
        packet.write_str_with_length("Testers").unwrap();
        assert_eq!(
            request(&packet.bytes),
            Some(GuildRequest::Create {
                name: "Testers".into()
            })
        );

        // Invitations and the rest come later.
        let mut packet = operation(5);
        packet.write_str_with_length("Bobby").unwrap();
        assert_eq!(request(&packet.bytes), None);
    }
}
