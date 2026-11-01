//! The Brawler's and Marauder's morphs (reports 225b, 225c): Oak Barrel
//! hides its user in a barrel (Morph.nx 1002), Transformation and Super
//! Transformation turn them into a superman (1000, 1001). The Thunder
//! Breaker's Transformation (1000) and the Wind Archer's Eagle Eye (1003)
//! do too. The client draws them and keeps most of their rules; the server
//! gives the morph and refuses what the v83 client never sends.

use game::{BuffStat, Player};

pub const OAK_BARREL: i32 = 5101007;
pub const TRANSFORMATION: i32 = 5111005;
pub const SUPER_TRANSFORMATION: i32 = 5121003;
pub const TB_TRANSFORMATION: i32 = 15111002;
pub const EAGLE_EYE: i32 = 13111005;
/// The skills that turn their user into a superman (Morph.nx `superman`),
/// which field limit 0x2 refuses (FUN_00969e21).
pub const TRANSFORMS: [i32; 4] = [
    TRANSFORMATION,
    SUPER_TRANSFORMATION,
    TB_TRANSFORMATION,
    EAGLE_EYE,
];
const SHOCKWAVE: i32 = 5111006;
const TB_SHOCKWAVE: i32 = 15111003;
const DEMOLITION: i32 = 5121004;
const SNATCH: i32 = 5121005;

/// Double Shot, Oak Barrel and Burst Fire, which a superman can't use
/// (0x9673cf).
const NOT_TRANSFORMED: [i32; 3] = [5001003, OAK_BARREL, 5210000];

/// The template a character turns into. The client draws the id it is
/// sent, so a woman's superman is the man's plus 100 (Morph.nx 1100, 1101,
/// 1103), as Cosmic sends.
pub fn template(morph: i32, gender: u8) -> i32 {
    if gender == 1 && matches!(morph, 1000 | 1001 | 1003) {
        morph + 100
    } else {
        morph
    }
}

/// The skill whose morph the player has on, if any.
fn morphed_by(player: &Player) -> Option<i32> {
    let buffs = player.buffs.active();
    let buff = buffs.iter().find(|buff| buff.stat == BuffStat::Morph)?;
    Some(buff.skill_id)
}

/// Whether the player is a ghost (a ghost item, 236xxxx; report c039),
/// which v83 lets neither attack nor use any skill (FUN_0094e256,
/// FUN_00966f7a).
pub fn ghosted(player: &Player) -> bool {
    player.buffs.has(BuffStat::Ghost)
}

/// Whether the v83 client refuses `skill_id` (0 for a normal attack) for
/// the player's morph, or because they are a ghost. Both Shockwaves need a
/// transformation and Demolition and Snatch Super Transformation
/// (FUN_00969465); a superman can't use Double Shot, Oak Barrel or Burst
/// Fire, and in the barrel nothing at all, as the attack key breaks it
/// instead (FUN_00966f7a, FUN_0094e256).
pub fn refused(player: &Player, skill_id: i32) -> bool {
    if ghosted(player) {
        return true;
    }
    let by = morphed_by(player);
    let transformed = by.is_some_and(|by| TRANSFORMS.contains(&by));
    match skill_id {
        SHOCKWAVE | TB_SHOCKWAVE => !transformed,
        DEMOLITION | SNATCH => by != Some(SUPER_TRANSFORMATION),
        _ if transformed => NOT_TRANSFORMED.contains(&skill_id),
        _ => by == Some(OAK_BARREL),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::Buff;
    use std::time::{Duration, Instant};

    fn morphed(skill_id: Option<i32>) -> Player {
        let mut player = crate::codec::stats::sample_player();
        if let Some(skill_id) = skill_id {
            let lasts = Duration::from_secs(120);
            (player.buffs).give(Buff::new(
                skill_id,
                BuffStat::Morph,
                1000,
                Instant::now(),
                lasts,
            ));
        }
        player
    }

    #[test]
    fn a_woman_turns_into_the_template_100_on() {
        assert_eq!(template(1000, 0), 1000);
        assert_eq!(template(1000, 1), 1100);
        assert_eq!(template(1001, 1), 1101);
        assert_eq!(template(1002, 1), 1002);
    }

    #[test]
    fn the_superman_skills_need_their_transformation() {
        let plain = morphed(None);
        let transformed = morphed(Some(TRANSFORMATION));
        let superman = morphed(Some(SUPER_TRANSFORMATION));
        assert!(refused(&plain, SHOCKWAVE));
        assert!(!refused(&transformed, SHOCKWAVE));
        assert!(refused(&transformed, DEMOLITION));
        assert!(!refused(&superman, SNATCH));
        assert!(refused(&superman, OAK_BARREL));
        assert!(!refused(&superman, 0));
        assert!(!refused(&plain, OAK_BARREL));
    }

    #[test]
    fn the_cygnus_transformations_make_a_superman() {
        let plain = morphed(None);
        let breaker = morphed(Some(TB_TRANSFORMATION));
        let eagle = morphed(Some(EAGLE_EYE));
        assert!(refused(&plain, TB_SHOCKWAVE));
        assert!(!refused(&breaker, TB_SHOCKWAVE));
        assert!(!refused(&eagle, SHOCKWAVE));
        assert!(refused(&breaker, DEMOLITION));
        assert!(refused(&eagle, OAK_BARREL));
        assert!(!refused(&eagle, 0));
        assert_eq!(template(1003, 1), 1103);
    }

    #[test]
    fn the_barrel_refuses_everything() {
        let barrelled = morphed(Some(OAK_BARREL));
        assert!(refused(&barrelled, 0));
        assert!(refused(&barrelled, 5001001));
    }
}
