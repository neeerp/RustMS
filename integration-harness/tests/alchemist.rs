//! Alchemist strengthens a Hermit's potions: at level 20 a Red Potion heals
//! 150% of its 50 HP.

use integration_harness::database::{grant_skill, grant_stack, set_hp, set_job};
use integration_harness::hunting::use_item;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{step, until};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const ALCHEMIST: i32 = 4110000;
const RED_POTION: i32 = 2000000;
const USE: i16 = 2;
/// STAT_CHANGED's HP bit.
const HP: i32 = 0x400;

#[tokio::test]
async fn alchemist_strengthens_potions() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 411, 70, 100).await.expect("hermit");
    set_hp(id, 100, 1000).await.expect("hurt");
    grant_skill(id, ALCHEMIST, 20)
        .await
        .expect("teach Alchemist");
    grant_stack(id, USE, 1, RED_POTION, 1)
        .await
        .expect("give a potion");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;

    let mut t = step(c, "red potion", use_item(1, RED_POTION)).await;
    until(c, &mut t, |t| t.stat(HP).is_some()).await;
    assert_eq!(t.stat(HP), Some(175));
}
