use integration_harness::login_two_players_to_world;
use integration_harness::preconditions::load_multi_harness_config_or_fail;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn presence_different_channels_isolated() {
    let mut config = load_multi_harness_config_or_fail().await;
    config.players[0].channel_id = 0;
    config.players[1].channel_id = 1;

    let (mut first, mut second) = login_two_players_to_world(&config)
        .await
        .expect("two-player login-to-world flow failed");

    let first_presence = timeout(
        Duration::from_millis(500),
        first.connection.read_packet("first cross-channel presence"),
    )
    .await;
    assert!(
        first_presence.is_err(),
        "first player unexpectedly received cross-channel presence"
    );

    let second_presence = timeout(
        Duration::from_millis(500),
        second
            .connection
            .read_packet("second cross-channel presence"),
    )
    .await;
    assert!(
        second_presence.is_err(),
        "second player unexpectedly received cross-channel presence"
    );
}
