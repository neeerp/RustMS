use integration_harness::packets::{build_whisper, decode_whisper_receive, decode_whisper_result};
use integration_harness::{login_two_players_to_world, MultiHarnessConfig};
use tokio::time::{timeout, Duration};

#[tokio::test]
#[ignore = "requires externally running login/world servers, two fixture players in integration-harness.toml, and whisper server support"]
async fn whisper_between_two_players() {
    let config =
        MultiHarnessConfig::from_file().expect("failed to load multi-player integration config");
    let (mut sender, mut recipient) = login_two_players_to_world(&config)
        .await
        .expect("two-player login-to-world flow failed");

    let message = "integration whisper test";
    sender
        .connection
        .send_packet(
            build_whisper(&recipient.character_name, message).expect("failed to build whisper"),
            "send whisper",
        )
        .await
        .expect("failed to send whisper packet");

    let sender_result_envelope = timeout(
        Duration::from_secs(5),
        sender.connection.read_packet("sender whisper result"),
    )
    .await
    .expect("timed out waiting for sender whisper result")
    .expect("failed to read sender whisper result");
    let sender_result = decode_whisper_result(&sender_result_envelope.packet)
        .expect("failed to decode sender whisper result");
    assert_eq!(sender_result.target_name, recipient.character_name);
    assert!(sender_result.success, "expected whisper delivery success");

    let recipient_envelope = timeout(
        Duration::from_secs(5),
        recipient.connection.read_packet("recipient whisper receive"),
    )
    .await
    .expect("timed out waiting for recipient whisper packet")
    .expect("failed to read recipient whisper packet");
    let recipient_whisper = decode_whisper_receive(&recipient_envelope.packet)
        .expect("failed to decode recipient whisper packet");
    assert_eq!(recipient_whisper.sender_name, sender.character_name);
    assert_eq!(recipient_whisper.message, message);
    assert_eq!(recipient_whisper.channel, 1);
    assert!(!recipient_whisper.from_admin);

    sender
        .connection
        .send_packet(
            build_whisper(&sender.character_name, "self whisper should fail")
                .expect("failed to build self-whisper"),
            "self whisper",
        )
        .await
        .expect("failed to send self-whisper packet");

    let self_result_envelope = timeout(
        Duration::from_secs(5),
        sender.connection.read_packet("self whisper result"),
    )
    .await
    .expect("timed out waiting for self-whisper result")
    .expect("failed to read self-whisper result");
    let self_result = decode_whisper_result(&self_result_envelope.packet)
        .expect("failed to decode self-whisper result");
    assert_eq!(self_result.target_name, sender.character_name);
    assert!(
        !self_result.success,
        "expected self-whisper to be rejected with a failure result"
    );
}
