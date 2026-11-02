//! About the Mushroom Castle...: The Rememberer takes the Recording Charms
//! of Zombie Mushroom Signal 3 (2251) and sends the player to Karcasa
//! (2254); Karcasa sends them on to the Camel Cab (2257), who has them
//! clear Meerkats against the clock (2258), talks only at Sahel 1 (2259),
//! and at level 30 tells the way to the castle (2260).

use integration_harness::database::{grant_stack, set_job, set_quest};
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

const REMEMBERERS_MAP: i32 = 105040300;
const KARCASAS_MAP: i32 = 260010600;
const OUTSIDE_ARIANT: i32 = 260020000;
const SAHEL_1: i32 = 260020700;
const REMEMBERER: i32 = 1061011;
const KARCASA: i32 = 2101013;
const CAMEL_CAB: i32 = 2110005;
const RECORDING_CHARM: i32 = 4032399;

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

#[tokio::test]
async fn the_camel_cab_tells_the_way_to_the_mushroom_castle() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 30, 50).await.unwrap();
    set_quest(id, 2250, 2, "").await.unwrap();
    grant_stack(id, 4, 1, RECORDING_CHARM, 20).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(
        warp(c, REMEMBERERS_MAP, "", &mut trace).await,
        REMEMBERERS_MAP
    );
    send(c, to(REMEMBERERS_MAP, REMEMBERER)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(START, 2251, REMEMBERER)).await;
    assert_eq!(t.states.get(&2251), Some(&1));
    let t = step(c, &m, build_quest_action(SCRIPT_END, 2251, REMEMBERER)).await;
    assert_eq!(t.prompts, [(REMEMBERER, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.items, [(RECORDING_CHARM, -20)]);
    assert_eq!(t.states.get(&2251), Some(&2));
    assert_eq!(t.gains, [8000]);

    let t = step(c, &m, build_quest_action(SCRIPT_START, 2254, REMEMBERER)).await;
    assert_eq!(t.prompts, [(REMEMBERER, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.progress, [(2254, 1)]);

    assert_eq!(warp(c, KARCASAS_MAP, "", &mut trace).await, KARCASAS_MAP);
    send(c, to(KARCASAS_MAP, KARCASA)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_completion(2254, KARCASA, -1)).await;
    assert_eq!(t.states.get(&2254), Some(&2));
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Karcasa's Memory 1 and 2 are the data's.
    set_quest(id, 2255, 2, "").await.unwrap();
    set_quest(id, 2256, 2, "").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, KARCASAS_MAP, "", &mut trace).await, KARCASAS_MAP);
    send(c, to(KARCASAS_MAP, KARCASA)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(START, 2257, KARCASA)).await;
    assert_eq!(t.states.get(&2257), Some(&1));

    assert_eq!(
        warp(c, OUTSIDE_ARIANT, "", &mut trace).await,
        OUTSIDE_ARIANT
    );
    send(c, to(OUTSIDE_ARIANT, CAMEL_CAB)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(SCRIPT_END, 2257, CAMEL_CAB)).await;
    assert_eq!(t.prompts, [(CAMEL_CAB, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&2257), Some(&2));

    // The Meerkats: asked, then started with its clock.
    let t = step(c, &m, build_quest_action(SCRIPT_START, 2258, CAMEL_CAB)).await;
    assert_eq!(t.prompts, [(CAMEL_CAB, 12)]);
    step(c, &m, answer(12, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.progress, [(2258, 1)]);
    assert_eq!(
        t.quest_clocks
            .iter()
            .map(|c| (c.0, c.1))
            .collect::<Vec<_>>(),
        [(6, 2258)]
    );
    // Not without the 40 Meerkats.
    let t = step(c, &m, build_quest_action(SCRIPT_END, 2258, CAMEL_CAB)).await;
    assert!(t.prompts.is_empty() && t.states.is_empty(), "{t:?}");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Leaving gives up the timed quest; the Meerkats are taken as done.
    set_quest(id, 2258, 2, "").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(
        warp(c, OUTSIDE_ARIANT, "", &mut trace).await,
        OUTSIDE_ARIANT
    );
    send(c, to(OUTSIDE_ARIANT, CAMEL_CAB)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(SCRIPT_START, 2259, CAMEL_CAB)).await;
    assert_eq!(t.prompts, [(CAMEL_CAB, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.progress, [(2259, 1)]);
    // Not where the Meerkats are.
    step(c, &m, build_quest_action(SCRIPT_END, 2259, CAMEL_CAB)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.states.is_empty(), "{t:?}");

    assert_eq!(warp(c, SAHEL_1, "", &mut trace).await, SAHEL_1);
    send(c, to(SAHEL_1, CAMEL_CAB)).await;
    barrier(c, &m, &mut trace).await;
    step(c, &m, build_quest_action(SCRIPT_END, 2259, CAMEL_CAB)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&2259), Some(&2));

    step(c, &m, build_quest_action(SCRIPT_START, 2260, CAMEL_CAB)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.progress, [(2260, 1)]);
    step(c, &m, build_quest_action(SCRIPT_END, 2260, CAMEL_CAB)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&2260), Some(&2));
}
