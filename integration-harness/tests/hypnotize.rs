//! Hypnotize: a Corsair's close gun shot with no `damage` of its own hits
//! for the plain blow and dazzles the mob (MobStat bit 28), at level 20
//! always. A shot that misses puts nothing on it. Unlike a plain blow, its
//! hit doesn't give the shooter aggro over the mob. Another mob's attack
//! that reaches a Hypnotized mob hurts it (MOB_DAMAGE_MOB), once in 2 s.

use integration_harness::database::{grant_skill, replace_item, set_job};
use integration_harness::hunting::{attack, skill_attack, Field};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

// Each test has a snail map no other test hunts on, so its snails are
// whole and none is dazzled by another test running beside it. Each kills
// the snail it hurt, for the next run on the same stack.
const HUNTING_GROUND: i32 = 1_000_004;
const QUIET_GROUND: i32 = 40_002;
const SNAIL: i32 = 100100;
const CORSAIR: i16 = 522;
const HYPNOTIZE: i32 = 5221009;
const GUN: i32 = 1492000;
const EQUIPPED: i16 = -1;
const WEAPON: i16 = -11;
const DAZZLE: u128 = 1 << 28;

#[tokio::test]
async fn hypnotize_dazzles_the_mob_it_hits() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, CORSAIR, 120, 1000).await.expect("Corsair");
    grant_skill(id, HYPNOTIZE, 20).await.expect("Hypnotize");
    replace_item(id, EQUIPPED, WEAPON, GUN)
        .await
        .expect("give a gun");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut h = Trace::default();
    assert_eq!(warp(c, HUNTING_GROUND, "", &mut h).await, HUNTING_GROUND);
    until(c, &mut h, |t| t.field.alive(SNAIL).len() >= 2).await;
    let snails = h.field.alive(SNAIL);

    let t = step(c, "miss", skill_attack(HYPNOTIZE, snails[1], 0)).await;
    assert!(!t.stat_updates.is_empty(), "the shot is taken");
    assert!(t.field.stat_sets.is_empty(), "a miss dazzles nothing");

    let t = step(c, "shot", skill_attack(HYPNOTIZE, snails[0], 1)).await;
    assert!(!t.stat_updates.is_empty(), "the shot is taken");
    assert_eq!(t.field.stat_sets, [(snails[0], DAZZLE, 1, HYPNOTIZE)]);
    assert!(t.field.aggro.is_empty(), "the shot draws no aggro");

    send(c, attack(snails[0], 1)).await;
    until(c, &mut h, |t| t.field.aggro.contains(&snails[0])).await;
    send(c, attack(snails[0], 8)).await;
    until(c, &mut h, |t| t.field.killed.contains(&snails[0])).await;
}

/// The pairs of snails no more than 600 x 300 apart, which can hurt each
/// other.
fn near_pairs(field: &Field) -> Vec<(i32, i32)> {
    let snails = field.alive(SNAIL);
    let at = |oid: &i32| field.mob_positions[oid];
    snails
        .iter()
        .flat_map(|a| snails.iter().map(move |b| (*a, *b)))
        .filter(|(a, b)| {
            let ((ax, ay), (bx, by)) = (at(a), at(b));
            a != b && (ax - bx).abs() <= 600 && (ay - by).abs() <= 300
        })
        .collect()
}

/// MOB_DAMAGE_MOB as the v83 client sends it (FUN_00670c63).
fn mob_damage_mob(attacker: i32, character_id: i32, victim: i32, damage: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::MobDamageMob as i16).unwrap();
    p.write_int(attacker).unwrap();
    p.write_int(character_id).unwrap();
    p.write_int(victim).unwrap();
    p.write_byte(0).unwrap();
    p.write_int(damage).unwrap();
    p.write_byte(0).unwrap();
    p.write_short(0).unwrap();
    p.write_short(0).unwrap();
    p
}

#[tokio::test]
async fn a_mob_hurts_a_hypnotized_neighbour() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    let other_config = load_harness_config_or_fail().await;
    login_to_world_session(&other_config)
        .await
        .expect("second login failed");
    drop(relogin(&other_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, CORSAIR, 120, 1000).await.expect("Corsair");
    grant_skill(id, HYPNOTIZE, 20).await.expect("Hypnotize");
    replace_item(id, EQUIPPED, WEAPON, GUN)
        .await
        .expect("give a gun");

    let mut watcher = relogin(&other_config).await;
    let w = &mut watcher.connection;
    let mut wt = Trace::default();
    assert_eq!(warp(w, QUIET_GROUND, "", &mut wt).await, QUIET_GROUND);
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut h = Trace::default();
    assert_eq!(warp(c, QUIET_GROUND, "", &mut h).await, QUIET_GROUND);
    until(c, &mut h, |t| !near_pairs(&t.field).is_empty()).await;

    // A run that failed may have left a snail hurt, which could die of the
    // first blow, so the victim is one the shot leaves at 4 of its 8 HP or
    // more (a bar of 50%). A dazzled snail can't hurt another, so no snail
    // already shot attacks.
    let mut fresh = None;
    let mut dead = h.field.killed.clone();
    let mut shot = Vec::new();
    for (victim, attacker) in near_pairs(&h.field) {
        if [victim, attacker]
            .iter()
            .any(|oid| dead.contains(oid) || shot.contains(oid))
        {
            continue;
        }
        shot.push(victim);
        let mut t = step(c, "shot", skill_attack(HYPNOTIZE, victim, 1)).await;
        assert_eq!(t.field.stat_sets, [(victim, DAZZLE, 1, HYPNOTIZE)]);
        let bar = |t: &Trace| {
            let mut bars = t.field.hp_bars.iter();
            bars.find(|&&(oid, _)| oid == victim).map(|&(_, hp)| hp)
        };
        until(c, &mut t, |t| {
            bar(t).is_some() || t.field.killed.contains(&victim)
        })
        .await;
        dead.extend(&t.field.killed);
        if bar(&t) >= Some(50) {
            fresh = Some((victim, attacker));
            break;
        }
    }
    let (victim, attacker) = fresh.expect("a snail with 4 HP or more beside another");

    // Only the Hypnotized one is hurt, once in 2 s; the others see it.
    send(c, mob_damage_mob(victim, id, attacker, 3)).await;
    send(c, mob_damage_mob(attacker, id, victim, 3)).await;
    send(c, mob_damage_mob(attacker, id, victim, 3)).await;
    until(w, &mut wt, |t| !t.field.mobs_hurt.is_empty()).await;
    let (hurt, damage, hp) = wt.field.mobs_hurt[0];
    assert_eq!((hurt, damage), (victim, 3));
    assert!(hp > 0, "a snail has more than 4 HP");

    // The next one, later, kills it for the shooter.
    tokio::time::sleep(Duration::from_millis(2100)).await;
    send(c, mob_damage_mob(attacker, id, victim, 500)).await;
    until(c, &mut h, |t| t.field.killed.contains(&victim)).await;
    until(c, &mut h, |t| !t.gains.is_empty()).await;
    read_for(w, &mut wt, 300).await;
    assert!(wt.field.killed.contains(&victim));
    assert_eq!(wt.field.mobs_hurt.len(), 1, "one hit in 2 s");
    assert!(h.field.mobs_hurt.is_empty(), "the sender showed its own");
}
