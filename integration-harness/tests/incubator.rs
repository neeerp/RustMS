//! The Incubator: a Pigmy Egg hatches into a prize for the incubator and
//! the egg, and is kept, with both, when the inventory is too full.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_item, grant_stack};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::send;
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;
use tokio::time::timeout;

const ETC: i16 = 4;
const CASH: i16 = 5;
const INCUBATOR: i32 = 5_060_002;
const EGG: i32 = 4_170_000;
/// The DS Egg, which the window refuses.
const DS_EGG: i32 = 4_170_012;
const EGG_SLOT: i16 = 10;
/// Bronze Ore, to fill the Etc tab with.
const ORE: i32 = 4_010_000;

/// What the server answered: the egg's prize from INCUBATOR_RESULT, if one
/// came, then the opcode of the packet that unlocked the client.
struct Answer {
    prize: Option<(i32, i16)>,
    unlock: i16,
}

fn incubate(slot: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(INCUBATOR).unwrap();
    p.write_int(i32::from(ETC)).unwrap();
    p.write_int(i32::from(EGG_SLOT)).unwrap();
    p.write_int(0).unwrap();
    p
}

async fn answer(c: &mut MapleTestConnection) -> Answer {
    let mut prize = None;
    loop {
        let p = timeout(Duration::from_secs(5), c.read_packet("incubator"))
            .await
            .expect("no answer within 5 s")
            .expect("read packet")
            .packet;
        let op = p.opcode();
        if op == SendOpcode::IncubatorResult as i16 {
            let b = &p.bytes[2..];
            assert_eq!(b.len(), 6);
            let id = i32::from_le_bytes([b[0], b[1], b[2], b[3]]);
            prize = Some((id, i16::from_le_bytes([b[4], b[5]])));
        } else if op == SendOpcode::InventoryOperation as i16 || op == SendOpcode::StatChange as i16
        {
            return Answer { prize, unlock: op };
        }
    }
}

/// A new character holding an incubator in Cash slot 1 and `eggs` of `egg`.
async fn holding(egg: i32, eggs: i16) -> (i32, integration_harness::config::HarnessConfig) {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_item(id, CASH, 1, INCUBATOR).await.expect("incubator");
    grant_stack(id, ETC, EGG_SLOT, egg, eggs)
        .await
        .expect("eggs");
    (id, config)
}

#[tokio::test]
async fn an_egg_hatches_for_the_incubator() {
    let (_, config) = holding(EGG, 2).await;
    let mut session = relogin(&config).await;
    send(&mut session.connection, incubate(1)).await;
    let Answer { prize, unlock } = answer(&mut session.connection).await;
    assert_eq!(unlock, SendOpcode::InventoryOperation as i16);
    let (prize, count) = prize.expect("INCUBATOR_RESULT first");
    assert!(prize > 0 && count > 0, "{prize} x{count}");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let session = relogin(&config).await;
    assert_eq!(session.items.get(&INCUBATOR), None);
    assert_eq!(session.items.get(&EGG), Some(&1));
    assert!(session.items.get(&prize) >= Some(&i32::from(count)));
}

#[tokio::test]
async fn a_full_etc_tab_keeps_the_egg() {
    // Two eggs, so the one hatched leaves no slot free.
    let (id, config) = holding(EGG, 2).await;
    for slot in (1..=24).filter(|&slot| slot != EGG_SLOT) {
        grant_item(id, ETC, slot, ORE).await.expect("ore");
    }
    let mut session = relogin(&config).await;
    send(&mut session.connection, incubate(1)).await;
    let Answer { prize, unlock } = answer(&mut session.connection).await;
    assert_eq!(prize, Some((0, 0)), "Your inventory is full.");
    assert_eq!(unlock, SendOpcode::StatChange as i16);
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let session = relogin(&config).await;
    assert_eq!(session.items.get(&INCUBATOR), Some(&1));
    assert_eq!(session.items.get(&EGG), Some(&2));
}

#[tokio::test]
async fn the_ds_egg_does_not_hatch() {
    let (_, config) = holding(DS_EGG, 1).await;
    let mut session = relogin(&config).await;
    send(&mut session.connection, incubate(1)).await;
    let Answer { prize, unlock } = answer(&mut session.connection).await;
    assert_eq!(prize, None);
    assert_eq!(unlock, SendOpcode::StatChange as i16);
}
