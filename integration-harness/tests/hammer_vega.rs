//! Vicious' Hammer and Vega's Spell. A hammer gives an equip one more
//! upgrade slot, twice at most; its window acks the answer with USE_HAMMER
//! and the answer to that closes it. Vega's Spell scrolls an equip at a
//! raised rate: the start code comes at once, the items and the result once
//! the window's roll is over.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{equip_upgrades, grant_stack, stow_equip};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::send;
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::{Duration, Instant};
use tokio::time::timeout;

const USE: i16 = 2;
const CASH: i16 = 5;
const HAMMER: i32 = 5_570_000;
const VEGA_60: i32 = 5_610_001;
/// Work Gloves, and a Scroll for Gloves for DEX 60%.
const GLOVE: i32 = 1_082_002;
const DEX_60: i32 = 2_040_801;

/// Read until a packet with `op` within `secs`; return its body and the
/// opcodes read before it.
async fn next(c: &mut MapleTestConnection, op: SendOpcode, secs: u64) -> (Vec<u8>, Vec<i16>) {
    let op = op as i16;
    let deadline = Instant::now() + Duration::from_secs(secs);
    let mut before = Vec::new();
    loop {
        let p = timeout(
            deadline.saturating_duration_since(Instant::now()),
            c.read_packet("hammer and vega"),
        )
        .await
        .unwrap_or_else(|_| panic!("no {op:#x} within {secs} s"))
        .expect("read packet")
        .packet;
        if p.opcode() == op {
            return (p.bytes[2..].to_vec(), before);
        }
        before.push(p.opcode());
    }
}

fn use_hammer(slot: i16, equip_slot: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(HAMMER).unwrap();
    for value in [1, equip_slot, 0] {
        p.write_int(value).unwrap();
    }
    p
}

fn hammer_ack() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseHammer as i16).unwrap();
    p.write_int(0x39).unwrap();
    p.write_int(0).unwrap();
    p
}

fn bytes(code: u8, values: &[i32]) -> Vec<u8> {
    let mut body = vec![code];
    for value in values {
        body.extend(value.to_le_bytes());
    }
    body
}

#[tokio::test]
async fn a_hammer_adds_an_upgrade_slot_twice_at_most() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    stow_equip(id, 1, GLOVE, 5).await.expect("gloves");
    grant_stack(id, CASH, 1, HAMMER, 3).await.expect("hammers");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    for applied in [1, 2] {
        send(c, use_hammer(1, 1)).await;
        let (body, before) = next(c, SendOpcode::ViciousHammer, 5).await;
        assert_eq!(body, bytes(0x39, &[0, applied]));
        assert!(before.contains(&(SendOpcode::InventoryOperation as i16)));
        send(c, hammer_ack()).await;
        let (body, _) = next(c, SendOpcode::ViciousHammer, 5).await;
        assert_eq!(body, bytes(0x3d, &[0]));
    }
    send(c, use_hammer(1, 1)).await;
    let (body, _) = next(c, SendOpcode::ViciousHammer, 5).await;
    assert_eq!(body, bytes(0x3e, &[2]), "two hammers are the most");
    drop(session);

    let session = relogin(&config).await;
    assert_eq!(session.items.get(&HAMMER), Some(&1));
    assert_eq!(equip_upgrades(id, 1).await.unwrap(), Some((7, 0, 2)));
}

#[tokio::test]
async fn vega_s_spell_shows_its_result_after_the_roll() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    stow_equip(id, 1, GLOVE, 5).await.expect("gloves");
    grant_stack(id, USE, 1, DEX_60, 1).await.expect("a scroll");
    grant_stack(id, CASH, 1, VEGA_60, 1).await.expect("a spell");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(1).unwrap();
    p.write_int(VEGA_60).unwrap();
    for value in [1, 1, 2, 1, 1, 0] {
        p.write_int(value).unwrap();
    }
    send(c, p).await;
    let (start, _) = next(c, SendOpcode::VegaScroll, 2).await;
    let started = Instant::now();
    let worked = match start[..] {
        [0x40] => true,
        [0x45] => false,
        _ => panic!("start code {start:?}"),
    };
    let (result, before) = next(c, SendOpcode::VegaScroll, 5).await;
    assert!(
        started.elapsed() >= Duration::from_secs(2),
        "the roll is over"
    );
    assert_eq!(result, [if worked { 0x41 } else { 0x43 }]);
    assert!(before.contains(&(SendOpcode::InventoryOperation as i16)));
    drop(session);

    let session = relogin(&config).await;
    assert_eq!(session.items.get(&VEGA_60), None);
    assert_eq!(session.items.get(&DEX_60), None);
    let level = i16::from(worked);
    assert_eq!(equip_upgrades(id, 1).await.unwrap(), Some((4, level, 0)));
}
