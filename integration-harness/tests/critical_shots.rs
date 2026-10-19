//! The damage cap of a shot counts the star it throws and the critical
//! skill behind it: an Assassin with 4 LUK, 12 STR and 5 DEX, a claw of 10
//! attack and Subis (15) hits for at most ceil((3.6 * 4 + 12 + 5) / 100 *
//! 25) = 8, so 16 for the two that v83 allows, and Critical Throw's 200%
//! doubles that.

use integration_harness::database::{
    grant_skill, grant_stack, replace_item, set_job, set_weapon_attack,
};
use integration_harness::hunting::ranged_attack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use tokio::time::Duration;

const HUNTING_GROUND: i32 = 40_001;
const SNAIL: i32 = 100100;
const CRITICAL_THROW: i32 = 4100001;
const GARNIER: i32 = 1472000;
const SUBI: i32 = 2070000;
const EQUIPPED: i16 = -1;
const WEAPON: i16 = -11;
const USE: i16 = 2;
const STARS: i16 = 20;

/// Shoot a snail for `too_much`, which is refused, then for `most`, which
/// kills it.
async fn shoot_up_to(session: &mut WorldSession, most: i32, too_much: i32) {
    let c = &mut session.connection;
    let mut h = Trace::default();
    assert_eq!(warp(c, HUNTING_GROUND, "", &mut h).await, HUNTING_GROUND);
    until(c, &mut h, |t| !t.field.alive(SNAIL).is_empty()).await;
    let snail = h.field.alive(SNAIL)[0];

    send(c, ranged_attack(0, STARS, snail, too_much)).await;
    read_for(c, &mut h, 400).await;
    assert!(!h.field.killed.contains(&snail), "{too_much} is too much");
    send(c, ranged_attack(0, STARS, snail, most)).await;
    until(c, &mut h, |t| t.field.killed.contains(&snail)).await;
}

#[tokio::test]
async fn critical_throw_raises_the_cap_of_a_shot() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 410, 30, 1000).await.expect("assassin");
    replace_item(id, EQUIPPED, WEAPON, GARNIER)
        .await
        .expect("give a claw");
    set_weapon_attack(id, 10).await.expect("claw attack");
    grant_stack(id, USE, STARS, SUBI, 100)
        .await
        .expect("give stars");

    let mut session = relogin(&config).await;
    shoot_up_to(&mut session, 16, 17).await;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    grant_skill(id, CRITICAL_THROW, 30)
        .await
        .expect("Critical Throw");
    let mut session = relogin(&config).await;
    shoot_up_to(&mut session, 32, 33).await;
}
