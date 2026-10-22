//! Hypnotize: a Corsair's close gun shot with no `damage` of its own hits
//! for the plain blow and dazzles the mob (MobStat bit 28), at level 20
//! always. A shot that misses puts nothing on it. Unlike a plain blow, its
//! hit doesn't give the shooter aggro over the mob.

use integration_harness::database::{grant_skill, replace_item, set_job};
use integration_harness::hunting::{attack, skill_attack};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const HUNTING_GROUND: i32 = 40_001;
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
}
