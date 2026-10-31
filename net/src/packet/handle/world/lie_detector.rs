//! The Lie Detector (2190000, report c025): a player uses it on someone on
//! their map who is fighting, and the server shows that player a picture
//! of a few letters to type back within a minute. A right answer earns
//! them 5000 mesos; a wrong one, or none, costs them up to 7000, which go
//! to the tester. A GM's Lie Detector skill (9001009) tests the same way,
//! for nothing won or lost, and the GM hears how it went.

use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::packet::build::world::anti_macro::{
    build_anti_macro_result, build_anti_macro_result_by, AntiMacroResult, Detector,
};
use crate::packet::build::world::inventory::build_inventory_operation;
use crate::packet::build::world::stat::{build_stat_update, StatUpdate};
use crate::packet::op::RecvOpcode;
use game::{InventoryType, Player};
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;
use std::time::Duration;

/// How long a tested player has to answer; the client counts the same.
pub const TEST_TIME: Duration = Duration::from_secs(60);
/// How recently a player must have attacked to be tested.
pub const ATTACK_WINDOW: Duration = Duration::from_secs(10);
/// How long after a test the same player can't be tested again.
pub const TESTED_FOR: Duration = Duration::from_secs(60 * 60);
/// How many answers a test takes: the client counts the tries left, two
/// with the first picture and one with the next (0x7e6a94).
pub const TRIES: u8 = 2;
/// What a pass earns.
pub const PASS_REWARD: i32 = 5000;
/// The most a fail costs, given to the tester.
pub const FAIL_FINE: i32 = 7000;

/// The GM's Lie Detector skill, which the v83 client handles though its
/// data has no such skill (report 229).
pub const LIE_DETECTOR_SKILL: i32 = 9001009;

/// What a Lie Detector test is given with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LieDetectorBy {
    /// The item at `slot` in the USE bag, taken once the test begins.
    Item { slot: i16, item_id: i32 },
    /// A GM's skill.
    Skill,
}

impl LieDetectorBy {
    pub fn detector(self) -> Detector {
        match self {
            LieDetectorBy::Item { .. } => Detector::Item,
            LieDetectorBy::Skill => Detector::Skill,
        }
    }
}

/// A Lie Detector in use, while the server looks for the character named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LieDetectorUse {
    pub by: LieDetectorBy,
    pub name: String,
}

/// Whether `item_id` is a Lie Detector, of USE class 219.
pub fn is_lie_detector(item_id: i32) -> bool {
    item_id / 10_000 == 219
}

/// Whether `opcode` is an attack, which makes a player one to test: up
/// close, ranged, magic, by touch or by a summon.
pub fn is_attack(opcode: i16) -> bool {
    [
        RecvOpcode::CloseRangeAttack,
        RecvOpcode::RangedAttack,
        RecvOpcode::MagicAttack,
        RecvOpcode::TouchMonsterAttack,
        RecvOpcode::SummonAttack,
    ]
    .into_iter()
    .any(|attack| attack as i16 == opcode)
}

fn reply(result: AntiMacroResult) -> WorldResult {
    Ok(HandlerResult::reply(build_anti_macro_result(&result)?))
}

/// `USE_LIE_DETECTOR` (0x67, v83 FUN_00a262d9): the name typed, the slot
/// in the USE bag and the item. The client keeps the item for the server
/// to take, refuses it in a town or where the map forbids it, and names
/// only a character on its map.
pub struct UseLieDetectorHandler;

impl WorldHandler for UseLieDetectorHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let name = reader.read_str_with_length()?;
        let slot = reader.read_short()?;
        let item_id = reader.read_int()?;
        let player = &*ctx.player;
        let held = player
            .inventory
            .get(InventoryType::Use, slot)
            .is_some_and(|item| item.item_id == item_id);
        let allowed = crate::get_game_data()
            .ok()
            .and_then(|data| data.field(player.stats.map_id))
            .is_some_and(|field| field.may_lie_detect());
        if player.stats.hp <= 0 || !held || !is_lie_detector(item_id) || !allowed {
            return Ok(HandlerResult::empty());
        }
        if name.is_empty() || name.eq_ignore_ascii_case(player.name()) {
            return reply(AntiMacroResult::NotFound);
        }
        let test = LieDetectorUse {
            by: LieDetectorBy::Item { slot, item_id },
            name,
        };
        Ok(HandlerResult::empty().with(WorldAction::LieDetect(test)))
    }
}

/// `USE_LIE_DETECTOR_SKILL` (0x68, v83 FUN_0096d0d1): the name a GM typed
/// for the Lie Detector skill. The client asks no town or map's leave and
/// names only a character on its map who is fighting.
pub struct UseLieDetectorSkillHandler;

impl WorldHandler for UseLieDetectorSkillHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let name = reader.read_str_with_length()?;
        let player = &*ctx.player;
        // No Skill.nx of v83 has the skill, so only the book says a GM has it.
        let has_skill = player.gm_level > 0 && player.skills.level(LIE_DETECTOR_SKILL) > 0;
        if player.stats.hp <= 0 || !has_skill {
            return Ok(HandlerResult::empty());
        }
        if name.is_empty() || name.eq_ignore_ascii_case(player.name()) {
            return Ok(HandlerResult::reply(build_anti_macro_result_by(
                Detector::Skill,
                &AntiMacroResult::NotFound,
            )?));
        }
        let test = LieDetectorUse {
            by: LieDetectorBy::Skill,
            name,
        };
        Ok(HandlerResult::empty().with(WorldAction::LieDetect(test)))
    }
}

/// `ANSWER_LIE_DETECTOR` (0x69, v83 0x7e76b2 and 0x7e872f): the text typed
/// in the challenge's box. The client closes the challenge as it sends it,
/// so every answer must be judged.
pub struct AnswerLieDetectorHandler;

impl WorldHandler for AnswerLieDetectorHandler {
    fn handle(&self, packet: &mut Packet, _ctx: &mut WorldContext) -> WorldResult {
        let mut reader = Cursor::new(&packet.bytes);
        reader.read_short()?;
        let answer = reader.read_str_with_length()?;
        Ok(HandlerResult::empty().with(WorldAction::LieDetectorAnswer(answer)))
    }
}

/// Why the player can't be tested now, if they can't: elsewhere than the
/// tester, already in a test, or not fighting within `ATTACK_WINDOW`.
pub fn refusal(
    same_field: bool,
    in_test: bool,
    since_attack: Option<Duration>,
) -> Option<AntiMacroResult> {
    if !same_field {
        Some(AntiMacroResult::NotFound)
    } else if in_test {
        Some(AntiMacroResult::InTest)
    } else if !since_attack.is_some_and(|since| since <= ATTACK_WINDOW) {
        Some(AntiMacroResult::NotAttacking)
    } else {
        None
    }
}

/// The tester's side once a test began: the Lie Detector used up, if it is
/// still where it was, or the GM told whom the skill tests.
pub fn spend_detector(player: &mut Player, test: &LieDetectorUse) -> WorldResult {
    let LieDetectorBy::Item { slot, item_id } = test.by else {
        let used = AntiMacroResult::Used {
            name: test.name.clone(),
        };
        return Ok(HandlerResult::reply(build_anti_macro_result_by(
            Detector::Skill,
            &used,
        )?));
    };
    let held = player
        .inventory
        .get(InventoryType::Use, slot)
        .is_some_and(|item| item.item_id == item_id);
    let changes = if held {
        player
            .inventory_mut()
            .take(InventoryType::Use, slot, 1)
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    if changes.is_empty() {
        return Ok(HandlerResult::empty());
    }
    Ok(HandlerResult::reply(build_inventory_operation(
        true, &changes,
    )?))
}

fn with_meso(player: &mut Player, meso: i32, result: AntiMacroResult) -> WorldResult {
    player.meso = meso;
    player.mark_dirty();
    Ok(
        HandlerResult::reply(build_stat_update(true, &[StatUpdate::Meso(meso)])?)
            .with_reply(build_anti_macro_result(&result)?),
    )
}

/// The tested player passed: `PASS_REWARD` mesos, as far as they can hold.
pub fn passed(player: &mut Player) -> WorldResult {
    let meso = player.meso.saturating_add(PASS_REWARD);
    with_meso(player, meso, AntiMacroResult::Passed)
}

/// The tested player failed: up to `FAIL_FINE` of their mesos taken, the
/// amount returned for the tester.
pub fn failed(
    player: &mut Player,
) -> Result<(HandlerResult<WorldAction>, i32), crate::error::NetworkError> {
    let fine = FAIL_FINE.min(player.meso.max(0));
    let meso = player.meso - fine;
    Ok((with_meso(player, meso, AntiMacroResult::Failed)?, fine))
}

/// The tester's share of a failed test: `fine` mesos, as far as they can
/// hold, with "The user has failed..." even when the fine was nothing.
pub fn fine_received(player: &mut Player, fine: i32) -> WorldResult {
    let meso = player.meso.saturating_add(fine);
    with_meso(player, meso, AntiMacroResult::FineReceived)
}

#[cfg(test)]
mod tests {
    use super::*;
    use packet::io::write::PktWrite;

    fn replies(result: &HandlerResult<WorldAction>) -> Vec<Vec<u8>> {
        result
            .actions
            .iter()
            .filter_map(|action| match action {
                WorldAction::Reply(packet) => Some(packet.bytes.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn only_a_fighter_on_the_testers_field_out_of_a_test_is_tested() {
        let fought = Some(Duration::from_secs(3));
        assert_eq!(refusal(true, false, fought), None);
        assert_eq!(
            refusal(false, false, fought),
            Some(AntiMacroResult::NotFound)
        );
        assert_eq!(refusal(true, true, fought), Some(AntiMacroResult::InTest));
        let long_ago = Some(Duration::from_secs(11));
        assert_eq!(
            refusal(true, false, long_ago),
            Some(AntiMacroResult::NotAttacking)
        );
        assert_eq!(
            refusal(true, false, None),
            Some(AntiMacroResult::NotAttacking)
        );
    }

    #[test]
    fn a_pass_pays_5000_and_a_fail_takes_up_to_7000_for_the_tester() {
        let mut player = crate::codec::stats::sample_player();
        player.meso = 100;
        let result = passed(&mut player).unwrap();
        assert_eq!(player.meso, 5100);
        assert_eq!(replies(&result)[1], [0x2B, 0, 9, 1]);

        let (result, fine) = failed(&mut player).unwrap();
        assert_eq!((fine, player.meso), (5100, 0));
        assert_eq!(replies(&result)[1], [0x2B, 0, 7, 1]);
        player.meso = 10_000;
        let (_, fine) = failed(&mut player).unwrap();
        assert_eq!((fine, player.meso), (7000, 3000));

        let mut tester = crate::codec::stats::sample_player();
        tester.meso = i32::MAX - 10;
        let result = fine_received(&mut tester, fine).unwrap();
        assert_eq!(tester.meso, i32::MAX);
        assert_eq!(replies(&result)[1], [0x2B, 0, 11, 1]);
    }

    #[test]
    fn a_detector_not_held_does_nothing() {
        let mut player = crate::codec::stats::sample_player();
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseLieDetector as i16).unwrap();
        p.write_str_with_length("").unwrap();
        p.write_short(1).unwrap();
        p.write_int(2190000).unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let result = UseLieDetectorHandler.handle(&mut p, &mut ctx).unwrap();
        assert!(result.actions.is_empty());
    }

    fn skill_packet(name: &str) -> Packet {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::UseLieDetectorSkill as i16)
            .unwrap();
        p.write_str_with_length(name).unwrap();
        p
    }

    #[test]
    fn only_a_gm_with_the_skill_tests_by_it() {
        let mut player = crate::codec::stats::sample_player();
        player.skills.set(game::SkillRecord {
            skill_id: LIE_DETECTOR_SKILL,
            level: 1,
            master_level: 0,
            expiration: None,
        });
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let result = UseLieDetectorSkillHandler
            .handle(&mut skill_packet("Bo"), &mut ctx)
            .unwrap();
        assert!(result.actions.is_empty());

        player.gm_level = 3;
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let result = UseLieDetectorSkillHandler
            .handle(&mut skill_packet("Bo"), &mut ctx)
            .unwrap();
        assert!(matches!(
            result.actions.as_slice(),
            [WorldAction::LieDetect(LieDetectorUse { by: LieDetectorBy::Skill, name })]
                if name == "Bo"
        ));
        let result = UseLieDetectorSkillHandler
            .handle(&mut skill_packet(""), &mut ctx)
            .unwrap();
        assert_eq!(replies(&result), [vec![0x2B, 0, 0, 2]]);
    }

    #[test]
    fn the_skill_spends_nothing_and_tells_the_gm_whom_it_tests() {
        let mut player = crate::codec::stats::sample_player();
        let test = LieDetectorUse {
            by: LieDetectorBy::Skill,
            name: "Bo".into(),
        };
        let result = spend_detector(&mut player, &test).unwrap();
        assert_eq!(replies(&result), [vec![0x2B, 0, 5, 2, 2, 0, b'B', b'o']]);
    }

    #[test]
    fn an_answer_is_passed_on() {
        let mut player = crate::codec::stats::sample_player();
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::AnswerLieDetector as i16).unwrap();
        p.write_str_with_length("ak47").unwrap();
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player: &mut player,
        };
        let result = AnswerLieDetectorHandler.handle(&mut p, &mut ctx).unwrap();
        assert!(matches!(
            result.actions.as_slice(),
            [WorldAction::LieDetectorAnswer(answer)] if answer == "ak47"
        ));
    }
}
