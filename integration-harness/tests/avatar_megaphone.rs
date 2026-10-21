//! Avatar megaphones: the banner goes to every channel, one at a time for
//! 10 s each; one that would wait past 15 s is refused with 'S' and kept,
//! as is one used at level 10 or below ('T').

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_stack, set_job};
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
const AVATAR_MEGAPHONE: i32 = 5_390_000;

/// Read until a packet with `op` within `secs`, and return its body.
async fn next(c: &mut MapleTestConnection, op: SendOpcode, secs: u64) -> Vec<u8> {
    let op = op as i16;
    let deadline = Instant::now() + Duration::from_secs(secs);
    loop {
        let p = timeout(
            deadline.saturating_duration_since(Instant::now()),
            c.read_packet("avatar megaphone"),
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

/// USE_CASH_ITEM with an avatar megaphone in Cash `slot`: four lines, the
/// ear, a look the server ignores, then the tick.
fn use_avatar_megaphone(slot: i16, lines: [&str; 4]) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseCashItem as i16).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(AVATAR_MEGAPHONE).unwrap();
    for line in lines {
        p.write_str_with_length(line).unwrap();
    }
    p.write_byte(1).unwrap();
    p.write_bytes(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF])
        .unwrap();
    p.write_bytes(&[0; 16]).unwrap();
    p.write_int(0).unwrap();
    p
}

/// The item id, sender, lines and channel of a banner.
fn banner(body: &[u8]) -> (i32, String, [String; 4], i32) {
    let mut r = Cursor::new(body);
    let item = r.read_int().unwrap();
    let name = r.read_str_with_length().unwrap();
    let lines = [(); 4].map(|_| r.read_str_with_length().unwrap());
    (item, name, lines, r.read_int().unwrap())
}

#[tokio::test]
async fn avatar_banners_reach_every_channel_in_turn() {
    let config = load_harness_config_or_fail().await.with_channel(0, 0);
    let id = login_to_world_session(&config)
        .await
        .expect("speaker login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 30, 50).await.expect("level 30");
    for slot in 1..=3 {
        grant_stack(id, CASH, slot, AVATAR_MEGAPHONE, 1)
            .await
            .expect("grant an avatar megaphone");
    }
    let mut speaker = relogin(&config).await;
    let name = speaker.character_name.clone();

    let low_config = load_harness_config_or_fail().await.with_channel(0, 0);
    let low_id = login_to_world_session(&low_config)
        .await
        .expect("low login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(low_id, 0, 10, 50).await.expect("level 10");
    grant_stack(low_id, CASH, 1, AVATAR_MEGAPHONE, 1)
        .await
        .expect("grant an avatar megaphone");
    let mut low = relogin(&low_config).await;

    let far_config = load_harness_config_or_fail().await.with_channel(0, 1);
    let mut far = login_to_world_session(&far_config).await.unwrap();
    let (s, l, f) = (
        &mut speaker.connection,
        &mut low.connection,
        &mut far.connection,
    );

    // Level 10 is too low: refused with 'T', and nothing shows.
    send(l, use_avatar_megaphone(1, ["too", "low", "", ""])).await;
    assert_eq!(next(l, SendOpcode::AvatarMegaphoneResult, 5).await, [b'T']);

    // The first banner shows at once on every channel, the megaphone used.
    send(s, use_avatar_megaphone(1, ["hello", "", "world", ""])).await;
    next(s, SendOpcode::InventoryOperation, 5).await;
    let first = banner(&next(f, SendOpcode::AvatarMegaphone, 5).await);
    let shown = Instant::now();
    assert_eq!(
        first,
        (
            AVATAR_MEGAPHONE,
            name.clone(),
            ["hello", "", "world", ""].map(String::from),
            0
        )
    );
    next(s, SendOpcode::AvatarMegaphone, 5).await;

    // The second waits its turn, used up all the same; a third would wait
    // past 15 s and is refused.
    send(s, use_avatar_megaphone(2, ["second", "", "", ""])).await;
    next(s, SendOpcode::InventoryOperation, 5).await;
    send(s, use_avatar_megaphone(3, ["third", "", "", ""])).await;
    assert_eq!(next(s, SendOpcode::AvatarMegaphoneResult, 5).await, [b'S']);

    let second = banner(&next(f, SendOpcode::AvatarMegaphone, 15).await);
    assert_eq!(second.2[0], "second");
    let waited = shown.elapsed();
    assert!(
        waited >= Duration::from_secs(9),
        "the second banner came after only {waited:?}"
    );

    // Its turn over, the line is empty again: the kept megaphone works.
    tokio::time::sleep(Duration::from_secs(10)).await;
    send(s, use_avatar_megaphone(3, ["third", "", "", ""])).await;
    next(s, SendOpcode::InventoryOperation, 5).await;
    let third = banner(&next(f, SendOpcode::AvatarMegaphone, 5).await);
    assert_eq!(third.2[0], "third");
}
