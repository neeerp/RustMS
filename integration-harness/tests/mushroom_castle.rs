//! The way into the Mushroom Kingdom: the Empty House's portal, the
//! poisonous mushrooms that a Poison Spore clears, and the castle wall's
//! thorns keep players back with a pink line until their level, quests (or
//! the spore) let them through.

use integration_harness::database::{grant_stack, set_job, set_quest};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{step, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const EMPTY_HOUSE: i32 = 100000002;
const MUSHROOM_FOREST_FIELD: i32 = 106020000;
const DEEP_INSIDE: i32 = 106020300;
const SPLIT_ROAD: i32 = 106020400;
const CASTLE_WALL_EDGE: i32 = 106020501;
const POISON_SPORE: i32 = 4000507;
const PINK: u8 = 5;

/// CHANGE_MAP_SPECIAL: walk into, or press Up on, the scripted portal
/// `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// Go to `map` and try its portal `portal`; returns what came back.
async fn try_portal(player: &mut WorldSession, map: i32, portal: &str) -> Trace {
    let mut trace = Trace::default();
    assert_eq!(warp(&mut player.connection, map, "", &mut trace).await, map);
    let name = player.character_name.clone();
    step(&mut player.connection, &name, enter(portal)).await
}

fn refused(trace: &Trace) {
    assert!(trace.warps.is_empty(), "{:?}", trace.warps);
    assert!(
        matches!(trace.broadcasts.as_slice(), [(PINK, text)] if !text.is_empty()),
        "{:?}",
        trace.broadcasts
    );
}

#[tokio::test]
async fn the_mushroom_kingdom_lets_players_through_once_they_may_go() {
    let config = load_harness_config_or_fail().await;
    let mut player = login_to_world_session(&config).await.unwrap();
    let id = player.character_id;

    refused(&try_portal(&mut player, EMPTY_HOUSE, "TD00").await);
    refused(&try_portal(&mut player, DEEP_INSIDE, "obstacle").await);
    refused(&try_portal(&mut player, SPLIT_ROAD, "right00").await);

    drop(player);
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    // Level 30, and the kingdom's opening scene already seen (record 2311;
    // `mushroom_opening` covers the first time).
    set_job(id, 0, 30, 50).await.unwrap();
    set_quest(id, 2311, 1, "1").await.unwrap();
    set_quest(id, 2324, 2, "").await.unwrap();
    grant_stack(id, 4, 1, POISON_SPORE, 1).await.unwrap();
    let mut player = relogin(&config).await;

    let trace = try_portal(&mut player, EMPTY_HOUSE, "TD00").await;
    assert_eq!(trace.warps, [MUSHROOM_FOREST_FIELD]);
    assert_eq!(trace.portal_sounds, 1);

    // The spore is used up on the way.
    let trace = try_portal(&mut player, DEEP_INSIDE, "obstacle").await;
    assert_eq!(trace.warps, [SPLIT_ROAD]);
    assert_eq!(trace.removed, [(4, 1)]);
    assert!(
        matches!(trace.broadcasts.as_slice(), [(PINK, _)]),
        "{:?}",
        trace.broadcasts
    );
    refused(&try_portal(&mut player, DEEP_INSIDE, "obstacle").await);

    let trace = try_portal(&mut player, SPLIT_ROAD, "right00").await;
    assert_eq!(trace.warps, [CASTLE_WALL_EDGE]);
}
