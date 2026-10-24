use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::packet::build::world::claim::{build_claim_result, ClaimResult};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

/// What a report costs; the report window says so ("YOU NEED 300 MESOS TO
/// ALERT GM").
pub const CLAIM_FEE: i32 = 300;

/// `CLAIM` (0x6A, v83 FUN_00a2719c): 1 for a conversation or 0 for an
/// illegal program, the character's name, the reason, the description,
/// and for a conversation the lines in which that character or the
/// reporter spoke.
pub struct ClaimHandler;

impl WorldHandler for ClaimHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let kind = reader.read_byte()?;
        let victim = reader.read_str_with_length()?;
        let reason = reader.read_byte()?;
        let description = reader.read_str_with_length()?;
        let chat_log = match kind {
            1 => reader.read_str_with_length()?,
            0 => String::new(),
            _ => return Ok(HandlerResult::empty()),
        };
        if ctx.player.meso < CLAIM_FEE {
            return Ok(HandlerResult::reply(build_claim_result(
                ClaimResult::NoMesos,
            )?));
        }
        Ok(HandlerResult::empty().with(WorldAction::Claim {
            conversation: kind == 1,
            victim,
            reason,
            description,
            chat_log,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use packet::io::write::PktWrite;

    fn claim(kind: u8, meso: i32) -> Vec<WorldAction> {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::Claim as i16).unwrap();
        p.write_byte(kind).unwrap();
        p.write_str_with_length("Rude").unwrap();
        p.write_byte(1).unwrap();
        p.write_str_with_length("said bad words").unwrap();
        if kind == 1 {
            p.write_str_with_length("Rude : bad words").unwrap();
        }
        let mut player = crate::codec::stats::sample_player();
        player.meso = meso;
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        ClaimHandler.handle(&mut p, &mut ctx).unwrap().actions
    }

    #[test]
    fn a_conversation_report_carries_its_lines() {
        let actions = claim(1, 300);
        let [WorldAction::Claim {
            conversation: true,
            victim,
            reason: 1,
            description,
            chat_log,
        }] = actions.as_slice()
        else {
            panic!("expected a claim: {actions:?}");
        };
        assert_eq!(
            (victim.as_str(), description.as_str(), chat_log.as_str()),
            ("Rude", "said bad words", "Rude : bad words")
        );
    }

    #[test]
    fn a_program_report_has_no_lines() {
        let actions = claim(0, 300);
        assert!(matches!(
            actions.as_slice(),
            [WorldAction::Claim { conversation: false, chat_log, .. }] if chat_log.is_empty()
        ));
    }

    #[test]
    fn a_report_costs_300_mesos() {
        let actions = claim(1, 299);
        let [WorldAction::Reply(packet)] = actions.as_slice() else {
            panic!("expected a reply: {actions:?}");
        };
        assert_eq!(packet.bytes, [0x2D, 0, 0x43]);
    }
}
