//! Poison Bomb and Flame Gear on Snail Hunting Ground III (40002), away
//! from the mist tests of 40001: the Night Walker's cloud falls where the
//! grenade went off, and the Blaze Wizard's at the caster; each lasts its
//! `time`, and each test waits for its clouds to lift.

use integration_harness::database::{grant_skill, set_job};
use integration_harness::hunting::{magic_attack, walk_to, MapMist};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::{Duration, Instant};

const HUNTING_GROUND: i32 = 40_002;
const SNAIL: i32 = 100100;
const NIGHT_WALKER: i16 = 1411;
const BLAZE_WIZARD: i16 = 1211;
const POISON_BOMB: i32 = 14111006;
const FLAME_GEAR: i32 = 12111005;

/// Poison Bomb's CLOSE_RANGE_ATTACK facing left, held 500 ms, hitting
/// `object_id` once, then where the player stands and where the grenade
/// went off (v83 0x952b58, 0x952b89).
fn poison_bomb(object_id: i32, at: (i16, i16), grenade: (i16, i16)) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::CloseRangeAttack as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_byte(0x11).unwrap();
    p.write_int(POISON_BOMB).unwrap();
    p.write_bytes(&[0; 8]).unwrap();
    p.write_int(500).unwrap();
    p.write_byte(0).unwrap();
    p.write_short(0x8005u16 as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_byte(4).unwrap();
    p.write_bytes(&[0; 4]).unwrap();
    p.write_int(object_id).unwrap();
    p.write_bytes(&[0; 14]).unwrap();
    p.write_int(1).unwrap();
    p.write_bytes(&[0; 4]).unwrap();
    for value in [at.0, at.1, grenade.0, grenade.1] {
        p.write_short(value).unwrap();
    }
    p
}

/// The clouds of `skill_id` seen so far; the other test's share the map.
fn mine(t: &Trace, skill_id: i32) -> Vec<MapMist> {
    t.field
        .mists
        .iter()
        .copied()
        .filter(|m| m.skill_id == skill_id)
        .collect()
}

/// A knight of `job` with `skill_id` at level 1, on the hunting ground
/// beside a snail: their session, trace, the snail and where they stand.
async fn knight_by_a_snail(
    job: i16,
    skill_id: i32,
) -> (integration_harness::WorldSession, Trace, i32, (i16, i16)) {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, job, 70, 2000).await.expect("make a knight");
    grant_skill(id, skill_id, 1).await.expect("teach the skill");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, HUNTING_GROUND, "", &mut t).await, HUNTING_GROUND);
    until(c, &mut t, |t| !t.field.alive(SNAIL).is_empty()).await;
    let snail = t.field.alive(SNAIL)[0];
    let at = t.field.mob_positions[&snail];
    send(c, walk_to(at.0, at.1)).await;
    (session, t, snail, at)
}

#[tokio::test]
async fn poison_bombs_cloud_falls_where_the_grenade_went_off() {
    let (mut session, mut t, snail, (x, y)) = knight_by_a_snail(NIGHT_WALKER, POISON_BOMB).await;
    let c = &mut session.connection;
    let (gx, gy) = (x + 150, y - 10);
    let cast = Instant::now();
    send(c, poison_bomb(snail, (x, y), (gx, gy))).await;
    until(c, &mut t, |t| mine(t, POISON_BOMB).len() == 1).await;
    let mist = mine(&t, POISON_BOMB)[0];
    assert_eq!((mist.kind, mist.level), (1, 1));
    let (gx, gy) = (i32::from(gx), i32::from(gy));
    assert_eq!(mist.area, (gx - 100, gy - 82, gx + 100, gy + 83));

    // A spot too far from the thrower leaves the cloud on them.
    send(c, poison_bomb(snail, (x, y), (x + 1500, y))).await;
    until(c, &mut t, |t| mine(t, POISON_BOMB).len() == 2).await;
    let far = mine(&t, POISON_BOMB)[1];
    let (x, y) = (i32::from(x), i32::from(y));
    assert_eq!(far.area, (x - 100, y - 82, x + 100, y + 83));

    until(c, &mut t, |t| {
        t.field.mists_removed.contains(&mist.object_id)
    })
    .await;
    let lasted = cast.elapsed();
    assert!(
        lasted >= Duration::from_secs(4) && lasted < Duration::from_secs(6),
        "4 seconds from its start: {lasted:?}"
    );
    until(c, &mut t, |t| {
        t.field.mists_removed.contains(&far.object_id)
    })
    .await;
}

#[tokio::test]
async fn flame_gear_leaves_its_cloud_at_the_caster() {
    let (mut session, mut t, snail, (x, y)) = knight_by_a_snail(BLAZE_WIZARD, FLAME_GEAR).await;
    let c = &mut session.connection;
    send(c, magic_attack(FLAME_GEAR, snail, 1)).await;
    until(c, &mut t, |t| !mine(t, FLAME_GEAR).is_empty()).await;
    let mist = mine(&t, FLAME_GEAR)[0];
    assert_eq!((mist.kind, mist.level), (1, 1));
    // Facing left, the box as the data has it.
    let (x, y) = (i32::from(x), i32::from(y));
    assert_eq!(mist.area, (x - 200, y - 250, x + 200, y + 30));
    until(c, &mut t, |t| {
        t.field.mists_removed.contains(&mist.object_id)
    })
    .await;
}
