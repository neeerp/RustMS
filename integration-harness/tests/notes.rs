//! Notes (MEMO_RESULT, NOTE_ACTION): the note item leaves an offline
//! character a note, which they get on entering the game and lose once read.
//! A gift's thank-you is a note that raises its reader's fame.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{gift_cash_item, grant_stack};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::io::Cursor;
use tokio::time::Duration;

const CASH: i16 = 5;
const NOTE: i32 = 5_090_000;
const FAME: i32 = 0x20000;

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

/// USE_CASH_ITEM with the note item in Cash slot 1.
fn write_note(to: &str, text: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(1).unwrap();
    p.write_int(NOTE).unwrap();
    p.write_str_with_length(to).unwrap();
    p.write_str_with_length(text).unwrap();
    p.write_int(0).unwrap();
    p
}

fn note_action(mode: u8, body: impl FnOnce(&mut Packet)) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::NoteAction as i16).unwrap();
    p.write_byte(mode).unwrap();
    body(&mut p);
    p
}

/// Read every note in a list (mode 3): its id, sender, text and flag.
fn notes(list: &[u8]) -> Vec<(i32, String, String, u8)> {
    let mut r = Cursor::new(list);
    assert_eq!(r.read_byte().unwrap(), 3, "a list");
    (0..r.read_byte().unwrap())
        .map(|_| {
            let id = r.read_int().unwrap();
            let sender = r.read_str_with_length().unwrap();
            let text = r.read_str_with_length().unwrap();
            r.read_long().unwrap();
            (id, sender, text, r.read_byte().unwrap())
        })
        .collect()
}

/// Read the notes `ids`, none of them an invitation.
fn read_notes(ids: &[i32]) -> Packet {
    note_action(1, |p| {
        p.write_bytes(&[ids.len() as u8, 0, 0]).unwrap();
        for &id in ids {
            p.write_int(id).unwrap();
            p.write_byte(0).unwrap();
        }
    })
}

#[tokio::test]
async fn a_note_waits_for_its_offline_receiver_until_read() {
    let receiver_config = load_harness_config_or_fail().await;
    let receiver_name = login_to_world_session(&receiver_config)
        .await
        .expect("receiver login failed")
        .character_name;
    let sender_config = load_harness_config_or_fail().await;
    let sender_id = login_to_world_session(&sender_config)
        .await
        .expect("sender login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(sender_id, CASH, 1, NOTE, 2)
        .await
        .expect("grant notes");
    let mut sender = relogin(&sender_config).await;
    let (s, mut st) = (&mut sender.connection, Trace::default());

    // Nobody by that name, and the item stays.
    send(s, write_note("nobodyhere", "hello")).await;
    assert_eq!(next(s, &mut st, SendOpcode::MemoResult).await, [5, 1]);

    // Someone online is to be whispered instead.
    let receiver = relogin(&receiver_config).await;
    send(s, write_note(&receiver_name, "hello")).await;
    assert_eq!(next(s, &mut st, SendOpcode::MemoResult).await, [5, 0]);
    drop(receiver);
    tokio::time::sleep(Duration::from_millis(500)).await;

    send(s, write_note(&receiver_name.to_lowercase(), "hello there")).await;
    let used = next(s, &mut st, SendOpcode::InventoryOperation).await;
    assert_eq!(used[..2], [1, 1], "unlocked, one change");
    assert_eq!(next(s, &mut st, SendOpcode::MemoResult).await, [4]);

    // The receiver finds it on entering the game, and again when asking.
    let mut receiver = relogin(&receiver_config).await;
    let (r, mut rt) = (&mut receiver.connection, Trace::default());
    let waiting = notes(&next(r, &mut rt, SendOpcode::MemoResult).await);
    let [(id, from, text, 0)] = &waiting[..] else {
        panic!("one plain note: {waiting:?}");
    };
    assert_eq!(
        (from, text.as_str()),
        (&sender.character_name, "hello there")
    );
    send(r, note_action(2, |_| {})).await;
    assert_eq!(
        notes(&next(r, &mut rt, SendOpcode::MemoResult).await),
        waiting
    );

    // Once read, it's gone.
    send(r, read_notes(&[*id])).await;
    send(r, note_action(2, |_| {})).await;
    assert_eq!(next(r, &mut rt, SendOpcode::MemoResult).await, [3, 0]);
}

#[tokio::test]
async fn thanks_for_a_gift_raise_the_givers_fame_once() {
    let giver_config = load_harness_config_or_fail().await;
    let mut giver = login_to_world_session(&giver_config)
        .await
        .expect("giver login failed");
    let receiver_config = load_harness_config_or_fail().await;
    let receiver_id = login_to_world_session(&receiver_config)
        .await
        .expect("receiver login failed")
        .character_id;
    let gift = gift_cash_item(receiver_id, NOTE, &giver.character_name)
        .await
        .expect("gift an item");
    let mut receiver = relogin(&receiver_config).await;
    let r = &mut receiver.connection;
    let (g, mut gt) = (&mut giver.connection, Trace::default());
    let thank = |to: &str, gift: i64| {
        let to = to.to_string();
        note_action(0, move |p| {
            p.write_str_with_length(&to).unwrap();
            p.write_str_with_length("thank you").unwrap();
            p.write_byte(1).unwrap();
            p.write_int(0).unwrap();
            p.write_long(gift).unwrap();
        })
    };

    // Only the gift's sender can be thanked, and they hear of it at once.
    send(r, thank("Somebody", gift)).await;
    send(r, thank(&giver.character_name, gift)).await;
    assert_eq!(next(g, &mut gt, SendOpcode::MemoResult).await, [7]);
    send(g, note_action(2, |_| {})).await;
    let waiting = notes(&next(g, &mut gt, SendOpcode::MemoResult).await);
    let [(id, from, text, 1)] = &waiting[..] else {
        panic!("one thank-you: {waiting:?}");
    };
    assert_eq!(
        (from, text.as_str()),
        (&receiver.character_name, "thank you")
    );

    send(g, read_notes(&[*id])).await;
    next(g, &mut gt, SendOpcode::StatChange).await;
    assert_eq!(gt.stat(FAME), Some(1));

    // A gift is thanked for once.
    send(r, thank(&giver.character_name, gift)).await;
    send(g, note_action(2, |_| {})).await;
    assert_eq!(next(g, &mut gt, SendOpcode::MemoResult).await, [3, 0]);
}
