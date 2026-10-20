//! CALC_DAMAGE_STAT (0x6C): the client's Berserk turned on or off
//! (report 221). It carries nothing, so the server works the state out
//! itself and shows the aura when it changed.

use crate::berserk::{self, BERSERK};
use crate::handler::{HandlerResult, WorldAction, WorldContext, WorldHandler, WorldResult};
use crate::packet::build::world::buff::{build_own_skill_effect, build_skill_effect};
use packet::{io::write::PktWrite, Packet};

pub struct CalcDamageStatHandler;

impl WorldHandler for CalcDamageStatHandler {
    fn handle(&self, _packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let player = &mut *ctx.player;
        let on = berserk::on(player);
        if on == player.berserk {
            return Ok(HandlerResult::empty());
        }
        player.berserk = on;
        // The client draws Berserk's `effect` from the skill alone, but
        // reads the levels first (SHOW_ITEM_GAIN_INCHAT 1, 0x937982).
        let skill_level = player.skills.level(BERSERK) as u8;
        let level = player.stats.level;
        let on = u8::from(on);
        let mut own = build_own_skill_effect(BERSERK, level, skill_level)?;
        own.write_byte(on)?;
        let mut others = build_skill_effect(player.stats.id, BERSERK, level, skill_level)?;
        others.write_byte(on)?;
        Ok(HandlerResult::reply(own).with(WorldAction::BerserkChanged(others)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::SendOpcode;
    use game::{Player, SkillRecord};

    fn dark_knight(hp: i16) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.stats.job = 132;
        player.stats.max_hp = 1000;
        player.stats.hp = hp;
        player.skills.set(SkillRecord {
            skill_id: BERSERK,
            level: 30,
            master_level: 0,
            expiration: None,
        });
        player
    }

    fn calc(player: &mut Player) -> Vec<WorldAction> {
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        let mut packet = Packet::new_empty();
        let result = CalcDamageStatHandler.handle(&mut packet, &mut ctx);
        result.unwrap().actions
    }

    #[test]
    fn berserk_shows_its_aura_when_it_turns_on_and_off() {
        let mut player = dark_knight(400);
        let level = player.stats.level;
        let id = player.stats.id.to_le_bytes();

        let on = calc(&mut player);
        assert!(player.berserk);
        let [WorldAction::Reply(mine), WorldAction::BerserkChanged(others)] = &on[..] else {
            panic!("{on:?}");
        };
        let mut own = (SendOpcode::ShowItemGainInChat as i16)
            .to_le_bytes()
            .to_vec();
        own.extend([1, 0x46, 0x24, 0x14, 0, level, 30, 1]);
        assert_eq!(mine.bytes, own);
        let mut foreign = (SendOpcode::ShowForeignEffect as i16)
            .to_le_bytes()
            .to_vec();
        foreign.extend(id);
        foreign.extend([1, 0x46, 0x24, 0x14, 0, level, 30, 1]);
        assert_eq!(others.bytes, foreign);
        assert!(player.field_avatar(0, 0, 0).berserk);

        // Nothing changed, nothing to show.
        assert!(calc(&mut player).is_empty());

        player.stats.hp = 600;
        let off = calc(&mut player);
        assert!(!player.berserk);
        let [WorldAction::Reply(mine), WorldAction::BerserkChanged(others)] = &off[..] else {
            panic!("{off:?}");
        };
        assert_eq!(mine.bytes[mine.bytes.len() - 3..], [level, 30, 0]);
        assert_eq!(others.bytes[others.bytes.len() - 3..], [level, 30, 0]);
    }
}
