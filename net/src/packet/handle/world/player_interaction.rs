use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::packet::build::world::mini_room::TRADE_ROOM;
use crate::trade::TradeRequest;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// `PLAYER_INTERACTION`, for the trading room only: mode 0 creates a room of
/// a type, 2 invites, 3 declines, 4 accepts (visits), 6 chats, 0xa leaves,
/// 0xf puts up an item, 0x10 mesos, and 0x11 presses Trade. The runtime holds
/// the offers and the field the room.
pub struct PlayerInteractionHandler;

impl WorldHandler for PlayerInteractionHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let request = match reader.read_byte()? {
            0x00 => match reader.read_byte()? {
                TRADE_ROOM => TradeRequest::Create,
                kind => {
                    debug!(kind, "Ignoring mini room of another type");
                    return Ok(HandlerResult::empty());
                }
            },
            0x02 => TradeRequest::Invite {
                target: reader.read_int()?,
            },
            0x03 => TradeRequest::Decline {
                sn: reader.read_int()?,
                reason: reader.read_byte()?,
            },
            0x04 => TradeRequest::Visit {
                sn: reader.read_int()?,
            },
            0x06 => TradeRequest::Chat {
                text: reader.read_str_with_length()?,
            },
            0x0a => TradeRequest::Leave,
            0x0f => TradeRequest::PutItem {
                kind: i16::from(reader.read_byte()?),
                slot: reader.read_short()?,
                count: reader.read_short()?,
                trade_slot: reader.read_byte()?,
            },
            0x10 => TradeRequest::Meso(reader.read_int()?),
            0x11 => TradeRequest::Confirm,
            other => {
                // 0x14 answers the partner's Trade press with item CRCs.
                debug!(mode = other, "Ignoring player interaction");
                return Ok(HandlerResult::empty());
            }
        };
        Ok(HandlerResult::empty().with(WorldAction::Trade(request)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn parse<T>(build: impl FnOnce(&mut Packet) -> T) -> Option<TradeRequest> {
        let mut packet = Packet::new_empty();
        packet
            .write_short(RecvOpcode::PlayerInteraction as i16)
            .unwrap();
        build(&mut packet);
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let result = PlayerInteractionHandler
            .handle(&mut packet, &mut ctx)
            .unwrap();
        match result.actions.into_iter().next() {
            Some(WorldAction::Trade(request)) => Some(request),
            None => None,
            Some(other) => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn reads_the_room_modes() {
        let create = parse(|p| p.write_bytes(&[0, 3, 0]).unwrap());
        assert_eq!(create, Some(TradeRequest::Create));
        let invite = parse(|p| {
            p.write_byte(2).unwrap();
            p.write_int(7).unwrap();
        });
        assert_eq!(invite, Some(TradeRequest::Invite { target: 7 }));
        let decline = parse(|p| {
            p.write_byte(3).unwrap();
            p.write_int(5).unwrap();
            p.write_byte(4).unwrap();
        });
        assert_eq!(decline, Some(TradeRequest::Decline { sn: 5, reason: 4 }));
        let visit = parse(|p| {
            p.write_byte(4).unwrap();
            p.write_int(5).unwrap();
            p.write_bytes(&[0, 0]).unwrap();
        });
        assert_eq!(visit, Some(TradeRequest::Visit { sn: 5 }));
        let chat = parse(|p| {
            p.write_byte(6).unwrap();
            p.write_str_with_length("hi").unwrap();
        });
        assert_eq!(
            chat,
            Some(TradeRequest::Chat {
                text: "hi".to_string()
            })
        );
        assert_eq!(parse(|p| p.write_byte(0x0a)), Some(TradeRequest::Leave));
    }

    #[test]
    fn reads_the_trade_modes() {
        let put = parse(|p| {
            p.write_byte(0x0f).unwrap();
            p.write_byte(2).unwrap();
            p.write_short(4).unwrap();
            p.write_short(10).unwrap();
            p.write_byte(1).unwrap();
        });
        assert_eq!(
            put,
            Some(TradeRequest::PutItem {
                kind: 2,
                slot: 4,
                count: 10,
                trade_slot: 1
            })
        );
        let meso = parse(|p| {
            p.write_byte(0x10).unwrap();
            p.write_int(5000).unwrap();
        });
        assert_eq!(meso, Some(TradeRequest::Meso(5000)));
        let confirm = parse(|p| {
            p.write_byte(0x11).unwrap();
            p.write_byte(0).unwrap();
        });
        assert_eq!(confirm, Some(TradeRequest::Confirm));
    }

    #[test]
    fn ignores_other_rooms_and_the_crc_answer() {
        assert_eq!(parse(|p| p.write_bytes(&[0, 4, 0])), None);
        assert_eq!(parse(|p| p.write_bytes(&[0x14, 0])), None);
    }
}
