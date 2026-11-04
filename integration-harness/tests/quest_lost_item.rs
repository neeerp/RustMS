//! A quest item from a quest's start, lost. The wrong NPC's `lost` page says
//! it will be made again, and OK on that page has v83's client send
//! `QUEST_ACTION` 0 with what is gone. Tae Sang's Herb Pouch (3826) is one.

use integration_harness::database::{grant_item, start_quest};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{barrier, send, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

const QUEST: i16 = 3826;
const HERB_POUCH: i32 = 4031555;
const SNAIL_SHELL: i32 = 4000000;
const ETC: i16 = 4;

fn ask_for_the_pouch() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::QuestAction as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_short(QUEST).unwrap();
    p.write_int(1).unwrap();
    p.write_int(HERB_POUCH).unwrap();
    p
}

#[tokio::test]
async fn a_lost_quest_item_comes_back_while_the_quest_is_started() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut session = login_to_world_session(&config)
        .await
        .expect("first login failed");
    let id = session.character_id;
    let c = &mut session.connection;

    // Not started: nothing to give back.
    let mut trace = Trace::default();
    send(c, ask_for_the_pouch()).await;
    barrier(c, &m, &mut trace).await;
    assert!(trace.items.is_empty(), "{:?}", trace.items);

    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    start_quest(id, QUEST).await.expect("start the quest");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    send(c, ask_for_the_pouch()).await;
    barrier(c, &m, &mut trace).await;
    assert_eq!(trace.items, [(HERB_POUCH, 1)]);
    assert!(trace.quest_refusals.is_empty());

    // Held again: a second ask gets nothing.
    send(c, ask_for_the_pouch()).await;
    barrier(c, &m, &mut trace).await;
    assert_eq!(trace.items, [(HERB_POUCH, 1)]);
}

#[tokio::test]
async fn a_full_etc_inventory_is_told_so() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    start_quest(id, QUEST).await.expect("start the quest");
    for slot in 1..=24 {
        grant_item(id, ETC, slot, SNAIL_SHELL)
            .await
            .expect("fill the Etc inventory");
    }

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    send(c, ask_for_the_pouch()).await;
    barrier(c, &m, &mut trace).await;
    assert_eq!(trace.quest_refusals, [(10, QUEST)]);
    assert!(trace.items.is_empty(), "{:?}", trace.items);
}
