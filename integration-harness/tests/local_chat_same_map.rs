use integration_harness::packets::{build_all_chat, decode_chat_text, decode_spawn_player};
use integration_harness::{login_two_players_to_world, MultiHarnessConfig};
use tokio::time::{timeout, Duration};

#[tokio::test]
#[ignore = "requires externally running login/world servers, two fixture players in integration-harness.toml, and same-map chat support"]
async fn local_chat_same_map() {
    let config =
        MultiHarnessConfig::from_file().expect("failed to load multi-player integration config");
    let (mut sender, mut recipient) = login_two_players_to_world(&config)
        .await
        .expect("two-player login-to-world flow failed");

    let sender_presence = timeout(
        Duration::from_secs(5),
        sender.connection.read_packet("sender same-map presence"),
    )
    .await
    .expect("timed out waiting for sender presence")
    .expect("failed to read sender presence");
    let _ = decode_spawn_player(&sender_presence.packet).expect("failed to decode sender presence");

    let recipient_presence = timeout(
        Duration::from_secs(5),
        recipient
            .connection
            .read_packet("recipient same-map presence"),
    )
    .await
    .expect("timed out waiting for recipient presence")
    .expect("failed to read recipient presence");
    let _ = decode_spawn_player(&recipient_presence.packet)
        .expect("failed to decode recipient presence");

    let message = "same-map chat integration";
    sender
        .connection
        .send_packet(
            build_all_chat(message, 0).expect("failed to build all-chat"),
            "send same-map chat",
        )
        .await
        .expect("failed to send all-chat packet");

    let sender_chat = timeout(
        Duration::from_secs(5),
        sender.connection.read_packet("sender same-map chat"),
    )
    .await
    .expect("timed out waiting for sender same-map chat")
    .expect("failed to read sender same-map chat");
    let sender_chat =
        decode_chat_text(&sender_chat.packet).expect("failed to decode sender same-map chat");
    assert_eq!(sender_chat.character_id, sender.character_id);
    assert_eq!(sender_chat.message, message);
    assert!(!sender_chat.from_admin);

    let recipient_chat = timeout(
        Duration::from_secs(5),
        recipient.connection.read_packet("recipient same-map chat"),
    )
    .await
    .expect("timed out waiting for recipient same-map chat")
    .expect("failed to read recipient same-map chat");
    let recipient_chat =
        decode_chat_text(&recipient_chat.packet).expect("failed to decode recipient same-map chat");
    assert_eq!(recipient_chat.character_id, sender.character_id);
    assert_eq!(recipient_chat.message, message);
    assert!(!recipient_chat.from_admin);
}
