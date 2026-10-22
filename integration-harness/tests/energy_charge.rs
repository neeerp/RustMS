//! Energy Charge (5110001): a Marauder's swings fill the gauge by the
//! skill's `x` for each mob they reach, 192 at level 40, up to 10000, and
//! Energy Blast (5111002) is refused until it is full.

use integration_harness::database::{grant_skill, replace_item, set_job};
use integration_harness::hunting::{attack, skill_attack};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{stat, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

/// A snail map no other test file hunts on at the same time.
const FIELD: i32 = 1_010_004;
const SNAIL: i32 = 100100;
const MARAUDER: i16 = 511;
const ENERGY_CHARGE: i32 = 5110001;
const ENERGY_BLAST: i32 = 5111002;
const PER_SWING: i16 = 192;
const FULL: i16 = 10000;
const KNUCKLE: i32 = 1482014;
const EQUIPPED: i16 = -1;
const WEAPON: i16 = -11;
const MARK: &str = "energy charge";

/// The gauge values GIVE_BUFF showed.
fn energy(t: &Trace) -> Vec<i16> {
    (t.buffs.iter())
        .filter(|&&(skill, _, _)| skill == ENERGY_CHARGE)
        .map(|&(_, value, _)| value)
        .collect()
}

#[tokio::test]
async fn swings_fill_the_gauge_and_energy_blast_waits_for_it() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, MARAUDER, 70, 1000).await.expect("marauder");
    grant_skill(id, ENERGY_CHARGE, 40)
        .await
        .expect("Energy Charge");
    grant_skill(id, ENERGY_BLAST, 30)
        .await
        .expect("Energy Blast");
    replace_item(id, EQUIPPED, WEAPON, KNUCKLE)
        .await
        .expect("give a knuckle");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut h = Trace::default();
    assert_eq!(warp(c, FIELD, "", &mut h).await, FIELD);
    until(c, &mut h, |t| !t.field.alive(SNAIL).is_empty()).await;
    // The swings do no damage, so the snail outlives reruns on one stack.
    let snail = h.field.alive(SNAIL)[0];

    // Energy Blast is refused outright, without even an MP reply.
    let blast = || skill_attack(ENERGY_BLAST, snail, 0);
    let t = step(c, MARK, blast()).await;
    assert!(t.stat_values(stat::MP).is_empty(), "an empty gauge blasted");

    // Each swing adds 192, and Energy Blast still waits at 9984.
    for swings in 1..=52 {
        let t = step(c, MARK, attack(snail, 0)).await;
        assert_eq!(energy(&t), [PER_SWING * swings]);
    }
    let t = step(c, MARK, blast()).await;
    assert!(t.stat_values(stat::MP).is_empty(), "9984 is not full");

    // The 53rd stops at 10000, and a full gauge gains nothing more.
    let t = step(c, MARK, attack(snail, 0)).await;
    assert_eq!(energy(&t), [FULL]);
    let t = step(c, MARK, attack(snail, 0)).await;
    assert!(energy(&t).is_empty(), "a full gauge changed");

    // Full, Energy Blast goes off (it costs no MP) and leaves the gauge.
    let t = step(c, MARK, blast()).await;
    assert!(
        !t.stat_values(stat::MP).is_empty(),
        "a full gauge was refused"
    );
    assert!(energy(&t).is_empty(), "Energy Blast changed the gauge");
}
