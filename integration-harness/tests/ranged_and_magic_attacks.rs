//! Shots and spells on Snail Hunting Ground II (40001): RANGED_ATTACK hurts
//! mobs like a swing, uses up the stars it throws, and the map sees it as a
//! shot; MAGIC_ATTACK needs a spell.

use integration_harness::database::{grant_stack, replace_item, set_job};
use integration_harness::hunting::{magic_attack, ranged_attack};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::SendOpcode;
use tokio::time::Duration;

const HUNTING_GROUND: i32 = 40_001;
const SNAIL: i32 = 100100;
const GARNIER: i32 = 1472000;
const SUBI: i32 = 2070000;
const EQUIPPED: i16 = -1;
const WEAPON: i16 = -11;
const USE: i16 = 2;
const STARS: i16 = 20;

#[tokio::test]
async fn shots_use_up_stars_and_spells_need_a_skill() {
    let hunter_config = load_harness_config_or_fail().await;
    let watcher_config = load_harness_config_or_fail().await;

    // Make the hunter, then give it a claw and two stars while logged out,
    // and the level 10 stars need.
    let hunter_id = login_to_world_session(&hunter_config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&hunter_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    replace_item(hunter_id, EQUIPPED, WEAPON, GARNIER)
        .await
        .expect("give a claw");
    grant_stack(hunter_id, USE, STARS, SUBI, 2)
        .await
        .expect("give stars");
    set_job(hunter_id, 0, 10, 50).await.expect("level 10");

    let mut hunter = relogin(&hunter_config).await;
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

    // Two shots of 4 kill an 8 HP snail, each throwing a star, and the
    // watcher sees them as shots.
    let snail = snails[0];
    send(&mut hunter.connection, ranged_attack(0, STARS, snail, 4)).await;
    until(&mut hunter.connection, &mut h, |t| {
        t.field.hp_bars.contains(&(snail, 50))
    })
    .await;
    assert_eq!(h.quantities, [(USE as u8, STARS, 1)]);
    send(&mut hunter.connection, ranged_attack(0, STARS, snail, 4)).await;
    until(&mut hunter.connection, &mut h, |t| {
        t.field.killed.contains(&snail) && !t.gains.is_empty()
    })
    .await;
    until(&mut hunter.connection, &mut h, |t| !t.removed.is_empty()).await;
    assert_eq!(h.removed, [(USE as u8, STARS)]);
    until(&mut watcher.connection, &mut w, |t| {
        t.field.killed.contains(&snail) && t.field.attack_ops.len() == 2
    })
    .await;
    assert_eq!(w.field.attacks_by, [hunter.character_id; 2]);
    assert_eq!(w.field.attack_ops, [SendOpcode::RangedAttack as i16; 2]);

    // Out of stars, a shot does nothing; so does a spell without a skill,
    // or with one the beginner lacks.
    let other = snails[1];
    send(&mut hunter.connection, ranged_attack(0, STARS, other, 4)).await;
    send(&mut hunter.connection, magic_attack(0, other, 4)).await;
    send(&mut hunter.connection, magic_attack(2001004, other, 4)).await;
    read_for(&mut hunter.connection, &mut h, 400).await;
    read_for(&mut watcher.connection, &mut w, 100).await;
    assert!(!h.field.hp_bars.iter().any(|(oid, _)| *oid == other));
    assert_eq!(w.field.attack_ops.len(), 2);
}
