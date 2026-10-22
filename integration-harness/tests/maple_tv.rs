//! Maple TV: a message goes up at once on the sender's channel with the
//! receiver's look beside the sender's, the next waits its turn, a player
//! who comes in sees the one up, and a heart for no one online is refused
//! and kept. A heart may be for oneself.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::grant_stack;
use integration_harness::packets::decode_look;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::send;
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::io::Cursor;
use std::time::{Duration, Instant};
use tokio::time::timeout;

const CASH: i16 = 5;
const MESSENGER: i32 = 5_075_000;
const HEART: i32 = 5_075_002;
const STAR_MEGASSENGER: i32 = 5_075_004;

/// Read until a packet with `op` within `secs`, and return its body.
async fn next(c: &mut MapleTestConnection, op: SendOpcode, secs: u64) -> Vec<u8> {
    let op = op as i16;
    let deadline = Instant::now() + Duration::from_secs(secs);
    loop {
        let p = timeout(
            deadline.saturating_duration_since(Instant::now()),
            c.read_packet("maple tv"),
        )
        .await
        .unwrap_or_else(|_| panic!("no {op:#x} within {secs} s"))
        .expect("read packet")
        .packet;
        if p.opcode() == op {
            return p.bytes[2..].to_vec();
        }
    }
}

/// Read until a packet with each of `ops` has come, in any order, within
/// `secs`, and return their bodies in the order of `ops`.
async fn each(c: &mut MapleTestConnection, ops: [SendOpcode; 3], secs: u64) -> Vec<Vec<u8>> {
    let ops = ops.map(|op| op as i16);
    let deadline = Instant::now() + Duration::from_secs(secs);
    let mut bodies: Vec<Option<Vec<u8>>> = vec![None; ops.len()];
    while bodies.iter().any(Option::is_none) {
        let p = timeout(
            deadline.saturating_duration_since(Instant::now()),
            c.read_packet("maple tv"),
        )
        .await
        .unwrap_or_else(|_| panic!("not all of {ops:#x?} within {secs} s"))
        .expect("read packet")
        .packet;
        if let Some(i) = ops.iter().position(|&op| op == p.opcode()) {
            bodies[i].get_or_insert_with(|| p.bytes[2..].to_vec());
        }
    }
    bodies.into_iter().flatten().collect()
}

/// USE_CASH_ITEM with a Maple TV in Cash `slot`: `head` (mode, ear,
/// receiver as the kind has them), five lines, then the tick.
fn use_tv(slot: i16, item_id: i32, head: impl FnOnce(&mut Packet), lines: [&str; 5]) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(item_id).unwrap();
    head(&mut p);
    for line in lines {
        p.write_str_with_length(line).unwrap();
    }
    p.write_int(0).unwrap();
    p
}

/// What a SEND_TV shows: flag, screen, sender, receiver, lines, the wait,
/// and whether a partner's look follows.
struct Shown {
    flag: u8,
    screen: u8,
    sender: String,
    receiver: String,
    lines: [String; 5],
    wait: i32,
    partner: bool,
}

fn shown(body: &[u8]) -> Shown {
    let mut r = Cursor::new(body);
    let flag = r.read_byte().unwrap();
    let screen = r.read_byte().unwrap();
    decode_look(&mut r).unwrap();
    let sender = r.read_str_with_length().unwrap();
    let receiver = r.read_str_with_length().unwrap();
    let lines = [(); 5].map(|_| r.read_str_with_length().unwrap());
    let wait = r.read_int().unwrap();
    let partner = flag & 2 != 0;
    if partner {
        decode_look(&mut r).unwrap();
    }
    assert_eq!(r.position() as usize, body.len(), "nothing after the looks");
    Shown {
        flag,
        screen,
        sender,
        receiver,
        lines,
        wait,
        partner,
    }
}

#[tokio::test]
async fn maple_tv_shows_one_message_at_a_time() {
    let config = load_harness_config_or_fail().await.with_channel(0, 0);
    let id = login_to_world_session(&config)
        .await
        .expect("sender login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    for (slot, item) in [
        (1, MESSENGER),
        (2, HEART),
        (3, STAR_MEGASSENGER),
        (4, HEART),
    ] {
        grant_stack(id, CASH, slot, item, 1)
            .await
            .expect("grant a Maple TV");
    }
    let mut sender = relogin(&config).await;
    let name = sender.character_name.clone();
    let near_config = load_harness_config_or_fail().await.with_channel(0, 0);
    let mut near = login_to_world_session(&near_config).await.unwrap();
    let friend = near.character_name.clone();
    let (s, n) = (&mut sender.connection, &mut near.connection);

    // A heart for no one online is refused with "wrong name" and kept.
    send(
        s,
        use_tv(
            2,
            HEART,
            |p| {
                p.write_str_with_length("NoSuchPlayer").unwrap();
            },
            ["for", "no one", "", "", ""],
        ),
    )
    .await;
    assert_eq!(next(s, SendOpcode::TvResult, 5).await, [1, 2]);
    assert_eq!(next(s, SendOpcode::StatChange, 5).await[0], 1, "unlocked");

    // The messenger for `near` goes up at once on the channel, both looks
    // shown; nothing waits behind it.
    send(
        s,
        use_tv(
            1,
            MESSENGER,
            |p| {
                p.write_byte(3).unwrap();
                p.write_str_with_length(&friend).unwrap();
            },
            ["hello", "", "there", "", ""],
        ),
    )
    .await;
    // The sender's actor hears it went in line after the directory has
    // shown it, so the TV may come first.
    let got = each(
        s,
        [
            SendOpcode::TvResult,
            SendOpcode::InventoryOperation,
            SendOpcode::SendTv,
        ],
        5,
    )
    .await;
    assert_eq!(got[0], [0]);
    shown(&got[2]);
    let up = shown(&next(n, SendOpcode::SendTv, 5).await);
    let up_at = Instant::now();
    assert_eq!((up.flag, up.screen, up.partner), (3, 0, true));
    assert_eq!(
        (up.sender.as_str(), up.receiver.as_str()),
        (name.as_str(), friend.as_str())
    );
    assert_eq!(up.lines, ["hello", "", "there", "", ""].map(String::from));
    assert!(up.wait <= 15, "only its own time to wait: {}", up.wait);

    // A star megassenger with the ear waits its turn, then reaches the
    // other channel too.
    let far_config = load_harness_config_or_fail().await.with_channel(0, 1);
    let mut far = login_to_world_session(&far_config).await.unwrap();
    let f = &mut far.connection;
    send(
        s,
        use_tv(
            3,
            STAR_MEGASSENGER,
            |p| {
                p.write_byte(1).unwrap();
            },
            ["", "", "everyone", "", ""],
        ),
    )
    .await;
    assert_eq!(next(s, SendOpcode::TvResult, 5).await, [0]);

    // A heart to oneself waits its turn too: the sender's own look is
    // asked for while the sender waits.
    send(
        s,
        use_tv(
            4,
            HEART,
            |p| {
                p.write_str_with_length(&name).unwrap();
            },
            ["to me", "", "", "", ""],
        ),
    )
    .await;
    assert_eq!(next(s, SendOpcode::TvResult, 5).await, [0]);

    // A player coming in on the channel sees the messenger still up.
    let late_config = load_harness_config_or_fail().await.with_channel(0, 0);
    let mut late = login_to_world_session(&late_config).await.unwrap();
    let seen = shown(&next(&mut late.connection, SendOpcode::SendTv, 5).await);
    assert_eq!((seen.screen, seen.lines[0].as_str()), (0, "hello"));

    next(n, SendOpcode::RemoveTv, 20).await;
    assert!(up_at.elapsed() >= Duration::from_secs(13), "shown its 15 s");
    let star = shown(&next(f, SendOpcode::SendTv, 5).await);
    assert_eq!((star.flag, star.screen, star.partner), (1, 1, false));
    assert_eq!(
        (star.sender.as_str(), star.receiver.as_str()),
        (name.as_str(), "")
    );
    assert_eq!(star.lines[2], "everyone");
    shown(&next(n, SendOpcode::SendTv, 5).await);
}
