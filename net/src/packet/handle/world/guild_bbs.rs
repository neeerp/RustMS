use crate::handler::{
    BbsRequest, GuildRequest, HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult,
};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// `GUILD_BBS`: write, delete, list, show or reply on the guild bulletin
/// board (reports scratch/251/bbs.md, section 1).
pub struct GuildBbsHandler;

impl WorldHandler for GuildBbsHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let request = match reader.read_byte()? {
            0 => {
                let edit = reader.read_byte()? == 1;
                let edit = if edit { Some(reader.read_int()?) } else { None };
                BbsRequest::Write {
                    edit,
                    notice: reader.read_byte()? == 1,
                    title: reader.read_str_with_length()?,
                    body: reader.read_str_with_length()?,
                    icon: reader.read_int()?,
                }
            }
            1 => BbsRequest::Delete {
                thread: reader.read_int()?,
            },
            2 => BbsRequest::List {
                page: reader.read_int()?,
            },
            3 => BbsRequest::Show {
                thread: reader.read_int()?,
            },
            4 => BbsRequest::Reply {
                thread: reader.read_int()?,
                text: reader.read_str_with_length()?,
            },
            // The reply's thread comes first; the reply names it already.
            5 => {
                reader.read_int()?;
                BbsRequest::DeleteReply {
                    reply: reader.read_int()?,
                }
            }
            _ => return Ok(HandlerResult::empty()),
        };
        Ok(HandlerResult::empty().with(WorldAction::Guild(GuildRequest::Bbs(request))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn request(packet: &Packet) -> Option<BbsRequest> {
        let mut packet = Packet::new(&packet.bytes);
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        match GuildBbsHandler
            .handle(&mut packet, &mut ctx)
            .unwrap()
            .actions
            .pop()
        {
            Some(WorldAction::Guild(GuildRequest::Bbs(request))) => Some(request),
            None => None,
            other => panic!("unexpected {other:?}"),
        }
    }

    fn bbs(mode: u8) -> Packet {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::GuildBbs as i16).unwrap();
        packet.write_byte(mode).unwrap();
        packet
    }

    #[test]
    fn reads_a_new_post_and_an_edit() {
        let mut packet = bbs(0);
        packet.write_byte(0).unwrap();
        packet.write_byte(1).unwrap();
        packet.write_str_with_length("Rules").unwrap();
        packet.write_str_with_length("Be nice").unwrap();
        packet.write_int(101).unwrap();
        assert_eq!(
            request(&packet),
            Some(BbsRequest::Write {
                edit: None,
                notice: true,
                title: "Rules".into(),
                body: "Be nice".into(),
                icon: 101,
            })
        );

        let mut packet = bbs(0);
        packet.write_byte(1).unwrap();
        packet.write_int(7).unwrap();
        packet.write_byte(0).unwrap();
        packet.write_str_with_length("t").unwrap();
        packet.write_str_with_length("b").unwrap();
        packet.write_int(0).unwrap();
        assert_eq!(
            request(&packet),
            Some(BbsRequest::Write {
                edit: Some(7),
                notice: false,
                title: "t".into(),
                body: "b".into(),
                icon: 0,
            })
        );
    }

    #[test]
    fn reads_the_other_modes() {
        let with_int = |mode, value| {
            let mut packet = bbs(mode);
            packet.write_int(value).unwrap();
            request(&packet)
        };
        assert_eq!(with_int(1, 3), Some(BbsRequest::Delete { thread: 3 }));
        assert_eq!(with_int(2, 1), Some(BbsRequest::List { page: 1 }));
        assert_eq!(with_int(3, 0), Some(BbsRequest::Show { thread: 0 }));

        let mut packet = bbs(4);
        packet.write_int(2).unwrap();
        packet.write_str_with_length("hi").unwrap();
        assert_eq!(
            request(&packet),
            Some(BbsRequest::Reply {
                thread: 2,
                text: "hi".into()
            })
        );

        let mut packet = bbs(5);
        packet.write_int(2).unwrap();
        packet.write_int(40).unwrap();
        assert_eq!(
            request(&packet),
            Some(BbsRequest::DeleteReply { reply: 40 })
        );

        assert_eq!(request(&bbs(6)), None);
    }
}
