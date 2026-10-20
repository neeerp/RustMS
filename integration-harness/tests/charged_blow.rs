//! Weapon charges: Charged Blow needs one and spends it, unless a Paladin's
//! Advanced Charge keeps it (its `x` is 100 at level 10).

use integration_harness::database::{grant_skill, replace_item, set_hp, set_job};
use integration_harness::hunting::{skill_attack, use_skill};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use tokio::time::Duration;

const HUNTING_GROUND: i32 = 40_001;
const SNAIL: i32 = 100100;
const CHARGED_BLOW: i32 = 1211002;
const FIRE_CHARGE: i32 = 1211003;
const ADVANCED_CHARGE: i32 = 1220010;
const SWORD: i32 = 1302000;
const EQUIPPED: i16 = -1;
const WEAPON: i16 = -11;
/// WeaponCharge's bit in CANCEL_BUFF's second mask.
const CHARGE_MASK: i64 = 1 << 54;

/// A knight of `job` with a sword, a charge and Charged Blow, and Advanced
/// Charge at `advanced`, at the hunting ground and a snail's object id.
async fn knight(job: i16, advanced: i32) -> (WorldSession, i32) {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, job, 120, 1000).await.expect("knight");
    set_hp(id, 1000, 1000).await.expect("HP for the blows");
    grant_skill(id, CHARGED_BLOW, 30)
        .await
        .expect("Charged Blow");
    grant_skill(id, FIRE_CHARGE, 30).await.expect("Fire Charge");
    if advanced > 0 {
        (grant_skill(id, ADVANCED_CHARGE, advanced).await).expect("Advanced Charge");
    }
    replace_item(id, EQUIPPED, WEAPON, SWORD)
        .await
        .expect("give a sword");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut h = Trace::default();
    assert_eq!(warp(c, HUNTING_GROUND, "", &mut h).await, HUNTING_GROUND);
    until(c, &mut h, |t| !t.field.alive(SNAIL).is_empty()).await;
    let snail = h.field.alive(SNAIL)[0];
    (session, snail)
}

#[tokio::test]
async fn charged_blow_spends_the_charge_unless_advanced_charge_keeps_it() {
    // A White Knight's blow needs a charge and spends it.
    let (mut session, snail) = knight(121, 0).await;
    let c = &mut session.connection;
    let t = step(c, "blow", skill_attack(CHARGED_BLOW, snail, 1)).await;
    assert!(t.stat_updates.is_empty(), "refused without a charge");
    step(c, "charge", use_skill(FIRE_CHARGE, 30)).await;
    let t = step(c, "blow", skill_attack(CHARGED_BLOW, snail, 1)).await;
    assert!(!t.stat_updates.is_empty(), "the blow lands");
    assert_eq!(t.cancelled_buffs, [CHARGE_MASK]);
    let t = step(c, "blow", skill_attack(CHARGED_BLOW, snail, 1)).await;
    assert!(t.stat_updates.is_empty(), "and the charge is gone");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // A Paladin's Advanced Charge 10 keeps it.
    let (mut session, snail) = knight(122, 10).await;
    let c = &mut session.connection;
    step(c, "charge", use_skill(FIRE_CHARGE, 30)).await;
    for _ in 0..2 {
        let t = step(c, "blow", skill_attack(CHARGED_BLOW, snail, 1)).await;
        assert!(!t.stat_updates.is_empty(), "the blow lands");
        assert!(t.cancelled_buffs.is_empty(), "and the charge stays");
    }
}
