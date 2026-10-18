//! Poison on Snail Hunting Ground II (40001): a Fire/Poison Mage's
//! Element Composition poisons a snail it hits, and the poison takes HP
//! each second, the mage seeing the snail's HP bar fall, but leaves it at 1.

use integration_harness::database::{grant_skill, set_job};
use integration_harness::hunting::magic_attack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::{Duration, Instant};

const HUNTING_GROUND: i32 = 40_001;
const SNAIL: i32 = 100100;
const FP_MAGE: i16 = 211;
const ELEMENT_COMPOSITION: i32 = 2111006;
const POISON: u128 = 1 << 9;

#[tokio::test]
async fn element_composition_poisons_a_snail_down_to_one_hp() {
    let config = load_harness_config_or_fail().await;
    let mage_id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(mage_id, FP_MAGE, 70, 2000)
        .await
        .expect("make a mage");
    grant_skill(mage_id, ELEMENT_COMPOSITION, 30)
        .await
        .expect("teach Element Composition");

    let mut mage = relogin(&config).await;
    let c = &mut mage.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, HUNTING_GROUND, "", &mut t).await, HUNTING_GROUND);
    until(c, &mut t, |t| !t.field.alive(SNAIL).is_empty()).await;

    // At 30 it takes hold 70% of the time, for 40 seconds: hit snails for
    // 1 until one is poisoned. A miss (0) never poisons.
    let snail = t.field.alive(SNAIL)[0];
    send(c, magic_attack(ELEMENT_COMPOSITION, snail, 0)).await;
    read_for(c, &mut t, 300).await;
    assert!(t.field.stat_sets.is_empty());
    let mut poisoned = None;
    let mut hit = vec![snail];
    for _ in 0..40 {
        let Some(&snail) = t.field.alive(SNAIL).first() else {
            read_for(c, &mut t, 500).await;
            continue;
        };
        hit.push(snail);
        send(c, magic_attack(ELEMENT_COMPOSITION, snail, 1)).await;
        read_for(c, &mut t, 300).await;
        if let Some(&(oid, ..)) = t.field.stat_sets.first() {
            poisoned = Some(oid);
            break;
        }
    }
    let snail = poisoned.expect("a snail was poisoned");
    let at = Instant::now();
    // Level 30 poison: max HP 8 over 40, rounded up, so 1 each second.
    assert_eq!(t.field.stat_sets, [(snail, POISON, 1, ELEMENT_COMPOSITION)]);
    let bars = t.field.hp_bars.len();

    read_for(c, &mut t, 8_500).await;
    let falling: Vec<u8> = t.field.hp_bars[bars..]
        .iter()
        .filter(|&&(oid, _)| oid == snail)
        .map(|&(_, percent)| percent)
        .collect();
    assert!(falling.len() >= 2, "{falling:?}");
    assert!(falling.windows(2).all(|w| w[0] > w[1]), "{falling:?}");
    assert_eq!(falling.last(), Some(&13));
    assert!(falling.len() <= 7, "one tick a second: {falling:?}");
    assert!(!t.field.killed.contains(&snail));
    assert!(at.elapsed() < Duration::from_secs(40));

    // Leave no hurt snails for the tests that hunt here next: 8 HP at
    // most, 1 a hit.
    hit.dedup();
    for oid in hit {
        for _ in 0..8 {
            if t.field.killed.contains(&oid) {
                break;
            }
            send(c, magic_attack(ELEMENT_COMPOSITION, oid, 1)).await;
            read_for(c, &mut t, 200).await;
        }
    }
    assert!(t.field.killed.contains(&snail));
}
