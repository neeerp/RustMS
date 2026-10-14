//! Spending AP and SP from the stat and skill windows: refusals still
//! answer (the client locks its UI until they do), and what is spent
//! survives logging out.

use integration_harness::hunting::{attack, spend_ap, spend_sp};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, stat, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use std::collections::BTreeMap;

const FOREST: i32 = 50_000;
const SNAIL: i32 = 100100;
const THREE_SNAILS: i32 = 1000;
const RECOVERY: i32 = 1001;
const POWER_STRIKE: i32 = 1001004;

#[tokio::test]
async fn ap_and_sp_raise_stats_and_skills_and_persist() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    let only_unlock = [BTreeMap::new()];

    // A new beginner has nothing to spend.
    let t = step(c, "no ap", spend_ap(stat::STR)).await;
    assert_eq!(t.stat_updates, only_unlock);
    let t = step(c, "no sp", spend_sp(THREE_SNAILS)).await;
    assert_eq!(t.stat_updates, only_unlock);
    assert!(t.skills.is_empty());

    // Five snails (15 EXP) reach level 2: 5 AP and one beginner SP.
    let mut h = Trace::default();
    assert_eq!(warp(c, FOREST, "", &mut h).await, FOREST);
    until(c, &mut h, |t| t.field.alive(SNAIL).len() >= 5).await;
    for &snail in &h.field.alive(SNAIL)[..5] {
        send(c, attack(snail, 8)).await;
    }
    until(c, &mut h, |t| t.stat(stat::LEVEL).is_some()).await;
    assert_eq!(h.stat(stat::AP), Some(5));
    let max_hp = h.stat(stat::MAX_HP).unwrap();

    let t = step(c, "str", spend_ap(stat::STR)).await;
    assert_eq!(t.stat(stat::STR), Some(i64::from(session.str) + 1));
    assert_eq!(t.stat(stat::AP), Some(4));
    // A beginner's AP adds 10 max HP.
    let t = step(c, "max hp", spend_ap(stat::MAX_HP)).await;
    assert_eq!(t.stat(stat::MAX_HP), Some(max_hp + 10));
    assert_eq!(t.stat(stat::AP), Some(3));
    // Current HP is not something AP buys.
    let t = step(c, "hp", spend_ap(stat::HP)).await;
    assert_eq!(t.stat_updates, only_unlock);

    let t = step(c, "three snails", spend_sp(THREE_SNAILS)).await;
    assert_eq!(t.skills, [(THREE_SNAILS, 1)]);
    // Level 2 has one beginner SP, and beginners can't learn warrior skills.
    for (marker, skill_id) in [("recovery", RECOVERY), ("power strike", POWER_STRIKE)] {
        let t = step(c, marker, spend_sp(skill_id)).await;
        assert_eq!(t.stat_updates, only_unlock, "{marker}");
        assert!(t.skills.is_empty(), "{marker}");
    }

    let str = session.str;
    drop(session);
    let session = relogin(&config).await;
    assert_eq!(session.level, 2);
    assert_eq!(
        (session.str, i64::from(session.max_hp)),
        (str + 1, max_hp + 10)
    );
    assert_eq!((session.ap, session.sp), (3, 0));
    assert_eq!(session.skills, BTreeMap::from([(THREE_SNAILS, 1)]));
}
