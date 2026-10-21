use crate::handler::{
    AllianceRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// `ALLIANCE_OPERATION`: ask for the alliance, leave it, invite a guild or
/// join on an invitation, expel a guild, hand the alliance over, or change
/// the rank titles, a member's rank, or the notice (report b023, section
/// 2). The client sends no op 5.
pub struct AllianceOperationHandler;

impl WorldHandler for AllianceOperationHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let request = match reader.read_byte()? {
            0x01 => AllianceRequest::Info,
            0x02 => AllianceRequest::Leave,
            0x03 => AllianceRequest::Invite {
                guild_name: reader.read_str_with_length()?,
            },
            // The player's guild's name follows.
            0x04 => AllianceRequest::Accept {
                alliance_id: reader.read_int()?,
            },
            // The alliance's id follows.
            0x06 => AllianceRequest::Expel {
                guild_id: reader.read_int()?,
            },
            0x07 => AllianceRequest::ChangeMaster {
                member_id: reader.read_int()?,
            },
            0x08 => AllianceRequest::SetTitles {
                titles: [
                    reader.read_str_with_length()?,
                    reader.read_str_with_length()?,
                    reader.read_str_with_length()?,
                    reader.read_str_with_length()?,
                    reader.read_str_with_length()?,
                ],
            },
            0x09 => AllianceRequest::SetRank {
                member_id: reader.read_int()?,
                up: reader.read_byte()? != 0,
            },
            0x0A => AllianceRequest::SetNotice {
                notice: reader.read_str_with_length()?,
            },
            _ => return Ok(HandlerResult::empty()),
        };
        Ok(HandlerResult::empty().with(WorldAction::Alliance(request)))
    }
}

/// `DENY_ALLIANCE_REQUEST`: the invited guild's master turned the card
/// down, or their client did (invitations switched off, or another card
/// showing). The inviter hears the same either way.
pub struct DenyAllianceRequestHandler;

impl WorldHandler for DenyAllianceRequestHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        // Why: 0x14 switched off, 0x15 another card, 0x16 declined.
        reader.read_byte()?;
        // The invitee's guild's name follows.
        let inviter = reader.read_str_with_length()?;
        Ok(HandlerResult::empty().with(WorldAction::Alliance(AllianceRequest::Deny { inviter })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn request(handler: &dyn WorldHandler, packet: &Packet) -> Option<AllianceRequest> {
        let mut packet = Packet::new(&packet.bytes);
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        match handler.handle(&mut packet, &mut ctx).unwrap().actions.pop() {
            Some(WorldAction::Alliance(request)) => Some(request),
            None => None,
            other => panic!("unexpected {other:?}"),
        }
    }

    fn operation(op: u8) -> Packet {
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::AllianceOperation as i16)
            .unwrap();
        packet.write_byte(op).unwrap();
        packet
    }

    fn read(packet: &Packet) -> Option<AllianceRequest> {
        request(&AllianceOperationHandler, packet)
    }

    #[test]
    fn reads_every_op_the_client_sends() {
        assert_eq!(read(&operation(1)), Some(AllianceRequest::Info));
        assert_eq!(read(&operation(2)), Some(AllianceRequest::Leave));
        assert_eq!(read(&operation(5)), None);

        let mut packet = operation(3);
        packet.write_str_with_length("Testers").unwrap();
        assert_eq!(
            read(&packet),
            Some(AllianceRequest::Invite {
                guild_name: "Testers".into()
            })
        );

        let mut packet = operation(4);
        packet.write_int(7).unwrap();
        packet.write_str_with_length("Mine").unwrap();
        assert_eq!(
            read(&packet),
            Some(AllianceRequest::Accept { alliance_id: 7 })
        );

        let mut packet = operation(6);
        packet.write_int(3).unwrap();
        packet.write_int(7).unwrap();
        assert_eq!(read(&packet), Some(AllianceRequest::Expel { guild_id: 3 }));

        let mut packet = operation(7);
        packet.write_int(20).unwrap();
        assert_eq!(
            read(&packet),
            Some(AllianceRequest::ChangeMaster { member_id: 20 })
        );

        let mut packet = operation(8);
        for title in ["A", "B", "C", "D", ""] {
            packet.write_str_with_length(title).unwrap();
        }
        assert_eq!(
            read(&packet),
            Some(AllianceRequest::SetTitles {
                titles: ["A", "B", "C", "D", ""].map(String::from)
            })
        );

        let mut packet = operation(9);
        packet.write_int(20).unwrap();
        packet.write_byte(0).unwrap();
        assert_eq!(
            read(&packet),
            Some(AllianceRequest::SetRank {
                member_id: 20,
                up: false
            })
        );

        let mut packet = operation(0x0A);
        packet.write_str_with_length("hi").unwrap();
        assert_eq!(
            read(&packet),
            Some(AllianceRequest::SetNotice {
                notice: "hi".into()
            })
        );
    }

    #[test]
    fn reads_the_inviter_of_a_denial() {
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::DenyAllianceRequest as i16)
            .unwrap();
        packet.write_byte(0x16).unwrap();
        packet.write_str_with_length("Alice").unwrap();
        packet.write_str_with_length("Mine").unwrap();
        assert_eq!(
            request(&DenyAllianceRequestHandler, &packet),
            Some(AllianceRequest::Deny {
                inviter: "Alice".into()
            })
        );
    }
}
