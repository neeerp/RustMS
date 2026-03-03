use integration_harness::packets::{build_player_move, decode_move_player, decode_spawn_player};
use integration_harness::preconditions::load_multi_harness_config_or_fail;
use integration_harness::{login_two_players_to_world, MultiHarnessConfig};
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn movement_same_map() {
    let config = load_multi_harness_config_or_fail().await;
    let (mut mover, mut observer) = login_two_players_to_world(&config)
        .await
        .expect("two-player login-to-world flow failed");

    let mover_presence = timeout(
        Duration::from_secs(5),
        mover.connection.read_packet("mover same-map presence"),
    )
    .await
    .expect("timed out waiting for mover presence")
    .expect("failed to read mover presence");
    let _ = decode_spawn_player(&mover_presence.packet).expect("failed to decode mover presence");

    let observer_presence = timeout(
        Duration::from_secs(5),
        observer
            .connection
            .read_packet("observer same-map presence"),
    )
    .await
    .expect("timed out waiting for observer presence")
    .expect("failed to read observer presence");
    let _ =
        decode_spawn_player(&observer_presence.packet).expect("failed to decode observer presence");

    let movement_bytes = vec![0x01, 0x02, 0x03, 0x04, 0x05];
    mover
        .connection
        .send_packet(
            build_player_move(&movement_bytes).expect("failed to build move packet"),
            "send same-map move",
        )
        .await
        .expect("failed to send move packet");

    let observer_move = timeout(
        Duration::from_secs(5),
        observer.connection.read_packet("observer same-map move"),
    )
    .await
    .expect("timed out waiting for observer move packet")
    .expect("failed to read observer move packet");
    let observer_move =
        decode_move_player(&observer_move.packet).expect("failed to decode observer move packet");
    assert_eq!(observer_move.character_id, mover.character_id);
    assert_eq!(observer_move.movement_bytes, movement_bytes);
}
