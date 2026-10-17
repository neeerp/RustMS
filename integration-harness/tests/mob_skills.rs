//! Mob skills: the server lets a controlled mob cast one of its skills in
//! the answer to a movement that wants to attack (MOVE_MONSTER_RESPONSE),
//! the controller casts it in a later movement, and the server applies it:
//! a Power Up on Horny Mushrooms, a Heal from a Lycanthrope, and a summon
//! from a Jr. Cerebes.

use integration_harness::database::{set_job, set_str};
use integration_harness::hunting::attack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

/// A Sleepywood dungeon map with Horny Mushrooms.
const MUSHROOM_MAP: i32 = 106_000_120;
const HORNY_MUSHROOM: i32 = 4_230_103;
const POWER_UP: u8 = 100;
const POWER_UP_STAT: u128 = 1 << 12;
/// An El Nath forest map with Lycanthropes.
const WOLF_MAP: i32 = 211_041_100;
const LYCANTHROPE: i32 = 6_090_000;
const HEAL: u8 = 114;
/// A Ludibrium clocktower map with Jr. Cerebes.
const CEREBES_MAP: i32 = 221_040_201;
const JR_CEREBES: i32 = 4_240_000;
const SUMMON: u8 = 200;
const SUMMONED: i32 = 4_230_116;

/// MOVE_LIFE for `object_id` at (x, y): wanting to attack, or playing
/// `action` (its first skill animation is 21, doubled with the facing)
/// with `skill_data` (skill | level << 8 | delay << 16).
fn move_mob(
    object_id: i32,
    (x, y): (i16, i16),
    want_attack: bool,
    action: u8,
    skill_data: i32,
) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::MoveLife as i16).unwrap();
    p.write_int(object_id).unwrap();
    p.write_short(1).unwrap();
    p.write_byte(u8::from(want_attack)).unwrap();
    p.write_byte(action).unwrap();
    p.write_int(skill_data).unwrap();
    p.write_bytes(&[0; 13]).unwrap();
    p.write_short(x).unwrap();
    p.write_short(y).unwrap();
    p.write_byte(1).unwrap();
    p.write_byte(0).unwrap();
    p.write_short(x).unwrap();
    p.write_short(y).unwrap();
    p.write_bytes(&[0; 6]).unwrap();
    p.write_byte(4).unwrap();
    p.write_short(100).unwrap();
    p
}

/// The movement casting the `skill` at `level` from the mob's `nth`
/// skill animation (1-based), `delay` ms into it.
fn cast(object_id: i32, at: (i16, i16), nth: u8, skill: u8, level: u8, delay: u16) -> Packet {
    let data = i32::from(skill) | i32::from(level) << 8 | i32::from(delay) << 16;
    move_mob(object_id, at, false, (20 + nth) << 1, data)
}

#[tokio::test]
async fn a_granted_power_up_is_cast_and_seen() {
    let caster_config = load_harness_config_or_fail().await;
    let watcher_config = load_harness_config_or_fail().await;
    let mut caster = login_to_world_session(&caster_config).await.unwrap();
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let (c, w) = (&mut caster.connection, &mut watcher.connection);

    let mut ct = Trace::default();
    assert_eq!(warp(c, MUSHROOM_MAP, "", &mut ct).await, MUSHROOM_MAP);
    until(c, &mut ct, |t| {
        t.field
            .controlled
            .iter()
            .any(|oid| t.field.mobs.contains(&(*oid, HORNY_MUSHROOM)))
    })
    .await;
    let mob = *ct
        .field
        .controlled
        .iter()
        .find(|oid| ct.field.mobs.contains(&(**oid, HORNY_MUSHROOM)))
        .unwrap();
    let at = ct.field.mob_positions[&mob];
    let mut wt = Trace::default();
    assert_eq!(warp(w, MUSHROOM_MAP, "", &mut wt).await, MUSHROOM_MAP);
    until(w, &mut wt, |t| {
        t.field.mobs.iter().any(|(oid, _)| *oid == mob)
    })
    .await;

    // Casting before it is granted does nothing.
    send(c, cast(mob, at, 1, POWER_UP, 1, 0)).await;
    read_for(c, &mut ct, 400).await;
    assert!(ct.field.stat_sets.is_empty());

    // Wanting to attack grants its one skill, Power Up 1.
    send(c, move_mob(mob, at, true, 0, 0)).await;
    until(c, &mut ct, |t| !t.field.grants.is_empty()).await;
    assert_eq!(ct.field.grants, [(mob, POWER_UP, 1)]);

    // A cast of another level is refused; the granted one powers it up to
    // 115% for everyone.
    send(c, cast(mob, at, 1, POWER_UP, 2, 0)).await;
    read_for(c, &mut ct, 400).await;
    assert!(ct.field.stat_sets.is_empty());
    send(c, cast(mob, at, 1, POWER_UP, 1, 0)).await;
    until(w, &mut wt, |t| !t.field.stat_sets.is_empty()).await;
    assert_eq!(
        wt.field.stat_sets,
        [(mob, POWER_UP_STAT, 115, i32::from(POWER_UP) | 1 << 16)]
    );

    // It is on cooldown for 40 seconds after.
    send(c, move_mob(mob, at, true, 0, 0)).await;
    read_for(c, &mut ct, 400).await;
    assert_eq!(ct.field.grants.len(), 1);
}

#[tokio::test]
async fn wounded_mobs_heal_and_summon() {
    let config = load_harness_config_or_fail().await;

    // A warrior with the STR to take a third off a mob in one hit.
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 70, 100).await.expect("make a warrior");
    set_str(id, 9_999).await.expect("make it strong");
    let mut player = relogin(&config).await;
    let c = &mut player.connection;

    // A Lycanthrope (11,000 HP) heals at 70% HP or under: up to 5,000 HP,
    // a second after its second skill animation starts.
    let mut t = Trace::default();
    assert_eq!(warp(c, WOLF_MAP, "", &mut t).await, WOLF_MAP);
    until(c, &mut t, |t| !t.field.alive(LYCANTHROPE).is_empty()).await;
    let wolf = t.field.alive(LYCANTHROPE)[0];
    let at = t.field.mob_positions[&wolf];
    send(c, attack(wolf, 4_000)).await;
    until(c, &mut t, |t| t.field.controlled.contains(&wolf)).await;
    send(c, move_mob(wolf, at, true, 0, 0)).await;
    until(c, &mut t, |t| !t.field.grants.is_empty()).await;
    assert_eq!(t.field.grants, [(wolf, HEAL, 12)]);
    send(c, cast(wolf, at, 2, HEAL, 12, 1_000)).await;
    until(c, &mut t, |t| !t.field.heals.is_empty()).await;
    assert_eq!(t.field.heals, [(wolf, 4_000)]);

    // A Jr. Cerebes (9,000 HP) at 75% or under calls up Bloctopuses that
    // appear with its summon effect, 6, under the same controller.
    let mut t = Trace::default();
    assert_eq!(warp(c, CEREBES_MAP, "", &mut t).await, CEREBES_MAP);
    until(c, &mut t, |t| !t.field.alive(JR_CEREBES).is_empty()).await;
    let cerebes = t.field.alive(JR_CEREBES)[0];
    let at = t.field.mob_positions[&cerebes];
    send(c, attack(cerebes, 3_000)).await;
    until(c, &mut t, |t| t.field.controlled.contains(&cerebes)).await;
    send(c, move_mob(cerebes, at, true, 0, 0)).await;
    until(c, &mut t, |t| !t.field.grants.is_empty()).await;
    assert_eq!(t.field.grants, [(cerebes, SUMMON, 27)]);
    let before = t.field.alive(SUMMONED);
    send(c, cast(cerebes, at, 1, SUMMON, 27, 0)).await;
    until(c, &mut t, |t| t.field.alive(SUMMONED).len() > before.len()).await;
    let summoned: Vec<i32> = t
        .field
        .alive(SUMMONED)
        .into_iter()
        .filter(|oid| !before.contains(oid))
        .collect();
    assert_eq!(summoned.len(), 1);
    assert_eq!(t.field.appearances[&summoned[0]], 6);
    assert_eq!(t.field.mob_positions[&summoned[0]], at);
    until(c, &mut t, |t| t.field.controlled.contains(&summoned[0])).await;
}
