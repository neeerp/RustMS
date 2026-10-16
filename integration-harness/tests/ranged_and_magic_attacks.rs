//! Shots and spells on Snail Hunting Ground II (40001): RANGED_ATTACK hurts
//! mobs like a swing and the map sees it as a shot, and MAGIC_ATTACK needs a
//! spell.

use integration_harness::hunting::{magic_attack, ranged_attack};
use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use net::packet::op::SendOpcode;

const HUNTING_GROUND: i32 = 40_001;
const SNAIL: i32 = 100100;

#[tokio::test]
async fn shots_kill_mobs_and_spells_need_a_skill() {
    let hunter_config = load_harness_config_or_fail().await;
    let watcher_config = load_harness_config_or_fail().await;
    let mut hunter = login_to_world_session(&hunter_config).await.unwrap();
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();

    let mut h = Trace::default();
    assert_eq!(
        warp(&mut hunter.connection, HUNTING_GROUND, "", &mut h).await,
        HUNTING_GROUND
    );
    let mut w = Trace::default();
    assert_eq!(
        warp(&mut watcher.connection, HUNTING_GROUND, "", &mut w).await,
        HUNTING_GROUND
    );
    // Earlier runs may have hunted here; wait for a respawn.
    until(&mut hunter.connection, &mut h, |t| {
        t.field.alive(SNAIL).len() >= 2
    })
    .await;
    let snails = h.field.alive(SNAIL);

    // Two shots of 4 kill an 8 HP snail, and the watcher sees them as shots.
    let snail = snails[0];
    send(&mut hunter.connection, ranged_attack(0, snail, 4)).await;
    until(&mut hunter.connection, &mut h, |t| {
        t.field.hp_bars.contains(&(snail, 50))
    })
    .await;
    send(&mut hunter.connection, ranged_attack(0, snail, 4)).await;
    until(&mut hunter.connection, &mut h, |t| {
        t.field.killed.contains(&snail) && !t.gains.is_empty()
    })
    .await;
    until(&mut watcher.connection, &mut w, |t| {
        t.field.killed.contains(&snail) && t.field.attack_ops.len() == 2
    })
    .await;
    assert_eq!(w.field.attacks_by, [hunter.character_id; 2]);
    assert_eq!(w.field.attack_ops, [SendOpcode::RangedAttack as i16; 2]);

    // A spell without a skill, or with one the beginner lacks, does nothing.
    let other = snails[1];
    send(&mut hunter.connection, magic_attack(0, other, 4)).await;
    send(&mut hunter.connection, magic_attack(2001004, other, 4)).await;
    read_for(&mut hunter.connection, &mut h, 400).await;
    read_for(&mut watcher.connection, &mut w, 100).await;
    assert!(!h.field.hp_bars.iter().any(|(oid, _)| *oid == other));
    assert_eq!(w.field.attack_ops.len(), 2);
}
