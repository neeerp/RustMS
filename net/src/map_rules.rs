//! The level rules of maps (Map.nx `info/lvLimit` and `info/lvForceMove`).
//! The v83 client reads neither, so the server enforces them; nothing shows
//! how a v83 server did, so these follow the recommended defaults in
//! `v83ref/lanes/QUESTIONS.md` (2026-10-22, row 196). And the portals whose
//! link the map data lost (`lost_link`).

use game::Player;
use game_data::FieldTemplate;

/// The Highlands (Masteria), whose floor is a row of touch portals, nearly all
/// naming a portal the Krakian Jungle Basin lacks.
const HIGHLANDS: i32 = 600010600;

/// Portals whose target map lacks the spawn portal they name, so the target's
/// default spawn stands in, as Cosmic's `GenericPortal` has it. Each is a way
/// a player can reach and, for the rooms, often the only way out on foot. The
/// rest of what the data lost stays closed (QUESTIONS b441): maps nothing
/// leads to, event rooms, the sky ferries' touch portals.
const LOST_LINKS: [(i32, &str); 30] = [
    // Rooms' ways out.
    (crate::reactors::BART_ROOM, "ntq00"),
    (crate::scripts::ARDINS_ROOM, "st00"),
    (crate::scripts::DISCIPLES_ROOM, "out00"),
    (crate::scripts::SEALED_SHRINE, "out00"),
    (crate::scripts::WOLVES_DEN, "out00"),
    (923000000, "out00"),
    (926120100, "out00"),
    (926120300, "out00"),
    (680000004, "out01"),
    (300000100, "out00"),
    (crate::events::CWK_LOBBY, "out00"),
    (crate::guild_quest::WAITING_ROOM, "st00"),
    // Walks between fields.
    (550000000, "west00"),
    (106020400, "left00"),
    (106021401, "out00"),
    (610010000, "U1_3"),
    (610010002, "U3_2"),
    (610010005, "U6_2"),
    (610020005, "CM6_2"),
    // The Haunted House's rooms and chimneys.
    (682000200, "rmA00"),
    (682000200, "rmA01"),
    (682000200, "rmB00"),
    (682000200, "rmB01"),
    (682000200, "rmC00"),
    (682000200, "rmD03"),
    (682000200, "rmF00"),
    (682000200, "rmL01"),
    (682000406, "lib00"),
    (682000900, "st00"),
    (682010200, "out00"),
];

/// Whether `map_id`'s `portal`, when the target map lacks the portal it names,
/// leads to that map's default spawn.
pub fn lost_link(map_id: i32, portal: &str) -> bool {
    LOST_LINKS.contains(&(map_id, portal)) || (map_id == HIGHLANDS && portal.starts_with('g'))
}

/// The five Training Centers (`910060000`, ... five maps each), whose
/// `lvForceMove` 20 matches their entrance NPCs' "only for those under level
/// 20" (as in Cosmic's scripts). The other maps with the key (Kerning
/// Square's 51, the Mushroom Castle's 39) have no second source and are left
/// alone.
const TRAINING_CENTERS: [i32; 5] = [910060000, 910120000, 910220000, 910310000, 912030000];

fn training_center(map_id: i32) -> bool {
    TRAINING_CENTERS.contains(&(map_id - map_id % 100))
}

/// Whether `player` may walk into `field` through a portal: they need its
/// `lvLimit`, unless they are a GM. Other ways in (NPCs, scripts, rocks,
/// map-id warps) are not checked.
pub fn may_walk_in(player: &Player, field: &FieldTemplate) -> bool {
    player.gm_level > 0
        || field
            .level_limit
            .is_none_or(|level| player.stats.level >= level)
}

/// Where `player`, having entered `field`, is moved out to: a Training
/// Center's `returnMap` once they reach its `lvForceMove`, unless they are a
/// GM.
pub fn moved_out_to(player: &Player, field: &FieldTemplate) -> Option<i32> {
    let level = field.level_force_move?;
    if player.gm_level > 0 || player.stats.level < level || !training_center(field.map_id) {
        return None;
    }
    field.return_map.filter(|&map_id| map_id != field.map_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A listed portal's target map has to be there, with a default spawn,
    /// and must still lack the portal it names: a link the data has again
    /// doesn't belong on the list.
    #[test]
    fn a_lost_link_names_a_portal_its_map_lacks() {
        let data = crate::game_data::get().unwrap();
        for &(map_id, name) in &LOST_LINKS {
            assert!(lost_link(map_id, name));
            let field = data.field(map_id).unwrap();
            let portal = field.portal_by_name(name).unwrap();
            let target = data.field(portal.to_map.unwrap()).unwrap();
            assert!(
                target.resolve_spawn_portal(&portal.to_name).is_none(),
                "{map_id} {name}"
            );
            assert!(target.resolve_spawn_portal("").is_some());
        }
    }

    /// The floor's touch portals lead back to the Basin, whose spawn stands in
    /// for the portal they name, and no other portal of the map is taken for
    /// one.
    #[test]
    fn the_highlands_floor_leads_back_to_the_basin() {
        let data = crate::game_data::get().unwrap();
        let highlands = data.field(HIGHLANDS).unwrap();
        let basin = data.field(600010200).unwrap();
        let floor = highlands
            .portals()
            .filter(|portal| portal.to_map == Some(600010200))
            .collect::<Vec<_>>();
        assert_eq!(floor.len(), 46);
        for portal in floor {
            assert!(lost_link(HIGHLANDS, &portal.name));
            assert!(basin.resolve_spawn_portal(&portal.to_name).is_none());
        }
        assert!(basin.resolve_spawn_portal("").is_some());
        assert!(!lost_link(HIGHLANDS, "st00") && !lost_link(HIGHLANDS, "in00"));
    }

    #[test]
    fn a_training_center_moves_out_players_from_level_20() {
        let data = crate::game_data::get().unwrap();
        let mut centers = data
            .field_ids()
            .filter(|&map_id| data.field(map_id).unwrap().level_force_move == Some(20))
            .collect::<Vec<_>>();
        centers.sort_unstable();
        assert_eq!(centers.len(), 25);
        assert!(centers.iter().all(|&map_id| training_center(map_id)));

        let center = data.field(910060000).unwrap();
        let mut player = crate::codec::stats::sample_player();
        player.stats.level = 19;
        assert_eq!(moved_out_to(&player, center), None);
        player.stats.level = 20;
        let town = moved_out_to(&player, center).unwrap();
        assert_eq!(Some(town), center.return_map);
        player.gm_level = 1;
        assert_eq!(moved_out_to(&player, center), None);
        // Kerning Square's 51 is not applied.
        player.gm_level = 0;
        player.stats.level = 60;
        assert_eq!(moved_out_to(&player, data.field(103040100).unwrap()), None);
    }

    #[test]
    fn walking_in_needs_the_maps_level() {
        let data = crate::game_data::get().unwrap();
        let forest = data.field(240010000).unwrap();
        let leafre = data.field(240000000).unwrap();
        assert_eq!(
            leafre.portal_by_name("west00").unwrap().to_map,
            Some(240010000)
        );
        let mut player = crate::codec::stats::sample_player();
        player.stats.level = 14;
        assert!(!may_walk_in(&player, forest));
        assert!(may_walk_in(&player, data.field(240000000).unwrap()));
        player.gm_level = 1;
        assert!(may_walk_in(&player, forest));
        player.gm_level = 0;
        player.stats.level = 15;
        assert!(may_walk_in(&player, forest));
    }
}
