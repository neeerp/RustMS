//! Drain: an Assassin's star gives back a share of the damage it does as
//! HP, but no more than the mob's max HP.

use integration_harness::database::{
    grant_skill, grant_stack, replace_item, set_hp, set_job, set_weapon_attack,
};
use integration_harness::hunting::ranged_attack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const HUNTING_GROUND: i32 = 40_001;
const SNAIL: i32 = 100100;
const DRAIN: i32 = 4101005;
const GARNIER: i32 = 1472000;
const SUBI: i32 = 2070000;
const EQUIPPED: i16 = -1;
const WEAPON: i16 = -11;
const USE: i16 = 2;
const STARS: i16 = 20;
/// STAT_CHANGED's HP bit.
const HP: i32 = 0x400;

#[tokio::test]
async fn drain_heals_a_share_of_the_damage() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 410, 40, 1000).await.expect("assassin");
    set_hp(id, 100, 1000).await.expect("low HP");
    grant_skill(id, DRAIN, 30).await.expect("Drain");
    replace_item(id, EQUIPPED, WEAPON, GARNIER)
        .await
        .expect("give a claw");
    set_weapon_attack(id, 500).await.expect("hit hard");
    grant_stack(id, USE, STARS, SUBI, 100)
        .await
        .expect("give stars");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut h = Trace::default();
    assert_eq!(warp(c, HUNTING_GROUND, "", &mut h).await, HUNTING_GROUND);
    until(c, &mut h, |t| t.field.alive(SNAIL).len() >= 2).await;
    let snails = h.field.alive(SNAIL);

    // Drain 30 gives back 45% of 4, rounded down.
    let t = step(c, "drain", ranged_attack(DRAIN, STARS, snails[0], 4)).await;
    assert_eq!(t.stat(HP), Some(101));
    // But no more than the snail's 8 HP however hard it hits.
    let t = step(c, "drain", ranged_attack(DRAIN, STARS, snails[1], 100)).await;
    assert_eq!(t.stat(HP), Some(109));
}
