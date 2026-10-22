//! The level rules of maps (Map.nx `info/lvLimit` and `info/lvForceMove`).
//! The v83 client reads neither, so the server enforces them; nothing shows
//! how a v83 server did, so these follow the recommended defaults in
//! `v83ref/lanes/QUESTIONS.md` (2026-10-22, row 196).

use game::Player;
use game_data::FieldTemplate;

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
