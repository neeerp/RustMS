use crate::handler::{
    HandlerResult, PartyRequest, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build::world::party::MULTICHAT_PARTY;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// `PARTY_OPERATION`: create, leave, accept, invite, expel, or pass the lead.
/// The world directory owns parties, so every operation goes there.
pub struct PartyOperationHandler;

impl WorldHandler for PartyOperationHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let request = match reader.read_byte()? {
            1 => PartyRequest::Create,
            2 => PartyRequest::Leave,
            3 => PartyRequest::Accept {
                party_id: reader.read_int()?,
            },
            4 => PartyRequest::Invite {
                name: reader.read_str_with_length()?,
            },
            5 => PartyRequest::Expel {
                member_id: reader.read_int()?,
            },
            6 => PartyRequest::ChangeLeader {
                member_id: reader.read_int()?,
            },
            _ => return Ok(HandlerResult::empty()),
        };
        Ok(HandlerResult::empty().with(WorldAction::Party(request)))
    }
}

/// `DENY_PARTY_REQUEST`: the receiver turned down an invitation.
pub struct DenyPartyRequestHandler;

impl WorldHandler for DenyPartyRequestHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        reader.read_byte()?;
        let inviter = reader.read_str_with_length()?;
        // Party search invitations name the inviter as "PS: name".
        let inviter = inviter.strip_prefix("PS: ").unwrap_or(&inviter).to_string();
        Ok(HandlerResult::empty().with(WorldAction::Party(PartyRequest::Decline { inviter })))
    }
}

/// `MULTI_CHAT`: a line for the party (buddy and guild chat are not
/// implemented). The client lists recipients, but the party decides who
/// hears it.
pub struct PartyChatHandler;

impl WorldHandler for PartyChatHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let mode = reader.read_byte()?;
        let recipients = reader.read_byte()?;
        for _ in 0..recipients {
            reader.read_int()?;
        }
        let text = reader.read_str_with_length()?;
        if mode != MULTICHAT_PARTY || text.is_empty() {
            return Ok(HandlerResult::empty());
        }
        Ok(HandlerResult::empty().with(WorldAction::Party(PartyRequest::Chat { text })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn request(handler: &dyn WorldHandler, bytes: &[u8]) -> Option<PartyRequest> {
        let mut packet = Packet::new(bytes);
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let mut actions = handler.handle(&mut packet, &mut ctx).unwrap().actions;
        match actions.pop() {
            Some(WorldAction::Party(request)) => Some(request),
            None => None,
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn reads_party_operations() {
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::PartyOperation as i16)
            .unwrap();
        packet.write_byte(4).unwrap();
        packet.write_str_with_length("Bobby").unwrap();
        assert_eq!(
            request(&PartyOperationHandler, &packet.bytes),
            Some(PartyRequest::Invite {
                name: "Bobby".into()
            })
        );

        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::PartyOperation as i16)
            .unwrap();
        packet.write_byte(3).unwrap();
        packet.write_int(9).unwrap();
        assert_eq!(
            request(&PartyOperationHandler, &packet.bytes),
            Some(PartyRequest::Accept { party_id: 9 })
        );
    }

    #[test]
    fn decline_drops_the_party_search_prefix() {
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::DenyPartyRequest as i16)
            .unwrap();
        packet.write_byte(0x17).unwrap();
        packet.write_str_with_length("PS: Alice").unwrap();
        assert_eq!(
            request(&DenyPartyRequestHandler, &packet.bytes),
            Some(PartyRequest::Decline {
                inviter: "Alice".into()
            })
        );
    }

    #[test]
    fn only_party_chat_is_forwarded() {
        let chat = |mode: u8| {
            let mut packet = Packet::new_empty();
            packet.write_short(RecvOpcode::PartyChat as i16).unwrap();
            packet.write_byte(mode).unwrap();
            packet.write_byte(2).unwrap();
            packet.write_int(2).unwrap();
            packet.write_int(3).unwrap();
            packet.write_str_with_length("hello").unwrap();
            request(&PartyChatHandler, &packet.bytes)
        };
        assert_eq!(
            chat(1),
            Some(PartyRequest::Chat {
                text: "hello".into()
            })
        );
        assert_eq!(chat(0), None);
    }
}
