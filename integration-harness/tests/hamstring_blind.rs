//! Hamstring and Blind are buffs the client only shows while the Bowmaster's
//! or Marksman's hits slow or blind the mobs they land on: a cast gives the
//! buff at its `x` for its `time`.

use integration_harness::database::{grant_skill, set_job};
use integration_harness::hunting::use_skill;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::step;
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const HAMSTRING: i32 = 3121007;
const BLIND: i32 = 3221006;

async fn cast(job: i16, skill: i32) -> Vec<(i32, i16, i32)> {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, job, 120, 1000).await.expect("make an archer");
    grant_skill(id, skill, 30).await.expect("teach the skill");

    let mut session = relogin(&config).await;
    let t = step(&mut session.connection, "cast", use_skill(skill, 30)).await;
    t.buffs.clone()
}

#[tokio::test]
async fn hamstring_and_blind_are_given_as_buffs() {
    assert_eq!(cast(312, HAMSTRING).await, [(HAMSTRING, -60, 180_000)]);
    assert_eq!(cast(322, BLIND).await, [(BLIND, 30, 180_000)]);
}
