//! New Leaf City's Masteria start scripts: Lita Lawless's bounty (8231)
//! is turned down, then taken and completed by the data's end, which opens
//! its repeatable follow-up (8232); Taggrin starts The Fallen Woods (8224).

use integration_harness::database::{grant_stack, set_job};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_START,
};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const NEW_LEAF_CITY: i32 = 600000000;
const LITA_LAWLESS: i32 = 9201054;
const FOOLS_GOLD: i16 = 8231;
const FOOLS_GOLD_AGAIN: i16 = 8232;
const LEPRECHAUN_GOLD: i32 = 4032031;
const PHANTOM_FOREST: i32 = 610010004;
const TAGGRIN: i32 = 9201100;
const THE_FALLEN_WOODS: i16 = 8224;

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

#[tokio::test]
async fn lita_and_taggrin_start_their_quests() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 45, 50).await.unwrap();
    grant_stack(id, 4, 1, LEPRECHAUN_GOLD, 30).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(warp(c, NEW_LEAF_CITY, "", &mut trace).await, NEW_LEAF_CITY);
    send(c, to(NEW_LEAF_CITY, LITA_LAWLESS)).await;
    barrier(c, &m, &mut trace).await;
    // Turned down: nothing starts.
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_START, FOOLS_GOLD, LITA_LAWLESS),
    )
    .await;
    assert_eq!(t.prompts, [(LITA_LAWLESS, 1)]);
    let t = step(c, &m, answer(1, 0, None)).await;
    assert_eq!(t.prompts, [(LITA_LAWLESS, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.states.is_empty(), "{t:?}");

    step(
        c,
        &m,
        build_quest_action(SCRIPT_START, FOOLS_GOLD, LITA_LAWLESS),
    )
    .await;
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.prompts, [(LITA_LAWLESS, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&FOOLS_GOLD), Some(&1));
    let t = step(c, &m, build_quest_completion(FOOLS_GOLD, LITA_LAWLESS, -1)).await;
    assert_eq!(t.states.get(&FOOLS_GOLD), Some(&2));
    assert_eq!(t.gains, [50000]);
    assert_eq!(t.meso_gains, [100000]);
    assert_eq!(t.items, [(LEPRECHAUN_GOLD, -30)]);

    step(
        c,
        &m,
        build_quest_action(SCRIPT_START, FOOLS_GOLD_AGAIN, LITA_LAWLESS),
    )
    .await;
    step(c, &m, answer(1, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&FOOLS_GOLD_AGAIN), Some(&1));

    assert_eq!(
        warp(c, PHANTOM_FOREST, "", &mut trace).await,
        PHANTOM_FOREST
    );
    send(c, to(PHANTOM_FOREST, TAGGRIN)).await;
    barrier(c, &m, &mut trace).await;
    step(
        c,
        &m,
        build_quest_action(SCRIPT_START, THE_FALLEN_WOODS, TAGGRIN),
    )
    .await;
    step(c, &m, answer(1, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&THE_FALLEN_WOODS), Some(&1));
}
