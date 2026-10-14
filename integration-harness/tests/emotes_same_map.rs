use integration_harness::connection::MapleTestConnection;
use integration_harness::login_two_players_to_world;
use integration_harness::packets::{build_all_chat, build_face_expression, decode_chat_text};
use integration_harness::preconditions::load_multi_harness_config_or_fail;
use net::packet::op::SendOpcode;
use packet::io::read::PktRead;
use std::io::Cursor;
use tokio::time::{timeout, Duration};

/// The (character id, emote) of every FACIAL_EXPRESSION read before the chat
/// line `marker`.
async fn expressions_until(c: &mut MapleTestConnection, marker: &str) -> Vec<(i32, i32)> {
    let mut shown = Vec::new();
    loop {
        let p = timeout(Duration::from_secs(5), c.read_packet("emotes"))
            .await
            .expect("timed out waiting for the marker")
            .expect("failed to read packet")
            .packet;
        if p.opcode() == SendOpcode::FacialExpression as i16 {
            let mut r = Cursor::new(&p.bytes);
            r.read_short().unwrap();
            shown.push((r.read_int().unwrap(), r.read_int().unwrap()));
        } else if p.opcode() == SendOpcode::ChatText as i16
            && decode_chat_text(&p).unwrap().message == marker
        {
            return shown;
        }
    }
}

#[tokio::test]
async fn emotes_are_shown_to_the_rest_of_the_map() {
    let config = load_multi_harness_config_or_fail().await;
    let (mut sender, mut observer) = login_two_players_to_world(&config)
        .await
        .expect("two-player login-to-world flow failed");

    // Smile, then an emote that needs a cash item.
    for emote in [2, 9] {
        sender
            .connection
            .send_packet(build_face_expression(emote).unwrap(), "emote")
            .await
            .unwrap();
    }
    let marker = "emotes sent";
    sender
        .connection
        .send_packet(build_all_chat(marker, 0).unwrap(), "marker")
        .await
        .unwrap();

    assert_eq!(
        expressions_until(&mut observer.connection, marker).await,
        [(sender.character_id, 2)]
    );
    // The sender already shows its own emote.
    assert!(expressions_until(&mut sender.connection, marker)
        .await
        .is_empty());
}
