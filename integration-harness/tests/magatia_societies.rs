//! Joining Magatia's societies: Han the Broker gets Zenumist's (3301) and
//! Alcadno's (3303) tests ready for two of a jewel ore the player picks,
//! and a society's head makes a lost cape again (3305, 3306) for the
//! data's jewels, Leather and mesos.

use integration_harness::database::{grant_stack, set_job, set_meso, set_quest};
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

const MAGATIA: i32 = 261000000;
const ZENUMIST_SOCIETY: i32 = 261000010;
const ALCADNO_SOCIETY: i32 = 261000020;
const HAN_THE_BROKER: i32 = 2111007;
const CARSON: i32 = 2111000;
const MAED: i32 = 2111001;
const AMETHYST_ORE: i32 = 4020002;
const AQUAMARINE_ORE: i32 = 4020003;
const EMERALD: i32 = 4021003;
const LEATHER: i32 = 4000021;
const ZENUMIST_CAPE: i32 = 1102135;

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

#[tokio::test]
async fn han_the_broker_and_the_society_capes() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 30, 50).await.unwrap();
    set_meso(id, 10_000).await.unwrap();
    // A Deal with the Broker, and both capes once had.
    set_quest(id, 3300, 2, "").await.unwrap();
    set_quest(id, 3302, 2, "").await.unwrap();
    set_quest(id, 3304, 2, "").await.unwrap();
    grant_stack(id, 4, 1, AMETHYST_ORE, 2).await.unwrap();
    grant_stack(id, 4, 2, AQUAMARINE_ORE, 3).await.unwrap();
    grant_stack(id, 4, 3, EMERALD, 5).await.unwrap();
    grant_stack(id, 4, 4, LEATHER, 10).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(warp(c, MAGATIA, "", &mut trace).await, MAGATIA);
    send(c, to(MAGATIA, HAN_THE_BROKER)).await;
    barrier(c, &m, &mut trace).await;
    for (quest, ore) in [(3301, AMETHYST_ORE), (3303, AQUAMARINE_ORE)] {
        let t = step(c, &m, build_quest_action(START, quest, HAN_THE_BROKER)).await;
        assert_eq!(t.states.get(&quest), Some(&1));
        let t = step(c, &m, build_quest_action(SCRIPT_END, quest, HAN_THE_BROKER)).await;
        assert_eq!(t.prompts, [(HAN_THE_BROKER, 4)]);
        // The ores held two of, in id order: the first is taken.
        let t = step(c, &m, answer(4, 1, Some(0))).await;
        assert_eq!(t.items, [(ore, -2)]);
        assert_eq!(t.states.get(&quest), Some(&2));
    }

    assert_eq!(
        warp(c, ZENUMIST_SOCIETY, "", &mut trace).await,
        ZENUMIST_SOCIETY
    );
    send(c, to(ZENUMIST_SOCIETY, CARSON)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(SCRIPT_START, 3305, CARSON)).await;
    assert_eq!(t.prompts, [(CARSON, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(CARSON, 1)]);
    step(c, &m, answer(1, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.progress, [(3305, 1)]);
    let t = step(c, &m, build_quest_completion(3305, CARSON, -1)).await;
    assert_eq!(t.states.get(&3305), Some(&2));
    assert_eq!(t.meso_gains, [-10_000]);
    let mut items = t.items.clone();
    items.sort();
    assert_eq!(items, [(ZENUMIST_CAPE, 1), (LEATHER, -10), (EMERALD, -5)]);

    // Maed asks too; no is the end of it.
    assert_eq!(
        warp(c, ALCADNO_SOCIETY, "", &mut trace).await,
        ALCADNO_SOCIETY
    );
    send(c, to(ALCADNO_SOCIETY, MAED)).await;
    barrier(c, &m, &mut trace).await;
    step(c, &m, build_quest_action(SCRIPT_START, 3306, MAED)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(MAED, 1)]);
    let t = step(c, &m, answer(1, 0, None)).await;
    assert!(t.progress.is_empty() && t.prompts.is_empty(), "{t:?}");
    step(c, &m, build_quest_action(SCRIPT_START, 3306, MAED)).await;
    step(c, &m, answer(0, 1, None)).await;
    step(c, &m, answer(1, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.progress, [(3306, 1)]);
}
