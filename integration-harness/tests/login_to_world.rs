use integration_harness::{login_to_world, HarnessConfig};

#[tokio::test]
#[ignore = "requires externally running login/world servers, a fixture account/character, and integration-harness.toml"]
async fn login_to_world_happy_path() {
    let config = HarnessConfig::from_file().expect("failed to load integration harness config");
    let result = login_to_world(&config)
        .await
        .expect("login-to-world flow failed");

    assert_eq!(result.character_name, config.character_name);
    assert!(result.map_id >= 0, "expected a non-negative map id");
}
