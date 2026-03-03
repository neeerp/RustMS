use integration_harness::packets::decode_spawn_player;
use integration_harness::{login_two_players_to_world, MultiHarnessConfig};
use tokio::time::{timeout, Duration};

#[tokio::test]
#[ignore = "requires externally running login/world servers, two fixture players in integration-harness.toml, and same-map presence support"]
async fn presence_same_map() {
    let config =
        MultiHarnessConfig::from_file().expect("failed to load multi-player integration config");
    let (mut first, mut second) = login_two_players_to_world(&config)
        .await
        .expect("two-player login-to-world flow failed");

    let first_presence = timeout(
        Duration::from_secs(5),
        first.connection.read_packet("first same-map presence"),
    )
    .await
    .expect("timed out waiting for first presence packet")
    .expect("failed to read first presence packet");
    let first_spawn = decode_spawn_player(&first_presence.packet)
        .expect("failed to decode first presence packet");
    assert_eq!(first_spawn.character_id, second.character_id);
    assert_eq!(first_spawn.character_name, second.character_name);

    let second_presence = timeout(
        Duration::from_secs(5),
        second.connection.read_packet("second same-map presence"),
    )
    .await
    .expect("timed out waiting for second presence packet")
    .expect("failed to read second presence packet");
    let second_spawn = decode_spawn_player(&second_presence.packet)
        .expect("failed to decode second presence packet");
    assert_eq!(second_spawn.character_id, first.character_id);
    assert_eq!(second_spawn.character_name, first.character_name);
}
