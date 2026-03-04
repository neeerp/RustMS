use integration_harness::packets::build_change_map;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::{login_to_world_session, HarnessError};
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn invalid_direct_map_target_disconnects_client() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config)
        .await
        .expect("login-to-world session flow failed");

    session
        .connection
        .send_packet(
            build_change_map(2_000_000_000, "").expect("failed to build invalid change-map packet"),
            "invalid direct map transfer",
        )
        .await
        .expect("failed to send invalid change-map packet");

    let read_result = timeout(
        Duration::from_secs(5),
        session
            .connection
            .read_packet("invalid direct map transfer response"),
    )
    .await
    .expect("timed out waiting for invalid map failure");

    assert!(
        matches!(
            read_result,
            Err(HarnessError::Io { .. }) | Err(HarnessError::Protocol { .. })
        ),
        "expected invalid target map transfer to fail, got: {read_result:?}"
    );
}
