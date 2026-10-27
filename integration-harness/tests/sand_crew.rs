//! The way into the Sand Crew: Manji's To the Desert... (2127) ends in his
//! script, opening A Special Assignment (2123); Moppie hands over a box
//! (2124), and Eleska's end script in Ariant takes it.

use integration_harness::database::{set_job, set_quest};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_END, START,
};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const PERION: i32 = 102000000;
const ORBIS: i32 = 200000000;
const ARIANT_HOUSE: i32 = 260000200;
const MANJI: i32 = 1022002;
const MOPPIE: i32 = 2012019;
const ELESKA: i32 = 2101002;
const MOPPIES_BOX: i32 = 4031619;

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

#[tokio::test]
async fn manji_moppie_and_eleska_lead_into_the_sand_crew() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 20, 50).await.unwrap();
    set_quest(id, 2122, 2, "").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(warp(c, PERION, "", &mut trace).await, PERION);
    send(c, to(PERION, MANJI)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(START, 2127, MANJI)).await;
    assert_eq!(t.states.get(&2127), Some(&1));
    let t = step(c, &m, build_quest_action(SCRIPT_END, 2127, MANJI)).await;
    assert_eq!(t.prompts, [(MANJI, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&2127), Some(&2));
    let t = step(c, &m, build_quest_action(START, 2123, MANJI)).await;
    assert_eq!(t.states.get(&2123), Some(&1));

    assert_eq!(warp(c, ORBIS, "", &mut trace).await, ORBIS);
    send(c, to(ORBIS, MOPPIE)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_completion(2123, MOPPIE, -1)).await;
    assert_eq!(t.states.get(&2123), Some(&2));
    let t = step(c, &m, build_quest_action(START, 2124, MOPPIE)).await;
    assert_eq!(t.states.get(&2124), Some(&1));
    assert_eq!(t.items, [(MOPPIES_BOX, 1)]);

    assert_eq!(warp(c, ARIANT_HOUSE, "", &mut trace).await, ARIANT_HOUSE);
    send(c, to(ARIANT_HOUSE, ELESKA)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(SCRIPT_END, 2124, ELESKA)).await;
    assert_eq!(t.prompts, [(ELESKA, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.items, [(MOPPIES_BOX, -1)]);
    assert_eq!(t.states.get(&2124), Some(&2));
}
