//! The login result's grade: a player's account and a GM's below level 3
//! log in with none, a GM's from level 3 with the GM bit, and an account
//! flagged `test_admin` as v83's admin account (0x11), whatever its GM
//! level. The sub-grade is always 0.

use integration_harness::database::{set_gm_level, set_test_admin};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::{login_grade, relogin};

#[tokio::test]
async fn the_grade_follows_the_gm_level_and_the_admin_flag() {
    let config = load_harness_config_or_fail().await;
    drop(relogin(&config).await);
    assert_eq!(login_grade(&config).await, (0, 0), "a player's");

    set_gm_level(&config.username, 2).await.expect("a helper");
    assert_eq!(login_grade(&config).await, (0, 0), "below level 3");

    set_gm_level(&config.username, 3)
        .await
        .expect("an admin GM");
    assert_eq!(login_grade(&config).await, (0x01, 0), "from level 3");

    set_gm_level(&config.username, 0)
        .await
        .expect("a player again");
    set_test_admin(&config.username, true)
        .await
        .expect("the admin account");
    assert_eq!(login_grade(&config).await, (0x11, 0), "the admin account");
}
