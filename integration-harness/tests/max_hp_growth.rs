//! Improving MaxHP Increase: a warrior who has it gains its `x` more max HP
//! on a level up and its `y` more for an AP.

use integration_harness::database::{grant_skill, set_job};
use integration_harness::hunting::{attack, spend_ap};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, stat, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const FOREST: i32 = 50_000;
const SNAIL: i32 = 100100;
const IMPROVING_MAX_HP: i32 = 1000001;

#[tokio::test]
async fn improving_max_hp_adds_to_level_ups_and_ap() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 1, 50).await.expect("level 1 warrior");
    grant_skill(id, IMPROVING_MAX_HP, 10)
        .await
        .expect("Improving MaxHP Increase");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let max_hp = i64::from(session.max_hp);

    // Five snails (15 EXP) reach level 2: 26 max HP, and 40 from the skill.
    let mut h = Trace::default();
    assert_eq!(warp(c, FOREST, "", &mut h).await, FOREST);
    until(c, &mut h, |t| t.field.alive(SNAIL).len() >= 5).await;
    for &snail in &h.field.alive(SNAIL)[..5] {
        send(c, attack(snail, 8)).await;
    }
    until(c, &mut h, |t| t.stat(stat::LEVEL).is_some()).await;
    assert_eq!(h.stat(stat::MAX_HP), Some(max_hp + 26 + 40));

    // An AP adds 22, and 30 from the skill.
    let t = step(c, "max hp", spend_ap(stat::MAX_HP)).await;
    assert_eq!(t.stat(stat::MAX_HP), Some(max_hp + 66 + 52));
}
