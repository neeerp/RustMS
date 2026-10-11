use integration_harness::login_to_world;
use integration_harness::preconditions::load_harness_config_or_fail;

#[tokio::test]
async fn login_to_world_happy_path() {
    let config = load_harness_config_or_fail().await;
    let result = login_to_world(&config)
        .await
        .expect("login-to-world flow failed");

    assert_eq!(result.character_name, config.character_name);
    assert_eq!(
        result.map_id, 10_000,
        "new Explorers must enter the tutorial route"
    );
}
