//! To Lith Harbor! (1028): Shanks's start script sails a Beginner carrying
//! Lucas's message to Lith Harbor for free and starts the quest; a no
//! stays in Southperry. Olaf completes it by the data. The start asks for
//! the Wizet Plain Suit, so only a GM holding one can take it.

use integration_harness::database::{grant_item, set_job, set_quest};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_START,
};
use integration_harness::trace::{barrier, send, stat, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const SOUTHPERRY: i32 = 60000;
const SHANKS: i32 = 22000;
const LITH_HARBOR: i32 = 104000000;
const OLAF: i32 = 1002101;
const TO_LITH_HARBOR: i16 = 1028;
/// The Wizet Plain Suit, a GM item, that the quest's start asks for.
const WIZET_PLAIN_SUIT: i32 = 1042003;

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

#[tokio::test]
async fn shanks_sails_for_lucas_and_olaf_completes_the_quest() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 5, 50).await.unwrap();
    set_quest(id, 1027, 2, "").await.unwrap();
    grant_item(id, 1, 1, WIZET_PLAIN_SUIT).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(warp(c, SOUTHPERRY, "", &mut trace).await, SOUTHPERRY);
    send(c, to(SOUTHPERRY, SHANKS)).await;
    barrier(c, &m, &mut trace).await;
    // No: nothing starts and the player stays.
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_START, TO_LITH_HARBOR, SHANKS),
    )
    .await;
    assert_eq!(t.prompts, [(SHANKS, 1)]);
    let t = step(c, &m, answer(1, 0, None)).await;
    assert_eq!(t.prompts, [(SHANKS, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.states.is_empty() && t.warps.is_empty(), "{t:?}");

    step(
        c,
        &m,
        build_quest_action(SCRIPT_START, TO_LITH_HARBOR, SHANKS),
    )
    .await;
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.states.get(&TO_LITH_HARBOR), Some(&1));
    assert_eq!(t.warps, [LITH_HARBOR]);
    assert!(t.stat_values(stat::MESO).is_empty());

    send(c, to(LITH_HARBOR, OLAF)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_completion(TO_LITH_HARBOR, OLAF, -1)).await;
    assert_eq!(t.states.get(&TO_LITH_HARBOR), Some(&2));
    assert_eq!(t.gains, [130]);
    let mut items = t.items.clone();
    items.sort();
    // Meat and Lemons.
    assert_eq!(items, [(2010001, 10), (2010004, 10)]);
}
