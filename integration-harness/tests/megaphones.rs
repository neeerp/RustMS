//! Megaphones: a plain megaphone speaks to the speaker's channel, the super,
//! item and triple ones to every channel. Each is used up.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_stack, set_job};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::io::Cursor;

const CASH: i16 = 5;
const USE: i16 = 2;
const MEGAPHONE: i32 = 5_071_000;
const SUPER_MEGAPHONE: i32 = 5_072_000;
const ITEM_MEGAPHONE: i32 = 5_076_000;
const TRIPLE_MEGAPHONE: i32 = 5_077_000;
const RED_POTION: i32 = 2_000_000;

/// Read until a packet with `op`, and return its body.
async fn next(c: &mut MapleTestConnection, t: &mut Trace, op: SendOpcode) -> Vec<u8> {
    let op = op as i16;
    loop {
        let p = read(c, t).await;
        if p.opcode() == op {
            return p.bytes[2..].to_vec();
        }
    }
}

/// USE_CASH_ITEM: the item in Cash `slot`, its fields, then the tick.
fn use_cash_item(slot: i16, item_id: i32, body: impl FnOnce(&mut Packet)) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(item_id).unwrap();
    body(&mut p);
    p.write_int(0).unwrap();
    p
}

#[tokio::test]
async fn megaphones_reach_their_channel_or_every_channel() {
    let config = load_harness_config_or_fail().await.with_channel(0, 0);
    let id = login_to_world_session(&config)
        .await
        .expect("speaker login failed")
        .character_id;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    set_job(id, 0, 30, 50).await.expect("level 30");
    for (slot, item) in [
        (1, MEGAPHONE),
        (2, SUPER_MEGAPHONE),
        (3, ITEM_MEGAPHONE),
        (4, TRIPLE_MEGAPHONE),
    ] {
        grant_stack(id, CASH, slot, item, 1)
            .await
            .expect("grant a megaphone");
    }
    grant_stack(id, USE, 1, RED_POTION, 5)
        .await
        .expect("grant a potion");
    let mut speaker = relogin(&config).await;
    let name = speaker.character_name.clone();
    let near_config = load_harness_config_or_fail().await.with_channel(0, 0);
    let mut near = login_to_world_session(&near_config).await.unwrap();
    let far_config = load_harness_config_or_fail().await.with_channel(0, 1);
    let mut far = login_to_world_session(&far_config).await.unwrap();
    let (s, mut st) = (&mut speaker.connection, Trace::default());
    let (n, mut nt) = (&mut near.connection, Trace::default());
    let (f, mut ft) = (&mut far.connection, Trace::default());

    // A plain megaphone reaches the speaker's channel, the speaker included.
    send(
        s,
        use_cash_item(1, MEGAPHONE, |p| {
            p.write_str_with_length("hello channel").unwrap();
        }),
    )
    .await;
    let used = next(s, &mut st, SendOpcode::InventoryOperation).await;
    assert_eq!(used[..2], [1, 1], "unlocked, one change");
    let line = format!("{name} : hello channel");
    let mut expected = vec![2];
    expected.extend((line.len() as u16).to_le_bytes());
    expected.extend(line.as_bytes());
    assert_eq!(next(s, &mut st, SendOpcode::BroadcastMsg).await, expected);
    assert_eq!(next(n, &mut nt, SendOpcode::BroadcastMsg).await, expected);

    // The super megaphone reaches the other channel, which never saw the
    // plain one, with the speaker's channel and ear.
    send(
        s,
        use_cash_item(2, SUPER_MEGAPHONE, |p| {
            p.write_str_with_length("hello world").unwrap();
            p.write_byte(1).unwrap();
        }),
    )
    .await;
    let heard = next(f, &mut ft, SendOpcode::BroadcastMsg).await;
    let mut r = Cursor::new(&heard);
    assert_eq!(r.read_byte().unwrap(), 3, "a super megaphone");
    assert_eq!(
        r.read_str_with_length().unwrap(),
        format!("{name} : hello world")
    );
    assert_eq!((r.read_byte().unwrap(), r.read_byte().unwrap()), (0, 1));

    // The item megaphone shows the potion.
    send(
        s,
        use_cash_item(3, ITEM_MEGAPHONE, |p| {
            p.write_str_with_length("see this").unwrap();
            p.write_byte(0).unwrap();
            p.write_byte(1).unwrap();
            p.write_int(USE.into()).unwrap();
            p.write_int(1).unwrap();
        }),
    )
    .await;
    let heard = next(f, &mut ft, SendOpcode::BroadcastMsg).await;
    let mut r = Cursor::new(&heard);
    assert_eq!(r.read_byte().unwrap(), 8, "an item megaphone");
    assert_eq!(
        r.read_str_with_length().unwrap(),
        format!("{name} : see this")
    );
    assert_eq!(r.read_bytes(3).unwrap(), [0, 0, 1], "channel, ear, an item");
    assert_eq!(r.read_byte().unwrap(), 2, "a stackable item");
    assert_eq!(r.read_int().unwrap(), RED_POTION);

    // The triple megaphone names the speaker on every line.
    send(
        s,
        use_cash_item(4, TRIPLE_MEGAPHONE, |p| {
            p.write_byte(2).unwrap();
            p.write_str_with_length("one").unwrap();
            p.write_str_with_length("two").unwrap();
            p.write_byte(0).unwrap();
        }),
    )
    .await;
    let heard = next(f, &mut ft, SendOpcode::BroadcastMsg).await;
    let mut r = Cursor::new(&heard);
    assert_eq!(r.read_byte().unwrap(), 10, "a triple megaphone");
    assert_eq!(r.read_str_with_length().unwrap(), format!("{name} : one"));
    assert_eq!(r.read_byte().unwrap(), 2, "two lines");
    assert_eq!(r.read_str_with_length().unwrap(), format!("{name} : two"));
    assert_eq!((r.read_byte().unwrap(), r.read_byte().unwrap()), (0, 0));

    // Each was used up: the megaphone's slot is empty now.
    send(
        s,
        use_cash_item(1, MEGAPHONE, |p| {
            p.write_str_with_length("again").unwrap();
        }),
    )
    .await;
    let unlocked = next(s, &mut st, SendOpcode::StatChange).await;
    assert_eq!(unlocked[0], 1, "an empty stat change unlocks the client");
}
