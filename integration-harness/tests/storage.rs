//! Storage (TRUNK_REQUEST, TRUNK_RESULT): Mr. Lee in Henesys Park keeps an
//! account's items and mesos for a fee, and they stay kept across logins.

use integration_harness::database::{grant_stack, set_meso};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, stat, step, until, warp, StorageResult, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::collections::BTreeMap;
use tokio::time::Duration;

const HENESYS: i32 = 100_000_000;
const HENESYS_PARK: i32 = 100_000_200;
const MR_LEE: i32 = 1012009;
const RED_POTION: i32 = 2000000;
const SNAIL_SHELL: i32 = 4000000;
const USE: u8 = 2;
const ETC: u8 = 4;

const TAKEN_OUT: u8 = 0x09;
const STORED: u8 = 0x0d;
const SORTED: u8 = 0x0f;
const MESO: u8 = 0x13;
const OPEN: u8 = 0x16;
const ERROR: u8 = 0x12;

fn talk(object_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(object_id).unwrap();
    p
}

fn request(mode: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TrunkRequest as i16).unwrap();
    p.write_byte(mode).unwrap();
    p
}

fn take_out(tab: u8, index: u8) -> Packet {
    let mut p = request(4);
    p.write_byte(tab).unwrap();
    p.write_byte(index).unwrap();
    p
}

fn store(slot: i16, item_id: i32, quantity: i16) -> Packet {
    let mut p = request(5);
    p.write_short(slot).unwrap();
    p.write_int(item_id).unwrap();
    p.write_short(quantity).unwrap();
    p
}

/// Positive takes mesos out, negative puts them in.
fn meso(amount: i32) -> Packet {
    let mut p = request(7);
    p.write_int(amount).unwrap();
    p
}

fn bags(contents: &[(u8, &[(i32, i16)])]) -> BTreeMap<u8, Vec<(i32, i16)>> {
    contents
        .iter()
        .map(|&(kind, items)| (kind, items.to_vec()))
        .collect()
}

/// Every bag, with the given ones filled.
fn all_bags(contents: &[(u8, &[(i32, i16)])]) -> BTreeMap<u8, Vec<(i32, i16)>> {
    let mut all: BTreeMap<_, _> = (1..=5).map(|kind| (kind, Vec::new())).collect();
    all.extend(bags(contents));
    all
}

async fn open(c: &mut integration_harness::connection::MapleTestConnection) -> StorageResult {
    let mut seen = Trace::default();
    until(c, &mut seen, |t| t.field.npc(MR_LEE).is_some()).await;
    send(c, talk(seen.field.npc(MR_LEE).unwrap())).await;
    until(c, &mut seen, |t| !t.storage.is_empty()).await;
    seen.storage[0].clone()
}

#[tokio::test]
async fn mr_lee_keeps_items_and_mesos_for_a_fee() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(id, 1000).await.expect("give mesos");
    grant_stack(id, ETC.into(), 1, SNAIL_SHELL, 10)
        .await
        .expect("give snail shells");
    grant_stack(id, USE.into(), 1, RED_POTION, 5)
        .await
        .expect("give red potions");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut seen = Trace::default();
    assert_eq!(warp(c, HENESYS_PARK, "", &mut seen).await, HENESYS_PARK);

    // A new account's storage is empty, with four slots.
    let opened = open(c).await;
    assert_eq!(
        opened,
        StorageResult {
            mode: OPEN,
            npc: Some(MR_LEE),
            slots: Some(4),
            meso: Some(0),
            bags: all_bags(&[]),
        }
    );

    // Storing part of a stack costs Mr. Lee's fee of 100 mesos.
    let t = step(c, "storage-shells", store(1, SNAIL_SHELL, 4)).await;
    assert_eq!(t.storage.len(), 1, "{:?}", t.storage);
    assert_eq!(t.storage[0].mode, STORED);
    assert_eq!(t.storage[0].bags, bags(&[(ETC, &[(SNAIL_SHELL, 4)])]));
    assert_eq!(t.stat(stat::MESO), Some(900));
    assert_eq!(t.quantities, [(ETC, 1, 6)]);

    // More than the stack holds is refused, and nothing moves.
    let t = step(c, "storage-too-many", store(1, SNAIL_SHELL, 7)).await;
    assert_eq!(
        t.storage.iter().map(|r| r.mode).collect::<Vec<_>>(),
        [ERROR]
    );
    assert!(t.quantities.is_empty() && t.stat(stat::MESO).is_none());

    // A whole stack leaves its slot.
    let t = step(c, "storage-potions", store(1, RED_POTION, 5)).await;
    assert_eq!(t.storage[0].bags, bags(&[(USE, &[(RED_POTION, 5)])]));
    assert_eq!(t.removed, [(USE, 1)]);
    assert_eq!(t.stat(stat::MESO), Some(800));

    // Mesos go in for free, and no more can come out than went in.
    let t = step(c, "storage-meso-in", meso(-300)).await;
    assert_eq!(t.storage[0].mode, MESO);
    assert_eq!(t.storage[0].meso, Some(300));
    assert_eq!(t.stat(stat::MESO), Some(500));
    let t = step(c, "storage-meso-out", meso(400)).await;
    assert_eq!(t.storage[0].mode, ERROR);
    assert!(t.stat(stat::MESO).is_none());

    // Sorting resends everything.
    let t = step(c, "storage-sort", request(6)).await;
    assert_eq!(t.storage[0].mode, SORTED);
    assert_eq!(t.storage[0].meso, Some(300));
    assert_eq!(
        t.storage[0].bags,
        all_bags(&[(USE, &[(RED_POTION, 5)]), (ETC, &[(SNAIL_SHELL, 4)])])
    );

    // Taking out is free here, and the shells rejoin their stack.
    let t = step(c, "storage-take-shells", take_out(ETC, 0)).await;
    assert_eq!(t.storage[0].mode, TAKEN_OUT);
    assert_eq!(t.storage[0].bags, bags(&[(ETC, &[])]));
    assert_eq!(t.quantities, [(ETC, 1, 10)]);
    let t = step(c, "storage-take-nothing", take_out(ETC, 0)).await;
    assert_eq!(t.storage[0].mode, ERROR);

    // Once closed, the storage answers nothing.
    send(c, request(8)).await;
    let t = step(c, "storage-closed", store(1, SNAIL_SHELL, 1)).await;
    assert!(t.storage.is_empty() && t.quantities.is_empty());
    drop(session);

    // Both sides were saved with each change.
    let mut session = relogin(&config).await;
    assert_eq!(session.meso, 500);
    assert_eq!(session.items.get(&SNAIL_SHELL), Some(&10));
    assert_eq!(session.items.get(&RED_POTION), None);
    // Login read the park's NPCs; walk out and back in to see Mr. Lee.
    let c = &mut session.connection;
    let mut seen = Trace::default();
    assert_eq!(warp(c, HENESYS, "", &mut seen).await, HENESYS);
    assert_eq!(warp(c, HENESYS_PARK, "", &mut seen).await, HENESYS_PARK);
    let opened = open(c).await;
    assert_eq!(opened.meso, Some(300));
    assert_eq!(opened.bags, all_bags(&[(USE, &[(RED_POTION, 5)])]));
}
