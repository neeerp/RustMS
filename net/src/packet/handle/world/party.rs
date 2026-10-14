use crate::handler::{
    BuddyRequest, HandlerResult, PartyRequest, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build::world::buddy::MULTICHAT_BUDDY;
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

/// `MULTI_CHAT`: a line for the party or for buddies (guild chat is not
/// implemented). The client lists recipients; the party decides who hears
/// party chat, and only listed mutual buddies hear buddy chat.
pub struct MultiChatHandler;

impl WorldHandler for MultiChatHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let mode = reader.read_byte()?;
        let count = reader.read_byte()?;
        let recipients = (0..count)
            .map(|_| reader.read_int())
            .collect::<Result<Vec<_>, _>>()?;
        let text = reader.read_str_with_length()?;
        let action = match mode {
            _ if text.is_empty() => return Ok(HandlerResult::empty()),
            MULTICHAT_PARTY => WorldAction::Party(PartyRequest::Chat { text }),
            MULTICHAT_BUDDY => WorldAction::Buddy(BuddyRequest::Chat { recipients, text }),
            _ => return Ok(HandlerResult::empty()),
        };
        Ok(HandlerResult::empty().with(action))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn action(handler: &dyn WorldHandler, bytes: &[u8]) -> Option<WorldAction> {
        let mut packet = Packet::new(bytes);
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        handler.handle(&mut packet, &mut ctx).unwrap().actions.pop()
    }

    fn request(handler: &dyn WorldHandler, bytes: &[u8]) -> Option<PartyRequest> {
        match action(handler, bytes) {
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
    fn party_and_buddy_chat_are_forwarded() {
        let chat = |mode: u8| {
            let mut packet = Packet::new_empty();
            packet.write_short(RecvOpcode::MultiChat as i16).unwrap();
            packet.write_byte(mode).unwrap();
            packet.write_byte(2).unwrap();
            packet.write_int(2).unwrap();
            packet.write_int(3).unwrap();
            packet.write_str_with_length("hello").unwrap();
            action(&MultiChatHandler, &packet.bytes)
        };
        assert!(matches!(
            chat(1),
            Some(WorldAction::Party(PartyRequest::Chat { text })) if text == "hello"
        ));
        assert!(matches!(
            chat(0),
            Some(WorldAction::Buddy(BuddyRequest::Chat { recipients, text }))
                if recipients == [2, 3] && text == "hello"
        ));
        assert!(chat(2).is_none());
    }
}
