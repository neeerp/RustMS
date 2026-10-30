//! A Chief Bandit's Pickpocket shakes mesos out of the lines that hurt,
//! anyone's at once, and Steal takes one item from a mob, once.

use integration_harness::database::{
    grant_skill, replace_item, set_dex, set_job, set_luk, set_str, set_weapon_attack,
};
use integration_harness::hunting::{
    cancel_buff, skill_attack, skill_attack_lines, use_skill, MapDrop,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const HUNTING_GROUND: i32 = 40_001;
const SNAIL: i32 = 100100;
const SAVAGE_BLOW: i32 = 4201005;
const STEAL: i32 = 4201004;
const PICKPOCKET: i32 = 4211003;
const DAGGER: i32 = 1332000;
const EQUIPPED: i16 = -1;
const WEAPON: i16 = -11;

#[tokio::test]
async fn pickpocket_shakes_out_mesos_and_steal_takes_one_item() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 421, 120, 1000).await.expect("chief bandit");
    for (skill, level) in [(SAVAGE_BLOW, 30), (STEAL, 30), (PICKPOCKET, 20)] {
        grant_skill(id, skill, level).await.expect("skill");
    }
    replace_item(id, EQUIPPED, WEAPON, DAGGER)
        .await
        .expect("give a dagger");
    set_weapon_attack(id, 500).await.expect("hit hard");
    set_str(id, 999).await.expect("strong");
    // The dagger asks for 25 DEX and LUK; under them it wouldn't count, and
    // its attack with it.
    set_dex(id, 25).await.expect("dagger's DEX");
    set_luk(id, 25).await.expect("dagger's LUK");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut h = Trace::default();
    assert_eq!(warp(c, HUNTING_GROUND, "", &mut h).await, HUNTING_GROUND);
    until(c, &mut h, |t| t.field.alive(SNAIL).len() >= 3).await;
    let snails = h.field.alive(SNAIL);
    // A line of 7000 picks 7000/20000 of level 20's 200: 70, more than a
    // snail drops.
    let picked = |drops: &[MapDrop]| -> Vec<MapDrop> {
        drops
            .iter()
            .filter(|d| d.meso && d.value == 70)
            .copied()
            .collect()
    };

    // Without the buff (a run before may have left it on), Savage Blow
    // shakes nothing out.
    step(c, "cancel", cancel_buff(PICKPOCKET)).await;
    let blow = skill_attack_lines(SAVAGE_BLOW, snails[0], &[7000; 6]);
    let mut drops = step(c, "no pickpocket", blow).await.field.drops;
    drops.extend(step(c, "settle", cancel_buff(0)).await.field.drops);
    assert!(picked(&drops).is_empty(), "{drops:?}");

    // With it, each line picks 60% of the time; 0.4^6 that none does. The
    // mesos are anyone's at once.
    step(c, "pickpocket", use_skill(PICKPOCKET, 20)).await;
    let blow = skill_attack_lines(SAVAGE_BLOW, snails[1], &[7000; 6]);
    let mut drops = step(c, "pickpocket", blow).await.field.drops;
    drops.extend(step(c, "settle", cancel_buff(0)).await.field.drops);
    let pocketed = picked(&drops);
    assert!(!pocketed.is_empty(), "{drops:?}");
    assert!(pocketed.iter().all(|d| d.ownership == 2), "{pocketed:?}");
    step(c, "settle", cancel_buff(0)).await.field.drops;

    // Steal at 30 takes 40% of the time, misses too, but from each mob
    // once: 0.6^40 that 40 misses take nothing.
    let mut stolen = Vec::new();
    for _ in 0..40 {
        let steal = skill_attack(STEAL, snails[2], 0);
        stolen.extend(step(c, "steal", steal).await.field.drops);
    }
    stolen.extend(step(c, "settle", cancel_buff(0)).await.field.drops);
    assert_eq!(stolen.len(), 1, "{stolen:?}");
    assert_eq!(stolen[0].ownership, 0);
}
