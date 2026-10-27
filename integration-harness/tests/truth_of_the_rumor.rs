//! Truth of the Rumor (2144): Winston asks around about the old tree; each
//! of five people in Perion and Ellinia completes their own quest (2148 to
//! 2152) as it starts, and with all five Winston completes 2144.

use integration_harness::database::set_job;
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_START, START,
};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const WINSTONS_MAP: i32 = 101030401;
const WINSTON: i32 = 1022006;
const TRUTH_OF_THE_RUMOR: i16 = 2144;
/// The tellers: quest, map, NPC and EXP.
const RUMORS: [(i16, i32, i32, i32); 5] = [
    (2148, 102000000, 1020000, 10),
    (2149, 102000000, 1022002, 10),
    (2150, 102000000, 1022007, 10),
    (2151, 102000003, 1022000, 10),
    (2152, 101000000, 1032104, 15),
];

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

#[tokio::test]
async fn five_tellers_and_winston_complete_the_truth_of_the_rumor() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 35, 50).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(warp(c, WINSTONS_MAP, "", &mut trace).await, WINSTONS_MAP);
    send(c, to(WINSTONS_MAP, WINSTON)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_action(START, TRUTH_OF_THE_RUMOR, WINSTON),
    )
    .await;
    assert_eq!(t.states.get(&TRUTH_OF_THE_RUMOR), Some(&1));
    // Not before the five are asked.
    let t = step(
        c,
        &m,
        build_quest_completion(TRUTH_OF_THE_RUMOR, WINSTON, -1),
    )
    .await;
    assert!(t.states.is_empty(), "{t:?}");

    for (quest, map, npc, exp) in RUMORS {
        assert_eq!(warp(c, map, "", &mut trace).await, map);
        send(c, to(map, npc)).await;
        barrier(c, &m, &mut trace).await;
        let t = step(c, &m, build_quest_action(SCRIPT_START, quest, npc)).await;
        assert_eq!(t.prompts, [(npc, 0)], "{quest}");
        let t = step(c, &m, answer(0, 1, None)).await;
        assert_eq!(t.states.get(&quest), Some(&2), "{quest}");
        assert_eq!(t.gains, [exp], "{quest}");
    }

    assert_eq!(warp(c, WINSTONS_MAP, "", &mut trace).await, WINSTONS_MAP);
    send(c, to(WINSTONS_MAP, WINSTON)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_completion(TRUTH_OF_THE_RUMOR, WINSTON, -1),
    )
    .await;
    assert_eq!(t.states.get(&TRUTH_OF_THE_RUMOR), Some(&2));
    assert_eq!(t.gains, [100]);
}
