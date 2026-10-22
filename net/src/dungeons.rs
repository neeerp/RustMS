//! Mini-dungeons: a base map's `MD00` portal takes a player, or a party's
//! leader with the members beside them, into the first free copy of the
//! dungeon, and the copy's `out00` takes them back. The copies are maps of
//! their own that share one body through `info/link`, and carry the time
//! limit and the way back (`forcedReturn`) in their own `info`.

use crate::content;
use game_data::server_content::keys;

/// One mini-dungeon and the maps it is played in.
#[derive(Debug, PartialEq, Eq)]
pub struct Dungeon {
    /// The portal script of both ways, in and out.
    pub script: &'static str,
    /// The map whose `MD00` leads in.
    pub base: i32,
    pub first: i32,
    /// How many copies there are: as many as Cosmic offers, but never more
    /// than Map.nx has.
    pub count: i32,
}

const fn dungeon(script: &'static str, base: i32, first: i32, count: i32) -> Dungeon {
    Dungeon {
        script,
        base,
        first,
        count,
    }
}

pub const DUNGEONS: [Dungeon; 12] = [
    dungeon("MD_pig", 100020000, 100020100, 30),
    dungeon("MD_golem", 105040304, 105040320, 30),
    dungeon("MD_mushroom", 105050100, 105050101, 30),
    dungeon("MD_drakeroom", 105090311, 105090320, 20),
    dungeon("MD_rabbit", 221023400, 221023401, 30),
    dungeon("MD_roundTable", 240020500, 240020512, 19),
    dungeon("MD_remember", 240040511, 240040800, 19),
    dungeon("MD_protect", 240040520, 240040900, 19),
    dungeon("MD_treasure", 251010402, 251010410, 20),
    dungeon("MD_sand", 260020600, 260020630, 34),
    dungeon("MD_error", 261020300, 261020301, 30),
    dungeon("MD_high", 551030000, 551030001, 19),
];

impl Dungeon {
    pub fn copies(&self) -> std::ops::Range<i32> {
        self.first..self.first + self.count
    }
}

pub fn by_script(script: &str) -> Option<&'static Dungeon> {
    DUNGEONS.iter().find(|dungeon| dungeon.script == script)
}

/// The dungeon `map_id` is a copy of.
pub fn of_copy(map_id: i32) -> Option<&'static Dungeon> {
    DUNGEONS
        .iter()
        .find(|dungeon| dungeon.copies().contains(&map_id))
}

/// Why a player can't go in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// They are in a party they don't lead.
    NotLeader,
    /// Every copy is taken.
    Full,
}

impl Refusal {
    /// What the portal says, in pink.
    pub fn text(self) -> String {
        let key = match self {
            Refusal::NotLeader => keys::DUNGEON_NOT_LEADER,
            Refusal::Full => keys::DUNGEON_FULL,
        };
        content::text().get(key, &[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_copy_is_in_the_map_data_with_its_way_out() {
        let data = crate::get_game_data().unwrap();
        for dungeon in &DUNGEONS {
            let base = data.field(dungeon.base).unwrap();
            let way_in = base.portal_by_name("MD00").unwrap();
            assert_eq!(way_in.script.as_deref(), Some(dungeon.script));
            for copy in dungeon.copies() {
                let field = data.field(copy).unwrap_or_else(|| panic!("{copy}"));
                let way_out = field.portal_by_name("out00").unwrap();
                assert_eq!(way_out.script.as_deref(), Some(dungeon.script), "{copy}");
                assert_eq!(field.forced_return, Some(dungeon.base), "{copy}");
                assert!(field.time_limit.is_some(), "{copy}");
            }
            assert_eq!(of_copy(dungeon.first), Some(dungeon));
            assert_eq!(of_copy(dungeon.base), None);
        }
    }
}
