//! Combo Attack: a Crusader's swings gather orbs, up to five at level 30,
//! and Panic spends them, but not without one. A Dawn Warrior's Soul and
//! Combo Attack keep each other.

use integration_harness::database::{grant_skill, replace_item, set_job};
use integration_harness::hunting::{attack, cancel_buff, skill_attack, summon, use_skill};
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
const DAWN_WARRIOR: i16 = 1111;
const SOUL: i32 = 11001004;
const DAWN_COMBO: i32 = 11111001;

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

#[tokio::test]
async fn a_dawn_warriors_soul_leaves_their_orbs_be() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, DAWN_WARRIOR, 70, 1000)
        .await
        .expect("dawn warrior");
    grant_skill(id, SOUL, 1).await.expect("Soul");
    grant_skill(id, DAWN_COMBO, 20).await.expect("Combo Attack");
    replace_item(id, EQUIPPED, WEAPON, SWORD)
        .await
        .expect("give a sword");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut h = Trace::default();
    assert_eq!(warp(c, HUNTING_GROUND, "", &mut h).await, HUNTING_GROUND);
    until(c, &mut h, |t| !t.field.alive(SNAIL).is_empty()).await;
    let snail = h.field.alive(SNAIL)[0];
    let (x, y) = h.field.mob_positions[&snail];
    let given = |t: &Trace, skill_id: i32| -> Vec<i16> {
        (t.buffs.iter())
            .filter(|&&(skill, _, _)| skill == skill_id)
            .map(|&(_, value, _)| value)
            .collect()
    };

    // The Soul comes with its icon, and Combo Attack's orbs take the slot.
    let t = step(c, "dawn soul", summon(SOUL, 1, x, y, true)).await;
    assert_eq!(given(&t, SOUL), [1], "{t:?}");
    let t = step(c, "dawn combo", use_skill(DAWN_COMBO, 20)).await;
    assert_eq!(given(&t, DAWN_COMBO), [1], "{t:?}");
    let t = step(c, "dawn swing", attack(snail, 1)).await;
    assert_eq!(given(&t, DAWN_COMBO), [2], "{t:?}");

    // Sending the Soul away cancels nothing the client holds: the orbs stay.
    let t = step(c, "dawn dismiss", cancel_buff(SOUL)).await;
    assert!(t.cancelled_buffs.is_empty(), "{t:?}");
    let t = step(c, "dawn swing", attack(snail, 1)).await;
    assert_eq!(given(&t, DAWN_COMBO), [3], "{t:?}");

    // A Soul called with the orbs up gives no icon, and takes none.
    let t = step(c, "dawn soul", summon(SOUL, 1, x, y, true)).await;
    assert!(given(&t, SOUL).is_empty(), "{t:?}");
    let t = step(c, "dawn swing", attack(snail, 1)).await;
    assert_eq!(given(&t, DAWN_COMBO), [4], "{t:?}");
}
