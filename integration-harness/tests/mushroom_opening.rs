//! The Mushroom Kingdom's opening: the Empty House's portal takes a level-30
//! player who hasn't seen it to its scene's map, whose script marks it seen
//! (record 2311), locks the UI and plays the scene; the scene's end asks for
//! the Mushroom Forest Field and gets the UI back there. Only then does The
//! Test (2312) start, and its officer completes it for 50 Mutated Spores.

use integration_harness::database::{grant_stack, set_job};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{build_quest_completion, START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const EMPTY_HOUSE: i32 = 100000002;
const OPENING: i32 = 106020001;
const MUSHROOM_FOREST_FIELD: i32 = 106020000;
const OPENING_RECORD: i16 = 2311;
const THE_TEST: i16 = 2312;
const OFFICER: i32 = 1300005;
const MUTATED_SPORE: i32 = 4000499;

/// CHANGE_MAP_SPECIAL: walk into the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// `QUEST_ACTION` start as v83 sends it for a quest its alert offers: no
/// position.
fn alert_start(quest: i16, npc: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::QuestAction as i16).unwrap();
    p.write_byte(START).unwrap();
    p.write_short(quest).unwrap();
    p.write_int(npc).unwrap();
    p
}

/// CHANGE_MAP by id, as the client asks for a scene's field when it ends.
fn scene_end(field: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMap as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_int(field).unwrap();
    p.write_str_with_length("").unwrap();
    p.write_byte(0).unwrap();
    p.write_byte(0).unwrap();
    p
}

#[tokio::test]
async fn the_opening_scene_comes_before_the_test() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 30, 50).await.unwrap();
    grant_stack(id, 4, 1, MUTATED_SPORE, 50).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    // Not before the scene.
    let t = step(c, &m, alert_start(THE_TEST, OFFICER)).await;
    assert!(t.states.is_empty(), "{t:?}");

    assert_eq!(warp(c, EMPTY_HOUSE, "", &mut trace).await, EMPTY_HOUSE);
    let t = step(c, &m, enter("TD00")).await;
    assert_eq!(t.warps, [OPENING]);
    assert_eq!(t.kills[&OPENING_RECORD], "1");
    assert_eq!(t.ui_locks, [true]);
    assert_eq!(t.scene_parts.len(), 10);
    assert_eq!(t.scene_parts[0], "Effect/Direction2.img/open/back0");
    assert_eq!(t.scene_parts[9], "Effect/Direction2.img/open/out");

    let t = step(c, &m, scene_end(MUSHROOM_FOREST_FIELD)).await;
    assert_eq!(t.warps, [MUSHROOM_FOREST_FIELD]);
    assert_eq!(t.ui_locks, [false]);

    let t = step(c, &m, alert_start(THE_TEST, OFFICER)).await;
    assert_eq!(t.states.get(&THE_TEST), Some(&1));

    let npcs = &maps().field(MUSHROOM_FOREST_FIELD).unwrap().map_npcs;
    let officer = npcs.iter().find(|n| n.npc_id == OFFICER).unwrap();
    send(c, walk_to(officer.x, officer.y)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_completion(THE_TEST, OFFICER, -1)).await;
    assert_eq!(t.states.get(&THE_TEST), Some(&2));
    assert_eq!(t.gains, [11_500]);
    assert_eq!(t.items, [(MUTATED_SPORE, -50)]);
}
