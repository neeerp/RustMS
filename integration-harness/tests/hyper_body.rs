//! Hyper Body raises max HP and MP by its percentages while it lasts: a
//! Power Elixir fills the raised maximum, and ending the buff brings HP and
//! MP back down to the plain one.

use integration_harness::database::{grant_skill, grant_stack, set_hp, set_job};
use integration_harness::hunting::{cancel_buff, use_item, use_skill};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{step, until};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const HYPER_BODY: i32 = 1301007;
const POWER_ELIXIR: i32 = 2000005;
const USE: i16 = 2;
/// STAT_CHANGED's HP and MP bits.
const HP: i32 = 0x400;
const MP: i32 = 0x1000;

#[tokio::test]
async fn hyper_body_raises_max_hp_and_mp() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 130, 70, 1000).await.expect("spearman");
    set_hp(id, 1000, 1000).await.expect("full HP");
    grant_skill(id, HYPER_BODY, 30)
        .await
        .expect("teach Hyper Body");
    grant_stack(id, USE, 1, POWER_ELIXIR, 2)
        .await
        .expect("give elixirs");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;

    // Hyper Body 30 raises both by 60%, so an elixir fills 1600 of each.
    let t = step(c, "hyper body", use_skill(HYPER_BODY, 30)).await;
    assert_eq!(t.stat(MP), Some(940));
    let mut t = step(c, "elixir", use_item(1, POWER_ELIXIR)).await;
    until(c, &mut t, |t| t.stat(HP).is_some()).await;
    assert_eq!((t.stat(HP), t.stat(MP)), (Some(1600), Some(1600)));

    // Cancelling it brings both back to 1000.
    let mut t = step(c, "cancel", cancel_buff(HYPER_BODY)).await;
    until(c, &mut t, |t| t.stat(HP).is_some()).await;
    assert_eq!((t.stat(HP), t.stat(MP)), (Some(1000), Some(1000)));
}
