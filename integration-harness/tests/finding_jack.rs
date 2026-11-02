//! John Barricade's brother Jack and the Keep's letter: Finding Jack (8219)
//! from John to Jack for the Map of the Phantom Forest, Lost in Translation
//! (8227 to 8229) from Jack to John to Elpam Gorlab and back, Lukan's The
//! Brewing Storm (8222) for ten badges, and Jack's Stemming the Tide (8230)
//! ended with the Crimsonwood Keystone kept.

use integration_harness::database::{grant_stack, set_job, set_quest};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_END, SCRIPT_START,
};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const NEW_LEAF_CITY: i32 = 600000000;
const MUSHROOM_MARKETPLACE: i32 = 600020000;
const LUKANS_MAP: i32 = 610010003;
const JACKS_MAP: i32 = 610020011;
const JOHN_BARRICADE: i32 = 9201051;
const JACK: i32 = 9201096;
const ELPAM_GORLAB: i32 = 9201055;
const LUKAN: i32 = 9201098;
const PHANTOM_FOREST_MAP: i32 = 3992040;
const CRIMSONWOOD_KEYSTONE: i32 = 3992041;
const CODED_COMMUNIQUE: i32 = 4032032;
const TRANSLATED_COMMUNIQUE: i32 = 4032018;
const STORMBREAKER_BADGE: i32 = 4032006;

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

#[tokio::test]
async fn jack_john_elpam_and_lukan_carry_the_masteria_story() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 80, 50).await.unwrap();
    grant_stack(id, 4, 1, STORMBREAKER_BADGE, 10).await.unwrap();
    grant_stack(id, 3, 5, CRIMSONWOOD_KEYSTONE, 1)
        .await
        .unwrap();
    // Finding Jack asks for two of New Leaf City's earlier quests done.
    set_quest(id, 4915, 2, "").await.unwrap();
    set_quest(id, 4917, 2, "").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    // Finding Jack: John asks, Jack rewards.
    let (map, npc) = (MUSHROOM_MARKETPLACE, JOHN_BARRICADE);
    assert_eq!(warp(c, map, "", &mut trace).await, map);
    send(c, to(map, npc)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(SCRIPT_START, 8219, npc)).await;
    assert_eq!(t.prompts, [(npc, 12)]);
    step(c, &m, answer(12, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&8219), Some(&1));

    let (map, npc) = (JACKS_MAP, JACK);
    assert_eq!(warp(c, map, "", &mut trace).await, map);
    send(c, to(map, npc)).await;
    barrier(c, &m, &mut trace).await;
    step(c, &m, build_quest_action(SCRIPT_END, 8219, npc)).await;
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&8219), Some(&2));
    assert_eq!(t.items, [(PHANTOM_FOREST_MAP, 1)]);
    assert_eq!(t.gains, [175_000]);

    // Lost in Translation 1: Jack's letter, the data's end at John.
    step(c, &m, build_quest_action(SCRIPT_START, 8227, npc)).await;
    step(c, &m, answer(12, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&8227), Some(&1));
    assert_eq!(t.items, [(CODED_COMMUNIQUE, 1)]);

    // Stemming the Tide, ended with the keystone kept.
    step(c, &m, build_quest_action(SCRIPT_START, 8230, npc)).await;
    step(c, &m, answer(12, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&8230), Some(&1));
    step(c, &m, build_quest_action(SCRIPT_END, 8230, npc)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&8230), Some(&2));
    assert!(t.items.is_empty(), "{t:?}");

    let (map, npc) = (MUSHROOM_MARKETPLACE, JOHN_BARRICADE);
    assert_eq!(warp(c, map, "", &mut trace).await, map);
    send(c, to(map, npc)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_completion(8227, npc, -1)).await;
    assert_eq!(t.states.get(&8227), Some(&2));
    // Lost in Translation 2: the letter already held goes to Elpam.
    step(c, &m, build_quest_action(SCRIPT_START, 8228, npc)).await;
    step(c, &m, answer(12, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&8228), Some(&1));
    assert!(t.items.is_empty(), "{t:?}");

    let (map, npc) = (NEW_LEAF_CITY, ELPAM_GORLAB);
    assert_eq!(warp(c, map, "", &mut trace).await, map);
    send(c, to(map, npc)).await;
    barrier(c, &m, &mut trace).await;
    step(c, &m, build_quest_action(SCRIPT_END, 8228, npc)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&8228), Some(&2));
    assert_eq!(t.items, [(CODED_COMMUNIQUE, -1)]);

    // Lost in Translation 3: John's translation, taken by Jack.
    let (map, npc) = (MUSHROOM_MARKETPLACE, JOHN_BARRICADE);
    assert_eq!(warp(c, map, "", &mut trace).await, map);
    send(c, to(map, npc)).await;
    barrier(c, &m, &mut trace).await;
    step(c, &m, build_quest_action(SCRIPT_START, 8229, npc)).await;
    let t = step(c, &m, answer(12, 1, None)).await;
    assert_eq!(t.states.get(&8229), Some(&1));
    assert_eq!(t.items, [(TRANSLATED_COMMUNIQUE, 1)]);

    let (map, npc) = (JACKS_MAP, JACK);
    assert_eq!(warp(c, map, "", &mut trace).await, map);
    send(c, to(map, npc)).await;
    barrier(c, &m, &mut trace).await;
    step(c, &m, build_quest_action(SCRIPT_END, 8229, npc)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&8229), Some(&2));
    assert_eq!(t.items, [(TRANSLATED_COMMUNIQUE, -1)]);
    assert_eq!(t.gains, [50_000]);

    // The Brewing Storm: Lukan takes the ten badges.
    let (map, npc) = (LUKANS_MAP, LUKAN);
    assert_eq!(warp(c, map, "", &mut trace).await, map);
    send(c, to(map, npc)).await;
    barrier(c, &m, &mut trace).await;
    step(c, &m, build_quest_action(SCRIPT_START, 8222, npc)).await;
    step(c, &m, answer(12, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&8222), Some(&1));
    step(c, &m, build_quest_action(SCRIPT_END, 8222, npc)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&8222), Some(&2));
    assert_eq!(t.items, [(STORMBREAKER_BADGE, -10)]);
}
