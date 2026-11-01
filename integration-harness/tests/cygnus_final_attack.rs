//! A Dawn Warrior's Final Attack (11101002) is a buff: casting it gives the
//! temp stat (bit 63) under which the client rolls its swing, for the
//! level's `time`.

use integration_harness::database::{grant_skill, set_job};
use integration_harness::hunting::use_skill;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{step, until};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const FINAL_ATTACK: i32 = 11101002;

#[tokio::test]
async fn a_dawn_warriors_final_attack_is_a_buff() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 1110, 70, 1000).await.expect("dawn warrior");
    grant_skill(id, FINAL_ATTACK, 30)
        .await
        .expect("teach Final Attack");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = step(c, "final attack", use_skill(FINAL_ATTACK, 30)).await;
    until(c, &mut t, |t| !t.buffs.is_empty()).await;
    assert_eq!(t.buffs, [(FINAL_ATTACK, 1, 300_000)]);
}
