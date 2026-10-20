//! Mounts (report 201): a player with Monster Rider, a Hog and a saddle
//! gets on, the map sees them riding, a Revitalizer rests the Hog, and they
//! get off. The Hog's level, EXP and tiredness outlive the session.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_skill, grant_stack, set_job, set_mount, wear_equip};
use integration_harness::preconditions::load_multi_harness_config_or_fail;
use integration_harness::trace::{send, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::io::Cursor;
use tokio::time::{timeout, Duration};

const HENESYS: i32 = 100_000_000;
const MONSTER_RIDER: i32 = 1004;
const HOG: i32 = 1902000;
const SADDLE: i32 = 1912000;
const REVITALIZER: i32 = 2260000;
/// Riding's flag, in the first half of a buff mask.
const RIDING: i64 = 1 << 53;

/// Read until `pick` takes a packet, failing after 5 seconds.
async fn expect<T>(c: &mut MapleTestConnection, pick: impl Fn(&Packet) -> Option<T>) -> T {
    loop {
        let p = timeout(Duration::from_secs(5), c.read_packet("mounts"))
            .await
            .expect("timed out")
            .expect("read packet")
            .packet;
        if let Some(picked) = pick(&p) {
            return picked;
        }
    }
}

/// The body of `p` if it is an `op`.
fn body(p: &Packet, op: SendOpcode) -> Option<Cursor<&Vec<u8>>> {
    if p.opcode() != op as i16 {
        return None;
    }
    let mut r = Cursor::new(&p.bytes);
    r.read_short().unwrap();
    Some(r)
}

/// SET_TAMING_MOB_INFO: (character, level, EXP, tiredness, level up).
fn mount_info(p: &Packet) -> Option<(i32, i32, i32, i32, bool)> {
    let mut r = body(p, SendOpcode::SetTamingMobInfo)?;
    Some((
        r.read_int().unwrap(),
        r.read_int().unwrap(),
        r.read_int().unwrap(),
        r.read_int().unwrap(),
        r.read_byte().unwrap() == 1,
    ))
}

/// The (mount, skill) after a riding mask and the defense bytes.
fn ride(r: &mut Cursor<&Vec<u8>>) -> Option<(i32, i32)> {
    if r.read_long().unwrap() != RIDING {
        return None;
    }
    r.read_long().unwrap();
    r.read_short().unwrap();
    Some((r.read_int().unwrap(), r.read_int().unwrap()))
}

fn use_skill(skill_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseSkill as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_int(skill_id).unwrap();
    p.write_byte(1).unwrap();
    p.write_short(0).unwrap();
    p
}

fn cancel_buff(skill_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::CancelBuff as i16).unwrap();
    p.write_int(skill_id).unwrap();
    p
}

fn mount_food(slot: i16, item_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseMountFood as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(item_id).unwrap();
    p
}

#[tokio::test]
async fn a_rider_gets_on_feeds_and_gets_off_their_hog() {
    let config = load_multi_harness_config_or_fail().await;
    let rider_config = config.player("sender").unwrap();
    let watcher_config = config.player("recipient").unwrap();

    // Make the rider, then give them Monster Rider, a Hog with a saddle, a
    // tired Hog and Revitalizers while logged out.
    let rider_id = login_to_world_session(&rider_config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&rider_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(rider_id, 0, 70, 100).await.expect("level up");
    grant_skill(rider_id, MONSTER_RIDER, 1)
        .await
        .expect("teach Monster Rider");
    wear_equip(rider_id, -18, HOG).await.expect("wear the Hog");
    wear_equip(rider_id, -19, SADDLE).await.expect("saddle it");
    grant_stack(rider_id, 2, 1, REVITALIZER, 2)
        .await
        .expect("grant Revitalizers");
    set_mount(rider_id, 1, 20, 45).await.expect("tire the Hog");

    // Coming in on a Hog, the rider learns its state.
    let mut rider = relogin(&rider_config).await;
    assert_eq!(
        expect(&mut rider.connection, mount_info).await,
        (rider_id, 1, 20, 45, false)
    );
    let mut r = Trace::default();
    assert_eq!(
        warp(&mut rider.connection, HENESYS, "", &mut r).await,
        HENESYS
    );

    // Getting on answers with riding's data.
    send(&mut rider.connection, use_skill(MONSTER_RIDER)).await;
    let got_on = expect(&mut rider.connection, |p| {
        ride(&mut body(p, SendOpcode::GiveBuff)?)
    })
    .await;
    assert_eq!(got_on, (HOG, MONSTER_RIDER));

    // Someone coming onto the map sees them riding.
    let mut watcher = login_to_world_session(&watcher_config)
        .await
        .expect("watcher login failed");
    let mut w = Trace::default();
    assert_eq!(
        warp(&mut watcher.connection, HENESYS, "", &mut w).await,
        HENESYS
    );
    let seen = expect(&mut watcher.connection, |p| {
        let r = body(p, SendOpcode::SpawnPlayer)?;
        let bytes = &r.get_ref()[2..];
        (bytes[..4] == rider_id.to_le_bytes()).then(|| {
            let ride = [HOG.to_le_bytes(), MONSTER_RIDER.to_le_bytes()].concat();
            bytes.windows(8).any(|w| w == ride)
        })
    })
    .await;
    assert!(seen, "the spawn carries the Hog and Monster Rider");

    // A Revitalizer takes 30 tiredness off for 8 EXP, a new level; the map
    // sees it too.
    send(&mut rider.connection, mount_food(1, REVITALIZER)).await;
    assert_eq!(
        expect(&mut rider.connection, mount_info).await,
        (rider_id, 2, 28, 15, true)
    );
    assert_eq!(
        expect(&mut watcher.connection, mount_info).await,
        (rider_id, 2, 28, 15, true)
    );

    // Getting off is confirmed to the rider and shown to the map.
    send(&mut rider.connection, cancel_buff(MONSTER_RIDER)).await;
    expect(&mut rider.connection, |p| {
        let mut r = body(p, SendOpcode::CancelBuff)?;
        (r.read_long().unwrap() == RIDING).then_some(())
    })
    .await;
    let off = expect(&mut watcher.connection, |p| {
        let mut r = body(p, SendOpcode::CancelForeignBuff)?;
        let who = r.read_int().unwrap();
        (r.read_long().unwrap() == RIDING).then_some(who)
    })
    .await;
    assert_eq!(off, rider_id);

    // Off the Hog, food is refused.
    send(&mut rider.connection, mount_food(1, REVITALIZER)).await;
    expect(&mut rider.connection, |p| {
        (p.opcode() == SendOpcode::StatChange as i16).then_some(())
    })
    .await;

    // The Hog is as it was left next time.
    drop(rider);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut rider = relogin(&rider_config).await;
    assert_eq!(
        expect(&mut rider.connection, mount_info).await,
        (rider_id, 2, 28, 15, false)
    );
}
