//! Mob counters: a Lilynouch (8220005) at half its HP or under casts its
//! counter (145 level 1: weapons and spells, x 2000, y 1500, ten
//! seconds), and each attack on it after hurts the attacker 2000, give or
//! take 750. A v83 client shows its own roll and sends nothing
//! (FUN_00953595), so the server takes the HP.
//!
//! The counter waits a minute between casts, so a rerun on the same stack
//! must wait that out.

use integration_harness::database::{set_hp, set_job, set_str, set_weapon_attack};
use integration_harness::hunting::{attack, move_mob};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, stat, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

/// Temple of Time's Lilynouch map.
const MAP: i32 = 270_020_500;
const LILYNOUCH: i32 = 8_220_005;
const HEAL: u8 = 114;
const COUNTER: u8 = 145;
const PCOUNTER: u128 = 1 << 29;
const MCOUNTER: u128 = 1 << 30;
const MAX_HP: i16 = 30_000;

/// The movement casting `skill` at `level` from the mob's `nth` skill
/// animation.
fn cast(object_id: i32, at: (i16, i16), nth: u8, skill: u8, level: u8) -> Packet {
    let data = i32::from(skill) | i32::from(level) << 8;
    move_mob(object_id, at, false, (20 + nth) << 1, data)
}

#[tokio::test]
async fn a_counter_hurts_whoever_attacks_the_mob() {
    let config = load_harness_config_or_fail().await;

    // A warrior who takes a boss to half in a few hits and outlives its
    // counter.
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 120, 100).await.expect("make a warrior");
    set_str(id, 9_999).await.expect("make it strong");
    set_weapon_attack(id, 30_000)
        .await
        .expect("sharpen its sword");
    set_hp(id, MAX_HP, MAX_HP).await.expect("make it hardy");
    let mut player = relogin(&config).await;
    let c = &mut player.connection;

    let mut t = Trace::default();
    assert_eq!(warp(c, MAP, "", &mut t).await, MAP);
    until(c, &mut t, |t| !t.field.alive(LILYNOUCH).is_empty()).await;
    let boss = t.field.alive(LILYNOUCH)[0];
    let at = t.field.mob_positions[&boss];
    until(c, &mut t, |t| t.field.controlled.contains(&boss)).await;

    // Take it to half (of 7.6M) in swings of v83's most, 199,999, unless
    // an earlier run on this stack did; nothing counters yet.
    send(c, attack(boss, 1)).await;
    until(c, &mut t, |t| !t.field.hp_bars.is_empty()).await;
    while t.field.hp_bars.last().is_some_and(|&(_, percent)| percent > 50) {
        let bars = t.field.hp_bars.len();
        send(c, attack(boss, 199_999)).await;
        until(c, &mut t, |t| t.field.hp_bars.len() > bars).await;
    }
    assert!(t
        .field
        .hp_bars
        .last()
        .is_some_and(|&(_, percent)| percent <= 50));
    assert!(t.stat(stat::HP).is_none());

    // It is granted its heal or its counter; a heal waits five minutes
    // after, so the counter comes next.
    send(c, move_mob(boss, at, true, 0, 0)).await;
    until(c, &mut t, |t| !t.field.grants.is_empty()).await;
    if t.field.grants.last() == Some(&(boss, HEAL, 23)) {
        send(c, cast(boss, at, 1, HEAL, 23)).await;
        until(c, &mut t, |t| !t.field.heals.is_empty()).await;
        tokio::time::sleep(Duration::from_millis(600)).await;
        send(c, move_mob(boss, at, true, 0, 0)).await;
        until(c, &mut t, |t| t.field.grants.len() > 1).await;
    }
    assert_eq!(t.field.grants.last(), Some(&(boss, COUNTER, 1)));
    send(c, cast(boss, at, 2, COUNTER, 1)).await;
    until(c, &mut t, |t| !t.field.stat_sets.is_empty()).await;
    assert_eq!(
        t.field.stat_sets,
        [(
            boss,
            PCOUNTER | MCOUNTER,
            2_000,
            i32::from(COUNTER) | 1 << 16
        )]
    );

    // A swing now costs 2000 HP, give or take 750.
    send(c, attack(boss, 1)).await;
    until(c, &mut t, |t| t.stat(stat::HP).is_some()).await;
    let lost = i64::from(MAX_HP) - t.stat(stat::HP).unwrap();
    assert!((1_250..2_750).contains(&lost), "lost {lost}");
}
