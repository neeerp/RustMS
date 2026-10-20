//! A Corsair's Homing Beacon on Snail Hunting Ground II (40001): the shot
//! marks the mob it hits (GIVE_BUFF GuidedBullet, bit 87), a shot at
//! another mob moves the mark, and leaving the map takes it off.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_skill, grant_stack, replace_item, set_job};
use integration_harness::hunting::ranged_attack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::SendOpcode;
use tokio::time::Duration;

const HUNTING_GROUND: i32 = 40_001;
const HENESYS: i32 = 100_000_000;
const SNAIL: i32 = 100100;
const HOMING_BEACON: i32 = 5_211_006;
const GUN: i32 = 1_492_014;
const BULLETS: i32 = 2_330_000;
const USE: i16 = 2;
const SLOT: i16 = 1;

/// GuidedBullet's flags: bit 87 in the first long.
const MASK: [u8; 16] = [0, 0, 0, 0, 0, 0, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0];

/// Read until the next GIVE_BUFF or CANCEL_BUFF for the mark, and return
/// the mob it marks, or `None` for a cancel.
async fn mark(c: &mut MapleTestConnection, t: &mut Trace) -> Option<i32> {
    let give = SendOpcode::GiveBuff as i16;
    let cancel = SendOpcode::CancelBuff as i16;
    loop {
        let p = read(c, t).await;
        let body = &p.bytes[2..];
        if !body.starts_with(&MASK) {
            continue;
        }
        if p.opcode() == cancel {
            return None;
        }
        if p.opcode() == give {
            // Defense, then 1, the skill, the start, and the mob.
            let data = &body[18..];
            assert_eq!(data[..4], 1i32.to_le_bytes());
            assert_eq!(data[4..8], HOMING_BEACON.to_le_bytes());
            return Some(i32::from_le_bytes(data[13..17].try_into().unwrap()));
        }
    }
}

#[tokio::test]
async fn a_beacon_marks_its_mob_until_the_player_leaves() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 522, 120, 5000).await.expect("corsair");
    grant_skill(id, HOMING_BEACON, 30).await.expect("beacon");
    replace_item(id, -1, -11, GUN).await.expect("a gun");
    grant_stack(id, USE, SLOT, BULLETS, 500)
        .await
        .expect("bullets");
    let mut corsair = relogin(&config).await;
    let (c, mut t) = (&mut corsair.connection, Trace::default());
    assert_eq!(warp(c, HUNTING_GROUND, "", &mut t).await, HUNTING_GROUND);
    until(c, &mut t, |t| t.field.alive(SNAIL).len() >= 2).await;
    let snails = t.field.alive(SNAIL);

    // Each shot kills its 8 HP snail, leaving none hurt for other tests.
    send(c, ranged_attack(HOMING_BEACON, SLOT, snails[0], 8)).await;
    assert_eq!(mark(c, &mut t).await, Some(snails[0]));

    // A new mob: the old mark comes off first.
    send(c, ranged_attack(HOMING_BEACON, SLOT, snails[1], 8)).await;
    assert_eq!(mark(c, &mut t).await, None);
    assert_eq!(mark(c, &mut t).await, Some(snails[1]));

    assert_eq!(warp(c, HENESYS, "", &mut t).await, HENESYS);
    assert_eq!(mark(c, &mut t).await, None);
}
