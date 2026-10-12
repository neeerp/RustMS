use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::quests::{QuestActionKind, QuestRequest};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// `QUEST_ACTION`: action byte, quest id, then the NPC for every action but
/// forfeit. The client may append its position, and a completion may append
/// a chosen reward; no supported quest has choosable rewards, so both are
/// ignored.
pub struct QuestActionHandler;

impl WorldHandler for QuestActionHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let action = reader.read_byte()?;
        let quest_id = reader.read_short()?;
        let kind = match action {
            1 => QuestActionKind::Start {
                npc_id: reader.read_int()?,
            },
            2 => QuestActionKind::Complete {
                npc_id: reader.read_int()?,
            },
            3 => QuestActionKind::Forfeit,
            4 => QuestActionKind::ScriptStart {
                npc_id: reader.read_int()?,
            },
            5 => QuestActionKind::ScriptEnd {
                npc_id: reader.read_int()?,
            },
            other => {
                // 0 restores a lost quest item; there are none yet.
                debug!(action = other, quest_id, "Ignoring quest action");
                return Ok(HandlerResult::empty());
            }
        };
        Ok(HandlerResult::empty().with(WorldAction::Quest(QuestRequest { quest_id, kind })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn parse(build: impl FnOnce(&mut Packet)) -> Option<QuestRequest> {
        let mut packet = Packet::new_empty();
        packet.write_short(RecvOpcode::QuestAction as i16).unwrap();
        build(&mut packet);
        let mut player = crate::codec::stats::sample_player();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let result = QuestActionHandler.handle(&mut packet, &mut ctx).unwrap();
        result.actions.into_iter().find_map(|a| match a {
            WorldAction::Quest(request) => Some(request),
            _ => None,
        })
    }

    #[test]
    fn parses_start_with_and_without_position() {
        let bare = parse(|p| {
            p.write_byte(1).unwrap();
            p.write_short(1031).unwrap();
            p.write_int(2101).unwrap();
        });
        let with_position = parse(|p| {
            p.write_byte(1).unwrap();
            p.write_short(1031).unwrap();
            p.write_int(2101).unwrap();
            p.write_short(-20).unwrap();
            p.write_short(150).unwrap();
        });
        let expected = QuestRequest {
            quest_id: 1031,
            kind: QuestActionKind::Start { npc_id: 2101 },
        };
        assert_eq!(bare, Some(expected));
        assert_eq!(with_position, Some(expected));
    }

    #[test]
    fn parses_completion_and_ignores_position_and_selection() {
        let request = parse(|p| {
            p.write_byte(2).unwrap();
            p.write_short(1031).unwrap();
            p.write_int(2100).unwrap();
            p.write_short(0).unwrap();
            p.write_short(0).unwrap();
            p.write_short(3).unwrap();
        });
        assert_eq!(
            request.unwrap().kind,
            QuestActionKind::Complete { npc_id: 2100 }
        );
    }

    #[test]
    fn parses_forfeit_and_ignores_lost_item_restores() {
        let forfeit = parse(|p| {
            p.write_byte(3).unwrap();
            p.write_short(1031).unwrap();
        });
        assert_eq!(forfeit.unwrap().kind, QuestActionKind::Forfeit);
        let restore = parse(|p| {
            p.write_byte(0).unwrap();
            p.write_short(1031).unwrap();
            p.write_int(0).unwrap();
            p.write_int(4031003).unwrap();
        });
        assert_eq!(restore, None);
    }
}
