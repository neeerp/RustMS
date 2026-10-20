//! A Dark Knight's Berserk (report 221). The v83 client keeps the state
//! itself: it turns Berserk on when HP falls to `x` percent of max HP and
//! off when HP climbs over it, and sends an empty CALC_DAMAGE_STAT (0x6C) on
//! each change (FUN_00a292bb). The server answers with the aura and allows
//! for Berserk's `damage` percent on every weapon hit.

use game::Player;

use crate::skills::{self, Learned};

pub const BERSERK: i32 = 1320006;
const DARK_KNIGHT: i16 = 132;

/// Berserk, if the player is a Dark Knight who learned it.
fn learned(player: &Player) -> Option<Learned> {
    if player.stats.job != DARK_KNIGHT {
        return None;
    }
    skills::learned(player, BERSERK)
}

/// Whether the player's HP is low enough for Berserk, by the client's
/// rule: no more than `x` percent of max HP.
fn low(player: &Player, skill: &Learned) -> bool {
    i32::from(player.stats.hp) <= skill.effect.x * player.effective_max_hp() / 100
}

/// Whether Berserk is on now.
pub fn on(player: &Player) -> bool {
    learned(player).is_some_and(|skill| low(player, &skill))
}

/// The percent Berserk raises every weapon hit by, 100 plus its `damage`
/// (PDamage FUN_0078df87); 100 when it is off. It counts while the server
/// last saw it on too, since the client decides by the HP it was last sent.
pub fn damage_percent(player: &Player) -> i32 {
    learned(player)
        .filter(|skill| player.berserk || low(player, skill))
        .map_or(100, |skill| 100 + skill.effect.damage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::SkillRecord;

    fn dark_knight(job: i16, level: i32, hp: i16) -> Player {
        let mut player = crate::codec::stats::sample_player();
        player.stats.job = job;
        player.stats.max_hp = 1000;
        player.stats.hp = hp;
        player.skills.set(SkillRecord {
            skill_id: BERSERK,
            level,
            master_level: 0,
            expiration: None,
        });
        player
    }

    #[test]
    fn berserk_is_on_at_x_percent_of_max_hp_and_below() {
        // Level 30: x 50, damage 100.
        assert!(on(&dark_knight(DARK_KNIGHT, 30, 500)));
        assert!(!on(&dark_knight(DARK_KNIGHT, 30, 501)));
        assert_eq!(damage_percent(&dark_knight(DARK_KNIGHT, 30, 100)), 200);
        assert_eq!(damage_percent(&dark_knight(DARK_KNIGHT, 30, 900)), 100);
        // Only a Dark Knight who learned it.
        assert!(!on(&dark_knight(131, 30, 100)));
        assert!(!on(&dark_knight(DARK_KNIGHT, 0, 100)));
    }

    #[test]
    fn berserk_still_counts_while_the_server_last_saw_it_on() {
        let mut player = dark_knight(DARK_KNIGHT, 30, 900);
        player.berserk = true;
        assert_eq!(damage_percent(&player), 200);
    }
}
