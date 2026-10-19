//! Combo Attack: a Crusader's swings gather orbs, up to five at level 30,
//! and Panic spends them, but not without one.

use integration_harness::database::{grant_skill, replace_item, set_job};
use integration_harness::hunting::{attack, skill_attack, use_skill};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const HUNTING_GROUND: i32 = 40_001;
const SNAIL: i32 = 100100;
const COMBO: i32 = 1111002;
const PANIC: i32 = 1111003;
const SWORD: i32 = 1302000;
const EQUIPPED: i16 = -1;
const WEAPON: i16 = -11;

#[tokio::test]
async fn swings_gather_orbs_and_panic_spends_them() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 111, 70, 1000).await.expect("crusader");
    grant_skill(id, COMBO, 30).await.expect("Combo Attack");
    grant_skill(id, PANIC, 30).await.expect("Panic");
    replace_item(id, EQUIPPED, WEAPON, SWORD)
        .await
        .expect("give a sword");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut h = Trace::default();
    assert_eq!(warp(c, HUNTING_GROUND, "", &mut h).await, HUNTING_GROUND);
    until(c, &mut h, |t| !t.field.alive(SNAIL).is_empty()).await;
    let snail = h.field.alive(SNAIL)[0];
    let combo = |t: &Trace| -> Vec<i16> {
        (t.buffs.iter())
            .filter(|&&(skill, _, _)| skill == COMBO)
            .map(|&(_, value, _)| value)
            .collect()
    };

    // Panic needs an orb.
    let t = step(c, "combo", use_skill(COMBO, 30)).await;
    assert_eq!(combo(&t), [1]);
    let t = step(c, "panic", skill_attack(PANIC, snail, 1)).await;
    assert!(combo(&t).is_empty());

    // Each swing that reaches a mob gains one, up to five.
    for value in 2..=6 {
        let t = step(c, "swing", attack(snail, 1)).await;
        assert_eq!(combo(&t), [value]);
        let (_, _, left) = t.buffs[0];
        assert!(left > 190_000 && left <= 200_000, "the buff keeps its time");
    }
    let t = step(c, "swing", attack(snail, 1)).await;
    assert!(combo(&t).is_empty());

    // Panic spends them all.
    let t = step(c, "panic", skill_attack(PANIC, snail, 1)).await;
    assert_eq!(combo(&t), [1]);
}
