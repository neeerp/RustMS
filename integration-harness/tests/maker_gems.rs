//! The Item Maker's gems and monster crystals. Making a gem takes its ore
//! and the fee and gives one of the gems its recipe names; a hundred etc
//! drops make the crystal of their level. Every ITEM_MAKE gets a
//! MAKER_RESULT, refusals included, since the window waits for one.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_skill, grant_stack, set_job, set_meso};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::send;
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::{Duration, Instant};
use tokio::time::timeout;

const ETC: i16 = 4;
/// The Beginner's Maker skill.
const MAKER: i32 = 1_007;
/// Diamond Ore, and the Basic, Intermediate and Advanced Diamonds it may
/// make.
const ORE: i32 = 4_021_007;
const GEMS: [i32; 3] = [4_250_000, 4_250_001, 4_250_002];
/// A level 35 drop, and the crystal a hundred of it make.
const DROP: i32 = 4_000_013;
const CRYSTAL: i32 = 4_260_000;
/// The Basic gem's fee as the window shows it.
const FEE: i32 = 110_000;

/// Read until a packet with `op` within `secs`; return its body and the
/// opcodes read before it.
async fn next(c: &mut MapleTestConnection, op: SendOpcode, secs: u64) -> (Vec<u8>, Vec<i16>) {
    let op = op as i16;
    let deadline = Instant::now() + Duration::from_secs(secs);
    let mut before = Vec::new();
    loop {
        let p = timeout(
            deadline.saturating_duration_since(Instant::now()),
            c.read_packet("item maker"),
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

fn item_make(values: &[i32], stimulator: Option<u8>) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemMake as i16).unwrap();
    let (head, tail) = values.split_at(values.len().min(2));
    for value in head {
        p.write_int(*value).unwrap();
    }
    if let Some(stimulator) = stimulator {
        p.write_byte(stimulator).unwrap();
    }
    for value in tail {
        p.write_int(*value).unwrap();
    }
    p
}

fn ints(values: &[i32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

#[tokio::test]
async fn the_maker_makes_gems_and_crystals() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 50, 100).await.expect("level 50");
    grant_skill(id, MAKER, 1).await.expect("the Maker");
    set_meso(id, 1_000_000).await.expect("mesos");
    grant_stack(id, ETC, 1, ORE, 1).await.expect("ore");
    grant_stack(id, ETC, 2, DROP, 100).await.expect("drops");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;

    // A gem: the ore and the fee go, one of the three gems comes.
    send(c, item_make(&[1, GEMS[0], 0], Some(0))).await;
    let (body, before) = next(c, SendOpcode::MakerResult, 5).await;
    assert!(before.contains(&(SendOpcode::InventoryOperation as i16)));
    assert_eq!(body[..9], [ints(&[0, 1]), vec![0]].concat());
    let made = i32::from_le_bytes(body[9..13].try_into().unwrap());
    assert!(GEMS.contains(&made), "made {made}");
    assert_eq!(
        body[13..],
        [ints(&[1, 1, ORE, 1, 0]), vec![0], ints(&[FEE])].concat()
    );
    let (effect, _) = next(c, SendOpcode::ShowItemGainInChat, 5).await;
    assert_eq!(effect, [vec![0x10], ints(&[0])].concat());

    // No ore left: the window still gets an answer.
    send(c, item_make(&[1, GEMS[0], 0], Some(0))).await;
    let (body, _) = next(c, SendOpcode::MakerResult, 5).await;
    assert_eq!(body, ints(&[2]));

    // A crystal from a hundred drops.
    send(c, item_make(&[3, DROP], None)).await;
    let (body, before) = next(c, SendOpcode::MakerResult, 5).await;
    assert!(before.contains(&(SendOpcode::InventoryOperation as i16)));
    assert_eq!(body, ints(&[0, 3, CRYSTAL, DROP]));
    drop(session);

    let session = relogin(&config).await;
    assert_eq!(session.meso, 1_000_000 - FEE);
    assert_eq!(session.items.get(&made), Some(&1));
    assert_eq!(session.items.get(&CRYSTAL), Some(&1));
    assert_eq!(session.items.get(&ORE), None);
    assert_eq!(session.items.get(&DROP), None);
}
