//! Enrage (1121010): a Hero spends a full Combo, ten or more, for 240 s of
//! Pad, and the Combo goes back to one with a 360 s cooldown. Below ten,
//! or without Combo at all, it is refused and spends nothing.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_skill, replace_item, set_job};
use integration_harness::hunting::{attack, use_skill};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{stat, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

/// A snail map no other test hunts on.
const FIELD: i32 = 1_010_004;
const SNAIL: i32 = 100100;
const HERO: i16 = 112;
const COMBO: i32 = 1111002;
const ADVANCED_COMBO: i32 = 1120003;
const ENRAGE: i32 = 1121010;
const ENRAGE_MP: i64 = 40;
const SWORD: i32 = 1302000;
const EQUIPPED: i16 = -1;
const WEAPON: i16 = -11;
const MARK: &str = "enrage";

/// The Combo values GIVE_BUFF showed.
fn combo(t: &Trace) -> Vec<i16> {
    (t.buffs.iter())
        .filter(|&&(skill, _, _)| skill == COMBO)
        .map(|&(_, value, _)| value)
        .collect()
}

/// Casts Enrage and checks it was refused: no MP spent, no buff, no
/// cooldown.
async fn refused(c: &mut MapleTestConnection) {
    let t = step(c, MARK, use_skill(ENRAGE, 30)).await;
    assert!(
        t.stat_values(stat::MP).is_empty(),
        "a refused Enrage spent MP"
    );
    assert!(
        t.buffs.is_empty() && t.cooldowns.is_empty(),
        "a refused Enrage took"
    );
}

#[tokio::test]
async fn enrage_spends_a_full_combo_once() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, HERO, 120, 1000).await.expect("hero");
    grant_skill(id, COMBO, 30).await.expect("Combo Attack");
    grant_skill(id, ADVANCED_COMBO, 30)
        .await
        .expect("Advanced Combo");
    grant_skill(id, ENRAGE, 30).await.expect("Enrage");
    replace_item(id, EQUIPPED, WEAPON, SWORD)
        .await
        .expect("give a sword");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut h = Trace::default();
    assert_eq!(warp(c, FIELD, "", &mut h).await, FIELD);
    until(c, &mut h, |t| t.field.alive(SNAIL).len() >= 3).await;
    // The swings do no damage, so the snails outlive reruns on one stack.
    let snails = h.field.alive(SNAIL);

    // Without Combo there is nothing to spend.
    refused(c).await;

    let t = step(c, MARK, use_skill(COMBO, 30)).await;
    assert_eq!(combo(&t), [1]);
    let mut mp = *t.stat_values(stat::MP).last().expect("Combo's MP");

    // Each swing gains one orb, sometimes two (Advanced Combo's `prop`);
    // below ten, Enrage is refused.
    let mut value = 1;
    for swing in 0.. {
        assert!(swing < 12, "the Combo never reached ten");
        let t = step(c, MARK, attack(snails[swing % snails.len()], 0)).await;
        let gained = combo(&t);
        assert!(!gained.is_empty(), "a swing gained no orb");
        value = *gained.last().unwrap();
        if value >= 10 {
            break;
        }
        refused(c).await;
    }
    assert!(value <= 11, "Advanced Combo at 30 holds ten orbs");

    // Enrage spends the Combo, back to one, and its MP, and cools down.
    let t = step(c, MARK, use_skill(ENRAGE, 30)).await;
    mp -= ENRAGE_MP;
    assert_eq!(t.stat_values(stat::MP), [mp]);
    assert_eq!(combo(&t), [1]);
    assert!(
        (t.buffs.iter()).any(|&(skill, _, time)| skill == ENRAGE && time == 240_000),
        "no Enrage buff: {:?}",
        t.buffs
    );
    assert_eq!(t.cooldowns, [(ENRAGE, 360)]);

    // Not again: the Combo is one, and it cools down.
    refused(c).await;
}
