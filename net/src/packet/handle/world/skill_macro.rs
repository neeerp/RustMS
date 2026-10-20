//! SKILL_MACRO (0x6E, v83 FUN_00631f3b): the whole macro list, sent after
//! every edit in the Skill Macro dialog. There is no answer.

use crate::handler::{HandlerResult, WorldContext, WorldHandler, WorldResult};
use game::skill_macro::{may_hold, MAX_MACROS};
use game::SkillMacro;
use packet::{io::read::PktRead, Packet};
use std::io::Cursor;

pub struct SkillMacroHandler;

impl WorldHandler for SkillMacroHandler {
    fn handle(&self, packet: &mut Packet, ctx: &mut WorldContext) -> WorldResult {
        let mut r = Cursor::new(&packet.bytes);
        r.read_short()?;
        let count = r.read_byte()? as usize;
        if count > MAX_MACROS {
            return Ok(HandlerResult::empty());
        }
        let mut macros = Vec::with_capacity(count);
        for _ in 0..count {
            let name = r.read_str_with_length()?;
            let silent = r.read_byte()? != 0;
            let mut skills = [0; 3];
            for skill in &mut skills {
                *skill = r.read_int()?;
                if !may_hold(*skill, has_cooltime(*skill)) {
                    *skill = 0;
                }
            }
            macros.push(SkillMacro {
                name,
                silent,
                skills,
            });
        }
        if ctx.player.skill_macros.replace(macros).is_ok() {
            ctx.player.mark_dirty();
        }
        Ok(HandlerResult::empty())
    }
}

/// Whether the skill has a cooldown at any level; the client keeps such
/// skills out of macros.
fn has_cooltime(skill_id: i32) -> bool {
    crate::skills::info(skill_id).is_some_and(|info| info.levels.iter().any(|l| l.cooldown > 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::op::RecvOpcode;
    use game::{Player, SkillMacros};
    use packet::io::write::PktWrite;

    fn send(player: &mut Player, macros: &[(&str, u8, [i32; 3])], count: u8) {
        let mut p = Packet::new_empty();
        p.write_short(RecvOpcode::SkillMacro as i16).unwrap();
        p.write_byte(count).unwrap();
        for (name, silent, skills) in macros {
            p.write_str_with_length(name).unwrap();
            p.write_byte(*silent).unwrap();
            for skill in skills {
                p.write_int(*skill).unwrap();
            }
        }
        let mut ctx = WorldContext {
            client_id: 1,
            channel_id: 0,
            player,
        };
        let result = SkillMacroHandler.handle(&mut p, &mut ctx).unwrap();
        assert!(result.actions.is_empty());
    }

    fn kept() -> SkillMacros {
        SkillMacros::new(vec![SkillMacro {
            name: "kept".into(),
            silent: false,
            skills: [1001004, 0, 0],
        }])
    }

    #[test]
    fn the_list_is_kept_with_illegal_skills_emptied() {
        let mut player = crate::codec::stats::sample_player();
        let macros = [
            ("buffs", 1, [1001003, 2101002, 0]),
            ("hit", 0, [1000000, 0, 1001004]),
        ];
        send(&mut player, &macros, 2);
        assert!(player.is_dirty());
        assert_eq!(
            player.skill_macros.macros(),
            [
                SkillMacro {
                    name: "buffs".into(),
                    silent: true,
                    skills: [1001003, 0, 0],
                },
                SkillMacro {
                    name: "hit".into(),
                    silent: false,
                    skills: [0, 0, 1001004],
                },
            ]
        );
        send(&mut player, &[], 0);
        assert!(player.skill_macros.macros().is_empty());
    }

    #[test]
    fn six_macros_or_a_long_name_are_ignored() {
        let mut player = crate::codec::stats::sample_player();
        player.skill_macros = kept();
        send(&mut player, &[("a", 0, [0; 3]); 6], 6);
        send(&mut player, &[("thirteen byte", 0, [0; 3])], 1);
        assert_eq!(player.skill_macros, kept());
        assert!(!player.is_dirty());
    }
}
