use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::quests::{QuestActionKind, QuestRequest};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use tracing::debug;

/// `QUEST_ACTION`: action byte, quest id, then the NPC for every action but
/// forfeit. The client may append its position, which is ignored. A
/// completion ends with the picked reward, -1 for none (v83 FUN_00716fe1).
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
            2 => {
                let npc_id = reader.read_int()?;
                // The choice comes last, after the position if there is one.
                let rest = packet.bytes.len() as u64 - reader.position();
                let choice = if rest >= 4 {
                    reader.set_position(packet.bytes.len() as u64 - 4);
                    reader.read_int()?
                } else {
                    -1
                };
                QuestActionKind::Complete { npc_id, choice }
            }
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
    fn parses_the_completion_choice_with_or_without_position() {
        let complete = |position: bool, choice: Option<i32>| {
            parse(|p| {
                p.write_byte(2).unwrap();
                p.write_short(3608).unwrap();
                p.write_int(2071001).unwrap();
                if position {
                    p.write_short(-20).unwrap();
                    p.write_short(150).unwrap();
                }
                if let Some(choice) = choice {
                    p.write_int(choice).unwrap();
                }
            })
            .unwrap()
            .kind
        };
        let picked = |choice| QuestActionKind::Complete {
            npc_id: 2071001,
            choice,
        };
        assert_eq!(complete(true, Some(1)), picked(1));
        assert_eq!(complete(false, Some(1)), picked(1));
        assert_eq!(complete(true, Some(-1)), picked(-1));
        // An older client sends no choice.
        assert_eq!(complete(false, None), picked(-1));
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
