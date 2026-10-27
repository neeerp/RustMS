//! Shining Stone (2166): Sharyl starts it with an empty info string; the
//! Shiny Stone in the Nautilus's Generator Room sets it to "5", once, and
//! Sharyl then completes the quest for 400 EXP.

use game_data::server_content::keys;
use integration_harness::database::{set_job, set_quest};
use integration_harness::events::{maps, said, talk_to};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, build_quest_completion, START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const SHARYLS_ROOM: i32 = 120000300;
const SHARYL: i32 = 1092003;
const GENERATOR_ROOM: i32 = 120000301;
const SHINY_STONE: i32 = 1092016;
const SHINING_STONE: i16 = 2166;

/// NPC `npc` on `map`: its object id, and a walk to it.
fn npc(map: i32, npc: i32) -> (i32, Packet) {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let (index, at) = (npcs.iter().enumerate())
        .find(|(_, n)| n.npc_id == npc)
        .unwrap();
    (1_000_000_000 + index as i32, walk_to(at.x, at.y))
}

#[tokio::test]
async fn the_shiny_stone_marks_shining_stone_for_sharyl() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 20, 50).await.unwrap();
    set_quest(id, 2165, 2, "").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, SHARYLS_ROOM, "", &mut trace).await, SHARYLS_ROOM);
    send(c, npc(SHARYLS_ROOM, SHARYL).1).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(START, SHINING_STONE, SHARYL)).await;
    assert_eq!(t.states.get(&SHINING_STONE), Some(&1));
    // Not yet: the string is still empty.
    let t = step(c, &m, build_quest_completion(SHINING_STONE, SHARYL, -1)).await;
    assert!(t.states.is_empty() && t.gains.is_empty(), "{t:?}");

    assert_eq!(
        warp(c, GENERATOR_ROOM, "", &mut trace).await,
        GENERATOR_ROOM
    );
    let (stone, walk) = npc(GENERATOR_ROOM, SHINY_STONE);
    send(c, walk).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, talk_to(stone)).await;
    assert_eq!(t.kills[&SHINING_STONE], "5");
    assert!(said(&t, keys::QUEST_INFO_SHINY_STONE));
    step(c, &m, answer(0, 1, None)).await;
    // Once only.
    let t = step(c, &m, talk_to(stone)).await;
    assert!(t.kills.is_empty(), "{t:?}");
    assert!(!said(&t, keys::QUEST_INFO_SHINY_STONE));
    step(c, &m, answer(0, 1, None)).await;

    assert_eq!(warp(c, SHARYLS_ROOM, "", &mut trace).await, SHARYLS_ROOM);
    send(c, npc(SHARYLS_ROOM, SHARYL).1).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_completion(SHINING_STONE, SHARYL, -1)).await;
    assert_eq!(t.states.get(&SHINING_STONE), Some(&2));
    assert_eq!(t.gains, [400]);
}
