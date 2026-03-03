use integration_harness::login_to_world;
use integration_harness::preconditions::load_harness_config_or_fail;

#[tokio::test]
async fn login_to_world_happy_path() {
    let config = load_harness_config_or_fail().await;
    let result = login_to_world(&config)
        .await
        .expect("login-to-world flow failed");

    assert_eq!(result.character_name, config.character_name);
    assert!(result.map_id >= 0, "expected a non-negative map id");
}
