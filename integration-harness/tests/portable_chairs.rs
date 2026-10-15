use integration_harness::connection::MapleTestConnection;
use integration_harness::database::grant_item;
use integration_harness::packets::{
    build_all_chat, build_cancel_chair, build_use_chair, decode_chat_text,
};
use integration_harness::preconditions::load_multi_harness_config_or_fail;
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::SendOpcode;
use packet::io::read::PktRead;
use packet::Packet;
use std::io::Cursor;
use tokio::time::{timeout, Duration};

const RELAXER: i32 = 3010000;
const SETUP: i16 = 3;

/// What was read before the chat line `marker`: the (character id, chair) of
/// every SHOW_CHAIR and the body of every CANCEL_CHAIR.
async fn chairs_until(
    c: &mut MapleTestConnection,
    marker: &str,
) -> (Vec<(i32, i32)>, Vec<Vec<u8>>) {
    let (mut shown, mut cancelled) = (Vec::new(), Vec::new());
    loop {
        let p = timeout(Duration::from_secs(5), c.read_packet("chairs"))
            .await
            .expect("timed out waiting for the marker")
            .expect("failed to read packet")
            .packet;
        if p.opcode() == SendOpcode::ShowChair as i16 {
            let mut r = Cursor::new(&p.bytes);
            r.read_short().unwrap();
            shown.push((r.read_int().unwrap(), r.read_int().unwrap()));
        } else if p.opcode() == SendOpcode::CancelChair as i16 {
            cancelled.push(p.bytes[2..].to_vec());
        } else if p.opcode() == SendOpcode::ChatText as i16
            && decode_chat_text(&p).unwrap().message == marker
        {
            return (shown, cancelled);
        }
    }
}

/// Send `packet`, then the chat line `marker`.
async fn send(c: &mut MapleTestConnection, packet: Packet, marker: &str) {
    c.send_packet(packet, "chair").await.unwrap();
    c.send_packet(build_all_chat(marker, 0).unwrap(), "marker")
        .await
        .unwrap();
}

#[tokio::test]
async fn the_map_sees_who_sits_on_a_portable_chair() {
    let config = load_multi_harness_config_or_fail().await;
    let sitter_config = config.player("sender").unwrap();
    let observer_config = config.player("recipient").unwrap();

    // Make the character, and let its logout save before giving it a chair.
    let character_id = login_to_world_session(&sitter_config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&sitter_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_item(character_id, SETUP, 1, RELAXER)
        .await
        .expect("grant the Relaxer");

    let mut sitter = relogin(&sitter_config).await;
    let mut observer = login_to_world_session(&observer_config)
        .await
        .expect("observer login failed");

    // A chair the player doesn't own is ignored; the Relaxer is shown.
    send(
        &mut sitter.connection,
        build_use_chair(3010001).unwrap(),
        "not owned",
    )
    .await;
    send(
        &mut sitter.connection,
        build_use_chair(RELAXER).unwrap(),
        "sat",
    )
    .await;
    assert_eq!(
        chairs_until(&mut observer.connection, "not owned").await,
        (vec![], vec![])
    );
    assert_eq!(
        chairs_until(&mut observer.connection, "sat").await,
        (vec![(sitter.character_id, RELAXER)], vec![])
    );

    // Standing up is confirmed to the sitter and shown to the map.
    send(
        &mut sitter.connection,
        build_cancel_chair(-1).unwrap(),
        "stood",
    )
    .await;
    assert_eq!(
        chairs_until(&mut observer.connection, "stood").await,
        (vec![(sitter.character_id, 0)], vec![])
    );
    let (shown, cancelled) = chairs_until(&mut sitter.connection, "stood").await;
    assert!(shown.is_empty());
    assert_eq!(cancelled, [vec![0]]);
}
