//! Tienk's greeting (2197): Tienk's end script completes the quest for a
//! new Beginner in Lith Harbor. The Last Song (2293): Maestro Rho's start
//! script starts it, and the data's end completes it with nothing given.

use integration_harness::database::{set_job, set_quest};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_END, SCRIPT_START, START,
};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const LITH_HARBOR: i32 = 104000000;
const TIENK: i32 = 2006;
const TIENKS_GREETING: i16 = 2197;
const SUBWAY: i32 = 103040000;
const MAESTRO_RHO: i32 = 1052120;
const THE_LAST_SONG: i16 = 2293;

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

#[tokio::test]
async fn tienk_completes_the_greeting() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 5, 50).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(warp(c, LITH_HARBOR, "", &mut trace).await, LITH_HARBOR);
    send(c, to(LITH_HARBOR, TIENK)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(START, TIENKS_GREETING, TIENK)).await;
    assert_eq!(t.states.get(&TIENKS_GREETING), Some(&1));
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_END, TIENKS_GREETING, TIENK),
    )
    .await;
    assert_eq!(t.prompts, [(TIENK, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&TIENKS_GREETING), Some(&2));
    assert!(t.gains.is_empty() && t.items.is_empty(), "{t:?}");
}

#[tokio::test]
async fn maestro_rho_starts_the_last_song() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 40, 50).await.unwrap();
    set_quest(id, 2288, 2, "").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(warp(c, SUBWAY, "", &mut trace).await, SUBWAY);
    send(c, to(SUBWAY, MAESTRO_RHO)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_START, THE_LAST_SONG, MAESTRO_RHO),
    )
    .await;
    assert_eq!(t.prompts, [(MAESTRO_RHO, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&THE_LAST_SONG), Some(&1));
    let t = step(
        c,
        &m,
        build_quest_completion(THE_LAST_SONG, MAESTRO_RHO, -1),
    )
    .await;
    assert_eq!(t.states.get(&THE_LAST_SONG), Some(&2));
    assert!(t.gains.is_empty(), "{t:?}");
}
