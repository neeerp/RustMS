//! Mob templates from `Mob.nx` (`<id>.img/info`), looked up on demand.

use crate::{GameDataError, NxDataFile};
use std::path::Path;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MobInfo {
    pub mob_id: i32,
    pub level: i16,
    pub max_hp: i32,
    pub max_mp: i32,
    pub exp: i32,
    /// Physical attack (`PADamage`): the most touching it can hurt.
    pub attack: i32,
    /// Magic attack (`MADamage`), behind its magic attacks.
    pub magic_attack: i32,
    pub defense: i32,
    pub accuracy: i32,
    pub avoid: i32,
    pub speed: i32,
    /// Hurts players who touch it.
    pub body_attack: bool,
    /// Never attacks.
    pub not_attack: bool,
    /// Goes for nearby players without being hit first.
    pub first_attack: bool,
    pub boss: bool,
    pub undead: bool,
    /// What every hit does to it, whatever the attack (`fixedDamage`), or
    /// 0.
    pub fixed_damage: i32,
    /// Power Guard sends nothing back at it (`invincible`).
    pub invincible: bool,
    /// Anyone may pick its drops up at once (`publicReward`).
    pub public_reward: bool,
    /// Its drops fly out wide, anyone's at once (`explosiveReward`).
    pub explosive_reward: bool,
    /// How each element hurts it (`elemAttr`): pairs of an element letter
    /// and a degree, `1` for immune, as "I1S2".
    pub element: String,
    /// Its attacks (`attack1/info`, `attack2/info`, ...), in order: the
    /// client names attack `i` by `i`.
    pub attacks: Vec<MobAttack>,
    /// The mob skills it may cast (`skill/<n>`), in order.
    pub skills: Vec<MobSkillEntry>,
    /// Where its Banish sends players (`ban`).
    pub banish: Option<Banish>,
    /// The mobs that come out of it where it dies (`revive`), in order.
    pub revive: Vec<i32>,
    /// How long it takes to die, in ms: its `die1` frames' delays (100 for
    /// one without), or 500 without any, as Cosmic has it.
    pub dying_ms: i32,
    /// How long it stays before it leaves on its own, in seconds
    /// (`removeAfter`), or 0 for as long as it lives.
    pub remove_after: i32,
}

/// Where a mob's Banish sends players (`ban/banMap/0`), and what it tells
/// them (`banMsg`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Banish {
    pub map_id: i32,
    pub portal: String,
    pub message: String,
}

/// One of a mob's skills (`skill/<n>`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MobSkillEntry {
    /// The `MobSkill.img` id and level.
    pub skill: i32,
    pub level: i32,
    /// Its animation, `skill<action>`.
    pub action: i32,
    /// Milliseconds into the animation before the skill's effect shows
    /// (`effectAfter`).
    pub effect_after: i32,
}

/// One of a mob's attacks (`attackN/info`), as far as the server cares.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MobAttack {
    /// 0 a rectangle, 1 a shot at a target, 2 a ball, 3 and 4 an area.
    pub kind: i32,
    /// Hurts with the mob's magic attack rather than its physical one.
    pub magic: bool,
    /// The MP it costs the mob (`conMP`).
    pub mp_cost: i32,
    /// Its own physical attack (`PADamage`), or 0 for the mob's.
    pub attack: i32,
    /// Leaves its victim at 1 HP (`deadlyAttack`).
    pub deadly: bool,
    /// The MP it burns (`mpBurn`), or 0.
    pub mp_burn: i32,
    /// The mob skill it inflicts (`disease`, `level`): 0 for none.
    pub disease: i32,
    pub disease_level: i32,
}

impl MobInfo {
    /// Whether the element `letter` (F fire, I ice, L lightning, S poison,
    /// H holy) can't hurt it.
    pub fn immune_to(&self, letter: char) -> bool {
        let bytes = self.element.as_bytes();
        bytes
            .chunks(2)
            .any(|pair| pair.len() == 2 && pair[0] == letter as u8 && pair[1] == b'1')
    }
}

pub struct MobData {
    nx: NxDataFile,
}

impl MobData {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, GameDataError> {
        Ok(Self {
            nx: NxDataFile::open(path.as_ref())?,
        })
    }

    pub fn info(&self, mob_id: i32) -> Result<Option<MobInfo>, GameDataError> {
        let nx = &self.nx;
        let Some(image) = nx.child_by_name(0, &format!("{mob_id:07}.img"))? else {
            return Ok(None);
        };
        let Some(mut info) = nx.child_by_name(image, "info")? else {
            return Ok(None);
        };
        // Some mobs act as another (info/link) but keep their own stats, as
        // Cosmic reads them; one whose info is only the link takes the
        // other's.
        let mut image = image;
        if let Some(link) = nx.string_child(info, "link")? {
            if let Some(linked) = nx.child_by_name(0, &format!("{link}.img"))? {
                image = linked;
                if nx.child_by_name(info, "maxHP")?.is_none() {
                    if let Some(linked_info) = nx.child_by_name(linked, "info")? {
                        info = linked_info;
                    }
                }
            }
        }
        let int =
            |key: &str| -> Result<i32, GameDataError> { Ok(nx.int_child(info, key)?.unwrap_or(0)) };
        Ok(Some(MobInfo {
            mob_id,
            level: i16::try_from(int("level")?).unwrap_or(0),
            max_hp: int("maxHP")?,
            max_mp: int("maxMP")?,
            exp: int("exp")?,
            attack: int("PADamage")?,
            magic_attack: int("MADamage")?,
            defense: int("PDDamage")?,
            accuracy: int("acc")?,
            avoid: int("eva")?,
            speed: int("speed")?,
            body_attack: int("bodyAttack")? != 0,
            not_attack: int("notAttack")? != 0,
            first_attack: int("firstAttack")? != 0,
            boss: int("boss")? != 0,
            undead: int("undead")? != 0,
            fixed_damage: int("fixedDamage")?,
            invincible: int("invincible")? != 0,
            public_reward: int("publicReward")? != 0,
            explosive_reward: int("explosiveReward")? != 0,
            element: nx.string_child(info, "elemAttr")?.unwrap_or_default(),
            attacks: self.attacks(image)?,
            skills: self.skills(info)?,
            banish: self.banish(info)?,
            revive: self.revive(info)?,
            dying_ms: self.dying_ms(image)?,
            remove_after: int("removeAfter")?,
        }))
    }

    fn revive(&self, info: u32) -> Result<Vec<i32>, GameDataError> {
        let nx = &self.nx;
        let mut revive = Vec::new();
        if let Some(list) = nx.child_by_name(info, "revive")? {
            while let Some(mob_id) = nx.int_child(list, &revive.len().to_string())? {
                revive.push(mob_id);
            }
        }
        Ok(revive)
    }

    fn dying_ms(&self, image: u32) -> Result<i32, GameDataError> {
        let nx = &self.nx;
        let Some(die) = nx.child_by_name(image, "die1")? else {
            return Ok(500);
        };
        let (mut frames, mut total) = (0, 0);
        while let Some(frame) = nx.child_by_name(die, &frames.to_string())? {
            total += nx.int_child(frame, "delay")?.unwrap_or(100);
            frames += 1;
        }
        Ok(if frames == 0 { 500 } else { total })
    }

    fn banish(&self, info: u32) -> Result<Option<Banish>, GameDataError> {
        let nx = &self.nx;
        let Some(ban) = nx.child_by_name(info, "ban")? else {
            return Ok(None);
        };
        let Some(to) = nx
            .child_by_name(ban, "banMap")?
            .map(|maps| nx.child_by_name(maps, "0"))
            .transpose()?
            .flatten()
        else {
            return Ok(None);
        };
        let Some(map_id) = nx.int_child(to, "field")? else {
            return Ok(None);
        };
        Ok(Some(Banish {
            map_id,
            portal: nx.string_child(to, "portal")?.unwrap_or_default(),
            message: nx.string_child(ban, "banMsg")?.unwrap_or_default(),
        }))
    }

    fn skills(&self, info: u32) -> Result<Vec<MobSkillEntry>, GameDataError> {
        let nx = &self.nx;
        let mut skills = Vec::new();
        let Some(list) = nx.child_by_name(info, "skill")? else {
            return Ok(skills);
        };
        while let Some(entry) = nx.child_by_name(list, &skills.len().to_string())? {
            let int = |key: &str| -> Result<i32, GameDataError> {
                Ok(nx.int_child(entry, key)?.unwrap_or(0))
            };
            skills.push(MobSkillEntry {
                skill: int("skill")?,
                level: int("level")?,
                action: int("action")?,
                effect_after: int("effectAfter")?,
            });
        }
        Ok(skills)
    }

    fn attacks(&self, image: u32) -> Result<Vec<MobAttack>, GameDataError> {
        let nx = &self.nx;
        let mut attacks = Vec::new();
        while let Some(attack) = nx.child_by_name(image, &format!("attack{}", attacks.len() + 1))? {
            let Some(info) = nx.child_by_name(attack, "info")? else {
                break;
            };
            let int = |key: &str| -> Result<i32, GameDataError> {
                Ok(nx.int_child(info, key)?.unwrap_or(0))
            };
            attacks.push(MobAttack {
                kind: int("type")?,
                magic: int("magic")? != 0,
                mp_cost: int("conMP")?,
                attack: int("PADamage")?,
                deadly: int("deadlyAttack")? != 0,
                mp_burn: int("mpBurn")?,
                disease: int("disease")?,
                disease_level: int("level")?,
            });
        }
        Ok(attacks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_tutorial_mobs() {
        let Some(path) = crate::test_assets::path("Mob.nx") else {
            return;
        };
        let data = MobData::load(path).unwrap();
        let snail = data.info(100100).unwrap().unwrap();
        assert_eq!(
            (snail.level, snail.max_hp, snail.exp, snail.attack),
            (1, 8, 3, 12)
        );
        assert!(snail.body_attack);
        let sentinel = data.info(9300018).unwrap().unwrap();
        assert!(sentinel.not_attack);
        assert!(data.info(9999998).unwrap().is_none());
    }

    #[test]
    fn reads_where_banish_sends_players() {
        let Some(path) = crate::test_assets::path("Mob.nx") else {
            return;
        };
        let data = MobData::load(path).unwrap();
        // Shade: to Kerning City's subway ticketing booth.
        let banish = data.info(5090000).unwrap().unwrap().banish.unwrap();
        assert_eq!((banish.map_id, banish.portal.as_str()), (103000100, "sp"));
        assert!(banish.message.ends_with("Subway Ticketing Booth."));
        assert_eq!(data.info(100100).unwrap().unwrap().banish, None);
    }

    #[test]
    fn reads_elements_fixed_damage_and_invincible() {
        let Some(path) = crate::test_assets::path("Mob.nx") else {
            return;
        };
        let data = MobData::load(path).unwrap();
        let mob = |id| data.info(id).unwrap().unwrap();
        // Green Mushroom: poison can't hurt it. Red Snail only fears fire.
        let green_mushroom = mob(1110100);
        assert_eq!(green_mushroom.element, "S1");
        assert!(green_mushroom.immune_to('S'));
        assert!(!mob(130100).immune_to('F'));
        assert_eq!(mob(9300295).fixed_damage, 1);
        assert_eq!(mob(100100).fixed_damage, 0);
        assert!(mob(9300295).invincible);
        assert!(!mob(100100).invincible);
        // Bergamot's drops are anyone's; Mano's in Mu Lung Dojo fly out.
        assert!(mob(7220003).public_reward && !mob(7220003).explosive_reward);
        assert!(mob(9300184).explosive_reward);
        assert!(!mob(100100).public_reward && !mob(100100).explosive_reward);
    }

    #[test]
    fn reads_attacks() {
        let Some(path) = crate::test_assets::path("Mob.nx") else {
            return;
        };
        let data = MobData::load(path).unwrap();
        let mob = |id| data.info(id).unwrap().unwrap();
        assert!(mob(100100).attacks.is_empty());
        // Star Pixie: a magic ball for 1 MP.
        let pixie = mob(3230200);
        assert_eq!(pixie.magic_attack, 130);
        assert_eq!(pixie.attacks.len(), 1);
        let ball = &pixie.attacks[0];
        assert_eq!((ball.kind, ball.magic, ball.mp_cost), (2, true, 1));
        // Stumpy's second attack hits harder than its body (125).
        let second = &mob(3220000).attacks[1];
        assert_eq!((second.kind, second.magic, second.attack), (2, false, 135));
    }

    #[test]
    fn linked_mobs_keep_their_own_stats() {
        let Some(path) = crate::test_assets::path("Mob.nx") else {
            return;
        };
        let data = MobData::load(path).unwrap();
        let mob = |id| data.info(id).unwrap().unwrap();
        // Cherry Bubble Tea acts as Red Bubble Tea (level 38, 1900 HP) but
        // is weaker, and without its weak spots.
        let (cherry, red) = (mob(3400000), mob(9410005));
        assert_eq!((cherry.level, cherry.max_hp, cherry.exp), (35, 1400, 80));
        assert_eq!((red.level, red.max_hp), (38, 1900));
        assert_eq!(cherry.element, "");
        assert_eq!(cherry.attacks, red.attacks);
        assert!(!cherry.attacks.is_empty());
        // Nothing but a link: a Wild Boar.
        assert_eq!(mob(9501016).max_hp, mob(2230102).max_hp);
    }

    #[test]
    fn reads_what_comes_out_of_a_dying_mob() {
        let Some(path) = crate::test_assets::path("Mob.nx") else {
            return;
        };
        let data = MobData::load(path).unwrap();
        let mob = |id| data.info(id).unwrap().unwrap();
        // Yeti and Pepe fall apart into a Yeti and a Pepe once its five
        // 120 ms frames of dying are over.
        let pair = mob(7130102);
        assert_eq!(
            (pair.revive.as_slice(), pair.dying_ms),
            (&[6300002, 6130102][..], 600)
        );
        let snail = mob(100100);
        assert!(snail.revive.is_empty());
        assert!(snail.dying_ms > 0);
    }

    #[test]
    fn reads_how_long_a_mob_stays() {
        let Some(path) = crate::test_assets::path("Mob.nx") else {
            return;
        };
        let data = MobData::load(path).unwrap();
        let mob = |id| data.info(id).unwrap().unwrap();
        // Lord Pirate's Devoted Kru; Spirit of Rock's Soul has it as text.
        assert_eq!(mob(9300117).remove_after, 10);
        assert_eq!(mob(4300017).remove_after, 15);
        assert_eq!(mob(100100).remove_after, 0);
    }

    #[test]
    fn reads_skills() {
        let Some(path) = crate::test_assets::path("Mob.nx") else {
            return;
        };
        let data = MobData::load(path).unwrap();
        let mob = |id| data.info(id).unwrap().unwrap();
        assert!(mob(100100).skills.is_empty());
        // Stumpy: area power up after 1.5 seconds, then a level 15 heal.
        let stumpy = mob(3220000).skills;
        assert_eq!(
            stumpy[0],
            MobSkillEntry {
                skill: 112,
                level: 1,
                action: 1,
                effect_after: 1500
            }
        );
        assert_eq!((stumpy[1].skill, stumpy[1].level), (114, 15));
    }

    #[test]
    fn reads_numbers_stored_as_strings() {
        let Some(path) = crate::test_assets::path("Mob.nx") else {
            return;
        };
        let data = MobData::load(path).unwrap();
        // 9400564's maxHP is the string "1000000".
        assert_eq!(data.info(9400564).unwrap().unwrap().max_hp, 1_000_000);
    }

    #[test]
    fn maps_list_their_mob_spawns() {
        let Some(path) = crate::test_assets::path("Map.nx") else {
            return;
        };
        let data = crate::GameData::load_from_nx_map(path).unwrap();
        let hunting_ground = data.field(40000).unwrap();
        assert_eq!(
            hunting_ground
                .mob_spawns
                .iter()
                .filter(|m| m.mob_id == 100100)
                .count(),
            15
        );
        let east_entrance = data.field(30000).unwrap();
        assert_eq!(
            east_entrance
                .mob_spawns
                .iter()
                .filter(|m| m.mob_id == 9300018)
                .count(),
            3
        );
        assert!(data.field(10000).unwrap().mob_spawns.is_empty());
    }
}
