//! Kerning's scents (2282 to 2284): each start puts a scent's effect on the
//! player, which completion at the other NPC needs, and the next scent
//! can't start while the last one's effect is on (`Check/1/buff`,
//! `Check/0/exceptbuff`).

use integration_harness::database::{set_job, set_quest};
use integration_harness::events::maps;
use integration_harness::hunting::{cancel_item_effect, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{build_quest_action, build_quest_completion, START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const KERNING_SQUARE: i32 = 103040000;
const LALA: i32 = 1052119;
const LANA: i32 = 1052121;
const ROSE_SCENT: i16 = 2282;
const ROSE: i32 = 2022631;
const FREESIA_SCENT: i16 = 2283;
const FREESIA: i32 = 2022632;

/// A walk to NPC `npc` in Kerning Square's lobby.
fn to(npc: i32) -> Packet {
    let npcs = &maps().field(KERNING_SQUARE).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

#[tokio::test]
async fn a_scent_completes_under_its_effect_and_the_next_waits_it_out() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 400, 40, 50).await.unwrap();
    set_quest(id, 2281, 2, "").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(
        warp(c, KERNING_SQUARE, "", &mut trace).await,
        KERNING_SQUARE
    );
    send(c, to(LALA)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(START, ROSE_SCENT, LALA)).await;
    assert_eq!(t.states.get(&ROSE_SCENT), Some(&1));
    assert!(t.buffs.iter().any(|b| b.0 == -ROSE), "{t:?}");

    send(c, to(LANA)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_completion(ROSE_SCENT, LANA, -1)).await;
    assert_eq!(t.states.get(&ROSE_SCENT), Some(&2));
    assert_eq!(t.gains, [2300]);

    // Freesia waits while the rose's effect is on.
    send(c, to(LALA)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(START, FREESIA_SCENT, LALA)).await;
    assert!(t.states.is_empty() && t.buffs.is_empty(), "{t:?}");
    send(c, cancel_item_effect(-ROSE)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(START, FREESIA_SCENT, LALA)).await;
    assert_eq!(t.states.get(&FREESIA_SCENT), Some(&1));
    assert!(t.buffs.iter().any(|b| b.0 == -FREESIA), "{t:?}");

    // Without its own effect, Freesia doesn't complete.
    send(c, cancel_item_effect(-FREESIA)).await;
    send(c, to(LANA)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_completion(FREESIA_SCENT, LANA, -1)).await;
    assert!(t.states.is_empty() && t.gains.is_empty(), "{t:?}");
}
