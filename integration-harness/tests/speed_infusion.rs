//! Speed Infusion gives its attack speed as a two-state stat, whose data
//! follows GIVE_BUFF's defense bytes, and the server ends it on time.

use integration_harness::database::{grant_skill, set_job};
use integration_harness::hunting::{cancel_buff, use_skill};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::step;
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const SPEED_INFUSION: i32 = 5121009;

#[tokio::test]
async fn speed_infusion_gives_its_attack_speed() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 512, 120, 1000).await.expect("corsair");
    grant_skill(id, SPEED_INFUSION, 20)
        .await
        .expect("teach Speed Infusion");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;

    // Level 20 speeds attacks up 2 steps for 300 seconds.
    let t = step(c, "infuse", use_skill(SPEED_INFUSION, 20)).await;
    assert_eq!(t.buffs, [(SPEED_INFUSION, -2, 300_000)]);
    let t = step(c, "cancel", cancel_buff(SPEED_INFUSION)).await;
    // Its bit 86 is in the first long, which the trace doesn't keep.
    assert_eq!(t.cancelled_buffs, [0]);
}
