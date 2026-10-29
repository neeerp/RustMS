use crate::handler::{
    GuildRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use crate::packet::build::world::guild::InviteRefusal;
use game::guild::Emblem;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// `GUILD_OPERATION`: ask for the guild again, name a new guild in answer
/// to the name prompt, agree to found the party leader's, invite, join,
/// leave, expel, or change the rank titles, a member's rank, the emblem or
/// the notice. Other modes aren't supported yet.
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
            0x05 => GuildRequest::Invite {
                name: reader.read_str_with_length()?,
            },
            // The player's id follows; the server knows it.
            0x06 => GuildRequest::Accept {
                guild_id: reader.read_int()?,
            },
            // The player's id and name follow.
            0x07 => GuildRequest::Leave,
            // The member's name follows.
            0x08 => GuildRequest::Expel {
                member_id: reader.read_int()?,
            },
            0x0D => GuildRequest::SetTitles {
                titles: [
                    reader.read_str_with_length()?,
                    reader.read_str_with_length()?,
                    reader.read_str_with_length()?,
                    reader.read_str_with_length()?,
                    reader.read_str_with_length()?,
                ],
            },
            0x0E => GuildRequest::SetRank {
                member_id: reader.read_int()?,
                rank: reader.read_byte()?,
            },
            0x0F => GuildRequest::SetEmblem {
                emblem: Emblem {
                    background: reader.read_short()? as u16,
                    background_color: reader.read_byte()?,
                    mark: reader.read_short()? as u16,
                    mark_color: reader.read_byte()?,
                },
            },
            0x10 => GuildRequest::SetNotice {
                notice: reader.read_str_with_length()?,
            },
            // The player's id comes first; the server knows it.
            0x1E => {
                reader.read_int()?;
                GuildRequest::Agree {
                    agree: reader.read_byte()? != 0,
                }
            }
            _ => return Ok(HandlerResult::empty()),
        };
        Ok(HandlerResult::empty().with(WorldAction::Guild(request)))
    }
}

/// `DENY_GUILD_REQUEST`: the invitee turned an invitation down, or their
/// client did because they were busy.
pub struct DenyGuildRequestHandler;

impl WorldHandler for DenyGuildRequestHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let Some(refusal) = InviteRefusal::from_code(reader.read_byte()?) else {
            return Ok(HandlerResult::empty());
        };
        // The invitee's own name follows.
        let inviter = reader.read_str_with_length()?;
        Ok(
            HandlerResult::empty()
                .with(WorldAction::Guild(GuildRequest::Deny { inviter, refusal })),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn request(handler: &dyn WorldHandler, bytes: &[u8]) -> Option<GuildRequest> {
        let mut packet = Packet::new(bytes);
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let result = handler.handle(&mut packet, &mut ctx);
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

    fn guild_request(packet: &Packet) -> Option<GuildRequest> {
        request(&GuildOperationHandler, &packet.bytes)
    }

    #[test]
    fn reads_reload_and_create() {
        assert_eq!(guild_request(&operation(0)), Some(GuildRequest::Reload));

        let mut packet = operation(2);
        packet.write_str_with_length("Testers").unwrap();
        assert_eq!(
            guild_request(&packet),
            Some(GuildRequest::Create {
                name: "Testers".into()
            })
        );

        // Alliances and the rest come later.
        assert_eq!(guild_request(&operation(0x30)), None);
    }

    #[test]
    fn reads_an_agreement() {
        for (byte, agree) in [(1, true), (0, false)] {
            let mut packet = operation(0x1E);
            packet.write_int(7).unwrap();
            packet.write_byte(byte).unwrap();
            assert_eq!(guild_request(&packet), Some(GuildRequest::Agree { agree }));
        }
    }

    #[test]
    fn reads_the_emblem() {
        let mut packet = operation(0x0F);
        packet.write_short(1001).unwrap();
        packet.write_byte(2).unwrap();
        packet.write_short(9026).unwrap();
        packet.write_byte(16).unwrap();
        assert_eq!(
            guild_request(&packet),
            Some(GuildRequest::SetEmblem {
                emblem: Emblem {
                    background: 1001,
                    background_color: 2,
                    mark: 9026,
                    mark_color: 16,
                }
            })
        );
    }

    #[test]
    fn reads_titles_rank_and_notice() {
        let mut packet = operation(0x0D);
        for title in ["Boss", "Deputy", "Grunt", "Newbie", ""] {
            packet.write_str_with_length(title).unwrap();
        }
        assert_eq!(
            guild_request(&packet),
            Some(GuildRequest::SetTitles {
                titles: ["Boss", "Deputy", "Grunt", "Newbie", ""].map(String::from)
            })
        );

        let mut packet = operation(0x0E);
        packet.write_int(7).unwrap();
        packet.write_byte(3).unwrap();
        assert_eq!(
            guild_request(&packet),
            Some(GuildRequest::SetRank {
                member_id: 7,
                rank: 3
            })
        );

        let mut packet = operation(0x10);
        packet.write_str_with_length("hi").unwrap();
        assert_eq!(
            guild_request(&packet),
            Some(GuildRequest::SetNotice {
                notice: "hi".into()
            })
        );
    }

    #[test]
    fn reads_invite_accept_leave_and_expel() {
        let mut packet = operation(5);
        packet.write_str_with_length("Bobby").unwrap();
        assert_eq!(
            guild_request(&packet),
            Some(GuildRequest::Invite {
                name: "Bobby".into()
            })
        );

        let mut packet = operation(6);
        packet.write_int(4).unwrap();
        packet.write_int(9).unwrap();
        assert_eq!(
            guild_request(&packet),
            Some(GuildRequest::Accept { guild_id: 4 })
        );

        let mut packet = operation(7);
        packet.write_int(9).unwrap();
        packet.write_str_with_length("Bobby").unwrap();
        assert_eq!(guild_request(&packet), Some(GuildRequest::Leave));

        let mut packet = operation(8);
        packet.write_int(9).unwrap();
        packet.write_str_with_length("Bobby").unwrap();
        assert_eq!(
            guild_request(&packet),
            Some(GuildRequest::Expel { member_id: 9 })
        );
    }

    #[test]
    fn reads_refusals() {
        let deny = |reason: u8| {
            let mut packet = Packet::new_empty();
            packet
                .write_short(RecvOpcode::DenyGuildRequest as i16)
                .unwrap();
            packet.write_byte(reason).unwrap();
            packet.write_str_with_length("Alice").unwrap();
            packet.write_str_with_length("Bobby").unwrap();
            request(&DenyGuildRequestHandler, &packet.bytes)
        };
        for (reason, refusal) in [
            (0x35, InviteRefusal::Busy),
            (0x36, InviteRefusal::AnotherInvitation),
            (0x37, InviteRefusal::Declined),
        ] {
            assert_eq!(
                deny(reason),
                Some(GuildRequest::Deny {
                    inviter: "Alice".into(),
                    refusal
                })
            );
        }
        assert_eq!(deny(0x2A), None);
    }
}
