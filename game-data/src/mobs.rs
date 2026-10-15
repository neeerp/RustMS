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
        // Some mobs reuse another mob's data through info/link.
        let Some(mut info) = nx.child_by_name(image, "info")? else {
            return Ok(None);
        };
        if let Some(link) = nx.string_child(info, "link")? {
            if let Some(linked) = nx.child_by_name(0, &format!("{link}.img"))? {
                if let Some(linked_info) = nx.child_by_name(linked, "info")? {
                    info = linked_info;
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
            defense: int("PDDamage")?,
            accuracy: int("acc")?,
            avoid: int("eva")?,
            speed: int("speed")?,
            body_attack: int("bodyAttack")? != 0,
            not_attack: int("notAttack")? != 0,
            first_attack: int("firstAttack")? != 0,
            boss: int("boss")? != 0,
            undead: int("undead")? != 0,
        }))
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
