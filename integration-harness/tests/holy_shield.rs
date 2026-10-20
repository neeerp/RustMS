//! Holy Shield is a buff the client only shows: a Bishop's cast gives it at
//! its `x` of 1 for its `time`.

use integration_harness::database::{grant_skill, set_job};
use integration_harness::hunting::use_skill;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::step;
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const HOLY_SHIELD: i32 = 2321005;

#[tokio::test]
async fn holy_shield_is_given_as_a_buff() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 232, 120, 1000).await.expect("bishop");
    grant_skill(id, HOLY_SHIELD, 1)
        .await
        .expect("teach Holy Shield");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;

    let t = step(c, "holy shield", use_skill(HOLY_SHIELD, 1)).await;
    assert_eq!(t.buffs, [(HOLY_SHIELD, 1, 11_000)]);
}
