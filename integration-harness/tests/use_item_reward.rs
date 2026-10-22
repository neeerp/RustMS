//! Reward boxes: USE_ITEM_REWARD trades a box for one of its entries, and
//! keeps the box when what it gives doesn't fit; the map sees the prize's
//! effect. A cash bag, sent as USE_CASH_ITEM, opens the same way.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_item, grant_stack};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;
use tokio::time::timeout;

const USE: i16 = 2;
const ETC: i16 = 4;
const CASH: i16 = 5;
/// A cash bag of a hundred Elixirs.
const BAG: i32 = 5_530_004;
const ELIXIR: i32 = 2_022_456;
/// Small Stories: one of two Etc items, 4001201 or 4001202.
const BOX: i32 = 2_022_422;
const STORIES: [i32; 2] = [4_001_201, 4_001_202];
const SLOT: i16 = 5;
/// Bronze Ore, to fill the Etc tab with.
const ORE: i32 = 4_010_000;
const FOREST: i32 = 50_000;

fn open(slot: i16, item_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseItemReward as i16).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(item_id).unwrap();
    p
}

/// The opcodes of the server's answer, up to and including the one that
/// unlocked the client, and the gain line after an inventory change.
async fn answer(c: &mut MapleTestConnection) -> Vec<i16> {
    let mut seen = Vec::new();
    loop {
        let p = timeout(Duration::from_secs(5), c.read_packet("reward box"))
            .await
            .expect("no answer within 5 s")
            .expect("read packet")
            .packet;
        let op = p.opcode();
        if op == SendOpcode::ShowStatusInfo as i16
            || op == SendOpcode::ShowItemGainInChat as i16
            || op == SendOpcode::InventoryOperation as i16
            || op == SendOpcode::StatChange as i16
        {
            seen.push(op);
        }
        if op == SendOpcode::StatChange as i16 || op == SendOpcode::ShowItemGainInChat as i16 {
            return seen;
        }
    }
}

/// A new character holding `boxes` Small Stories in Use slot 5.
async fn holding(boxes: i16) -> (i32, integration_harness::config::HarnessConfig) {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, USE, SLOT, BOX, boxes).await.expect("boxes");
    (id, config)
}

#[tokio::test]
async fn a_box_gives_one_of_its_entries() {
    let (_, config) = holding(2).await;
    let mut session = relogin(&config).await;
    send(&mut session.connection, open(SLOT, BOX)).await;
    assert_eq!(
        answer(&mut session.connection).await,
        [
            SendOpcode::InventoryOperation as i16,
            SendOpcode::ShowItemGainInChat as i16
        ]
    );
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let session = relogin(&config).await;
    assert_eq!(session.items.get(&BOX), Some(&1));
    let got: i32 = STORIES.iter().filter_map(|id| session.items.get(id)).sum();
    assert_eq!(got, 1);
}

#[tokio::test]
async fn a_cash_bag_opens_from_the_cash_tab() {
    let (id, config) = holding(1).await;
    grant_item(id, CASH, 1, BAG).await.expect("bag");
    let mut session = relogin(&config).await;
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(1).unwrap();
    p.write_int(BAG).unwrap();
    p.write_int(0).unwrap();
    send(&mut session.connection, p).await;
    assert_eq!(
        answer(&mut session.connection).await,
        [
            SendOpcode::InventoryOperation as i16,
            SendOpcode::ShowItemGainInChat as i16
        ]
    );
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let session = relogin(&config).await;
    assert_eq!(session.items.get(&BAG), None);
    assert_eq!(session.items.get(&ELIXIR), Some(&100));
}

#[tokio::test]
async fn a_full_etc_tab_keeps_the_box() {
    let (id, config) = holding(1).await;
    for slot in 1..=24 {
        grant_item(id, ETC, slot, ORE).await.expect("ore");
    }
    let mut session = relogin(&config).await;
    send(&mut session.connection, open(SLOT, BOX)).await;
    assert_eq!(
        answer(&mut session.connection).await,
        [
            SendOpcode::ShowStatusInfo as i16,
            SendOpcode::StatChange as i16
        ]
    );
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let session = relogin(&config).await;
    assert_eq!(session.items.get(&BOX), Some(&1));
}

#[tokio::test]
async fn a_box_not_held_is_refused() {
    let (_, config) = holding(1).await;
    let mut session = relogin(&config).await;
    send(&mut session.connection, open(SLOT + 1, BOX)).await;
    assert_eq!(
        answer(&mut session.connection).await,
        [SendOpcode::StatChange as i16]
    );
}

#[tokio::test]
async fn the_map_sees_the_prize() {
    let (_, config) = holding(1).await;
    let watcher_config = load_harness_config_or_fail().await;
    let mut opener = relogin(&config).await;
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let (mut o, mut w) = (Trace::default(), Trace::default());
    assert_eq!(
        warp(&mut opener.connection, FOREST, "", &mut o).await,
        FOREST
    );
    assert_eq!(
        warp(&mut watcher.connection, FOREST, "", &mut w).await,
        FOREST
    );
    read_for(&mut opener.connection, &mut o, 300).await;
    read_for(&mut watcher.connection, &mut w, 300).await;

    send(&mut opener.connection, open(SLOT, BOX)).await;
    loop {
        let p = timeout(
            Duration::from_secs(5),
            watcher.connection.read_packet("prize"),
        )
        .await
        .expect("no effect within 5 s")
        .expect("read packet")
        .packet;
        let b = &p.bytes;
        if p.opcode() != SendOpcode::ShowForeignEffect as i16 || b[6] != 14 {
            continue;
        }
        // The opener, then one of the stories and no animation.
        let cid = i32::from_le_bytes([b[2], b[3], b[4], b[5]]);
        let item = i32::from_le_bytes([b[7], b[8], b[9], b[10]]);
        assert_eq!(cid, opener.character_id);
        assert!(STORIES.contains(&item), "{item}");
        break;
    }
}
