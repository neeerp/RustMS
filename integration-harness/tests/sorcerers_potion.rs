//! The Sorcerer Who Sells Emotions (3514): the data's start sells the
//! Sorcerer's Potion for 1,000,000 mesos; drunk, it leaves 1 HP, and with
//! it gone the Sorcerer's end script gives the EXP.

use integration_harness::database::{set_job, set_meso, set_quest};
use integration_harness::events::maps;
use integration_harness::hunting::{use_item, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END, START};
use integration_harness::trace::{barrier, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const SORCERERS_MAP: i32 = 270020211;
const SORCERER: i32 = 2140002;
const SORCERERS_POTION: i32 = 2022337;
const SELLS_EMOTIONS: i16 = 3514;
const HP: i32 = 0x400;

/// A walk to the Sorcerer.
fn to_sorcerer() -> Packet {
    let npcs = &maps().field(SORCERERS_MAP).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == SORCERER).unwrap();
    walk_to(at.x, at.y)
}

#[tokio::test]
async fn the_sorcerers_potion_leaves_one_hp_and_completes_the_quest() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 120, 50).await.unwrap();
    set_meso(id, 1_200_000).await.unwrap();
    set_quest(id, 3513, 2, "").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(warp(c, SORCERERS_MAP, "", &mut trace).await, SORCERERS_MAP);
    send(c, to_sorcerer()).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(START, SELLS_EMOTIONS, SORCERER)).await;
    assert_eq!(t.states.get(&SELLS_EMOTIONS), Some(&1));
    assert_eq!(t.items, [(SORCERERS_POTION, 1)]);
    assert_eq!(t.meso_gains, [-1_000_000]);
    let (_, slot, _) = t.added[0];

    // Not while the potion is held.
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_END, SELLS_EMOTIONS, SORCERER),
    )
    .await;
    assert!(t.prompts.is_empty() && t.states.is_empty(), "{t:?}");

    let mut t = step(c, &m, use_item(slot, SORCERERS_POTION)).await;
    until(c, &mut t, |t| t.stat(HP).is_some()).await;
    assert_eq!(t.stat(HP), Some(1));

    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_END, SELLS_EMOTIONS, SORCERER),
    )
    .await;
    assert_eq!(t.prompts, [(SORCERER, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.gains, [891_500]);
    assert_eq!(t.states.get(&SELLS_EMOTIONS), Some(&2));
}
