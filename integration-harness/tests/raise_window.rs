//! The raise window: "A Cute Baby Bird" (3250) has Ghosthunter Bob hand out
//! a Timer's Egg, which grows in its window on Bird Feed dragged onto it.
//! Each feeding takes one and adds 100 to the record (7067) the quest's
//! completion reads, up to 3000; grown, it takes no more, and Bob takes the
//! egg back.

use integration_harness::database::{grant_stack, set_job, set_quest};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END, SCRIPT_START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const LOST_TIME: i32 = 220070000;
const BOB: i32 = 2041026;
const A_CUTE_BABY_BIRD: i16 = 3250;
const GROWTH: i16 = 7067;
const TIMERS_EGG: i32 = 4220046;
const BIRD_FEED: i32 = 4031992;
/// The Etc slot the feed is granted in.
const FEED_SLOT: i16 = 5;

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

/// The window asking for its record.
fn ask_growth() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::RaiseRecord as i16).unwrap();
    p.write_short(GROWTH).unwrap();
    p
}

/// Bird Feed dragged onto the egg's window.
fn feed() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::RaiseFeed as i16).unwrap();
    p.write_byte(4).unwrap();
    p.write_short(FEED_SLOT).unwrap();
    p.write_int(TIMERS_EGG).unwrap();
    p
}

#[tokio::test]
async fn the_timers_egg_grows_on_bird_feed_until_bob_takes_it_back() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 70, 50).await.unwrap();
    set_quest(id, 3249, 2, "").await.unwrap();
    grant_stack(id, 4, FEED_SLOT, BIRD_FEED, 40).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    // Before the quest, the window gets no growth and the feed stays.
    let t = step(c, &m, ask_growth()).await;
    assert!(t.kills.is_empty(), "{t:?}");
    let t = step(c, &m, feed()).await;
    assert!(t.quantities.is_empty() && t.kills.is_empty(), "{t:?}");

    assert_eq!(warp(c, LOST_TIME, "", &mut trace).await, LOST_TIME);
    send(c, to(LOST_TIME, BOB)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_START, A_CUTE_BABY_BIRD, BOB),
    )
    .await;
    assert_eq!(t.prompts, [(BOB, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(BOB, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.states.get(&A_CUTE_BABY_BIRD), Some(&1));
    assert_eq!(t.items, [(TIMERS_EGG, 1)]);
    step(c, &m, answer(0, 1, None)).await;
    send(c, answer(0, 1, None)).await;

    // Not grown yet: Bob can't end it.
    let t = step(c, &m, build_quest_action(SCRIPT_END, A_CUTE_BABY_BIRD, BOB)).await;
    assert!(t.prompts.is_empty() && t.states.is_empty(), "{t:?}");

    let t = step(c, &m, ask_growth()).await;
    assert_eq!(t.kills[&GROWTH], "0");
    for n in 1..=30i16 {
        let t = step(c, &m, feed()).await;
        assert_eq!(t.kills[&GROWTH], (n * 100).to_string());
        assert_eq!(t.quantities, [(4, FEED_SLOT, 40 - n)]);
    }
    // Grown, it takes no more.
    let t = step(c, &m, feed()).await;
    assert!(t.quantities.is_empty() && t.kills.is_empty(), "{t:?}");
    let t = step(c, &m, ask_growth()).await;
    assert_eq!(t.kills[&GROWTH], "3000");

    let t = step(c, &m, build_quest_action(SCRIPT_END, A_CUTE_BABY_BIRD, BOB)).await;
    assert_eq!(t.prompts, [(BOB, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&A_CUTE_BABY_BIRD), Some(&2));
    assert_eq!(t.items, [(TIMERS_EGG, -1)]);
}
