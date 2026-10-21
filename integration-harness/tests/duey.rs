//! Duey (DUEY_ACTION, PARCEL): a character sends mesos and items to
//! another, who claims or deletes them in the Duey window, and hears of
//! them on arrival or on entering the game.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_stack, set_meso};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, stat, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const HENESYS: i32 = 100_000_000;
const DUEY: i32 = 9010009;
const RED_POTION: i32 = 2000000;
const COUPON: i32 = 5330000;
const USE: u8 = 2;
const CASH: i16 = 5;

const OPEN: u8 = 0x08;
const NO_MESOS: u8 = 0x0a;
const NO_RECEIVER: u8 = 0x0c;
const SENT: u8 = 0x12;
const UNKNOWN: u8 = 0x13;
const REMOVED: u8 = 0x17;
const ARRIVED: u8 = 0x19;
const QUICK_SEND: u8 = 0x1a;
const WAITING: u8 = 0x1b;
const DELETED: u8 = 3;
const CLAIMED: u8 = 4;

fn talk(object_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(object_id).unwrap();
    p
}

fn action(op: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::DueyAction as i16).unwrap();
    p.write_byte(op).unwrap();
    p
}

fn send_parcel(kind: u8, slot: i16, quantity: i16, mesos: i32, to: &str) -> Packet {
    let mut p = action(2);
    p.write_byte(kind).unwrap();
    p.write_short(slot).unwrap();
    p.write_short(quantity).unwrap();
    p.write_int(mesos).unwrap();
    p.write_str_with_length(to).unwrap();
    p
}

fn normal(kind: u8, slot: i16, quantity: i16, mesos: i32, to: &str) -> Packet {
    let mut p = send_parcel(kind, slot, quantity, mesos, to);
    p.write_byte(0).unwrap();
    p
}

fn quick(mesos: i32, to: &str, message: &str) -> Packet {
    let mut p = send_parcel(0, 0, 0, mesos, to);
    p.write_byte(1).unwrap();
    p.write_str_with_length(message).unwrap();
    p.write_int(1).unwrap();
    p
}

fn with_id(op: u8, id: i32) -> Packet {
    let mut p = action(op);
    p.write_int(id).unwrap();
    p
}

fn use_coupon() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(1).unwrap();
    p.write_int(COUPON).unwrap();
    p.write_int(0).unwrap();
    p
}

/// A parcel as the window lists it.
#[derive(Debug, PartialEq, Eq)]
struct Listed {
    id: i32,
    sender: String,
    mesos: i32,
    message: String,
    quick: bool,
    item: Option<i32>,
}

/// Read a window's parcels and the number that expired.
fn window(body: &[u8]) -> (Vec<Listed>, u8) {
    assert_eq!(body[..2], [OPEN, 0], "a full window");
    let int = |at: usize| i32::from_le_bytes(body[at..at + 4].try_into().unwrap());
    let text = |at: usize, len: usize| {
        let bytes = &body[at..at + len];
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(len);
        String::from_utf8_lossy(&bytes[..end]).into_owned()
    };
    let mut at = 3;
    let mut listed = Vec::new();
    for _ in 0..body[2] {
        let has_item = body[at + 234] == 1;
        listed.push(Listed {
            id: int(at),
            sender: text(at + 4, 13),
            mesos: int(at + 17),
            message: text(at + 33, 200),
            quick: body[at + 233] == 1,
            item: has_item.then(|| int(at + 236)),
        });
        // Only plain stacks are sent here: type, id, cash flag, expiry,
        // quantity, owner, flags.
        at += 235
            + if has_item {
                1 + 4 + 1 + 8 + 2 + 2 + 2
            } else {
                0
            };
    }
    (listed, body[at])
}

async fn open(c: &mut MapleTestConnection, seen: &mut Trace) -> (Vec<Listed>, u8) {
    until(c, seen, |t| t.field.npc(DUEY).is_some()).await;
    let duey = seen.field.npc(DUEY).unwrap();
    let before = seen.parcels.len();
    send(c, talk(duey)).await;
    until(c, seen, |t| t.parcels.len() > before).await;
    window(&seen.parcels[before])
}

#[tokio::test]
async fn duey_carries_mesos_and_items_to_an_offline_character() {
    let receiver_config = load_harness_config_or_fail().await;
    let receiver = login_to_world_session(&receiver_config)
        .await
        .expect("receiver login failed");
    let receiver_name = receiver.character_name.clone();
    drop(receiver);
    let sender_config = load_harness_config_or_fail().await;
    let sender_id = login_to_world_session(&sender_config)
        .await
        .expect("sender login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(sender_id, 100_000).await.expect("give mesos");
    grant_stack(sender_id, USE.into(), 1, RED_POTION, 10)
        .await
        .expect("give red potions");

    let mut sender = relogin(&sender_config).await;
    let s = &mut sender.connection;
    let mut seen = Trace::default();
    assert_eq!(warp(s, HENESYS, "", &mut seen).await, HENESYS);
    assert_eq!(open(s, &mut seen).await, (Vec::new(), 0));

    // Nobody by that name, nor the sender themself.
    let t = step(s, "duey-nobody", normal(0, 0, 0, 1000, "nobodyhere")).await;
    assert_eq!(t.parcels, [[NO_RECEIVER]]);
    let t = step(
        s,
        "duey-self",
        normal(0, 0, 0, 1000, &sender.character_name),
    )
    .await;
    assert_eq!(t.parcels, [[NO_RECEIVER]]);

    // Four potions and 1000 mesos, for the fee of 5000.
    let t = step(s, "duey-send", normal(USE, 1, 4, 1000, &receiver_name)).await;
    assert_eq!(t.parcels, [[SENT]]);
    assert_eq!(t.stat(stat::MESO), Some(94_000));
    assert_eq!(t.quantities, [(USE, 1, 6)]);

    // The fee comes on top of the mesos sent.
    let t = step(s, "duey-broke", normal(0, 0, 0, 90_000, &receiver_name)).await;
    assert_eq!(t.parcels, [[NO_MESOS]]);
    assert!(t.stat(stat::MESO).is_none());
    send(s, action(7)).await;

    // The receiver hears of it on entering the game, and claims it.
    let mut receiver = relogin(&receiver_config).await;
    let r = &mut receiver.connection;
    let mut seen = Trace::default();
    until(r, &mut seen, |t| !t.parcels.is_empty()).await;
    assert_eq!(seen.parcels, [[WAITING, 0]]);
    assert_eq!(warp(r, HENESYS, "", &mut seen).await, HENESYS);
    let (listed, expired) = open(r, &mut seen).await;
    let [parcel] = &listed[..] else {
        panic!("one parcel: {listed:?}");
    };
    assert_eq!(expired, 0);
    assert_eq!(
        (&parcel.sender, parcel.mesos, parcel.item, parcel.quick),
        (&sender.character_name, 1000, Some(RED_POTION), false)
    );
    let t = step(r, "duey-claim", with_id(4, parcel.id)).await;
    let id = parcel.id.to_le_bytes();
    assert_eq!(
        t.parcels,
        [vec![REMOVED, id[0], id[1], id[2], id[3], CLAIMED]]
    );
    assert_eq!(t.stat(stat::MESO), Some(1000));

    // It's gone.
    let t = step(r, "duey-claim-again", with_id(4, parcel.id)).await;
    assert_eq!(t.parcels, [[UNKNOWN]]);
}

#[tokio::test]
async fn a_quick_delivery_reaches_an_online_character_who_may_delete_it() {
    let receiver_config = load_harness_config_or_fail().await;
    let mut receiver = login_to_world_session(&receiver_config)
        .await
        .expect("receiver login failed");
    let sender_config = load_harness_config_or_fail().await;
    let sender_id = login_to_world_session(&sender_config)
        .await
        .expect("sender login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(sender_id, 300_000).await.expect("give mesos");
    grant_stack(sender_id, CASH, 1, COUPON, 1)
        .await
        .expect("give a coupon");

    // The coupon opens the window for a quick send, and goes with it: no
    // base fee, only the tax on 200,000.
    let mut sender = relogin(&sender_config).await;
    let s = &mut sender.connection;
    let t = step(s, "duey-coupon", use_coupon()).await;
    assert_eq!(t.parcels, [[QUICK_SEND]]);
    let t = step(
        s,
        "duey-quick",
        quick(200_000, &receiver.character_name, "for you"),
    )
    .await;
    assert_eq!(t.parcels, [[SENT]]);
    assert_eq!(t.stat(stat::MESO), Some(300_000 - 200_000 - 1_600));
    send(s, action(7)).await;

    let r = &mut receiver.connection;
    let mut seen = Trace::default();
    until(r, &mut seen, |t| !t.parcels.is_empty()).await;
    let name = sender.character_name.as_bytes();
    let mut arrived = vec![ARRIVED, name.len() as u8, 0];
    arrived.extend_from_slice(name);
    arrived.push(1);
    assert_eq!(seen.parcels, [arrived]);

    assert_eq!(warp(r, HENESYS, "", &mut seen).await, HENESYS);
    let (listed, _) = open(r, &mut seen).await;
    let [parcel] = &listed[..] else {
        panic!("one parcel: {listed:?}");
    };
    assert_eq!(
        (
            parcel.mesos,
            parcel.message.as_str(),
            parcel.quick,
            parcel.item
        ),
        (200_000, "for you", true, None)
    );
    let t = step(r, "duey-delete", with_id(5, parcel.id)).await;
    let id = parcel.id.to_le_bytes();
    assert_eq!(
        t.parcels,
        [vec![REMOVED, id[0], id[1], id[2], id[3], DELETED]]
    );

    // Closed and opened again, the window is empty.
    send(r, action(7)).await;
    assert_eq!(open(r, &mut seen).await, (Vec::new(), 0));
}
