//! Double-clicking a character (CHAR_INFO_REQUEST) answers with their
//! CHAR_INFO when they are on the requester's map, the requester included.

use integration_harness::connection::MapleTestConnection;
use integration_harness::login_two_players_to_world;
use integration_harness::preconditions::load_multi_harness_config_or_fail;
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::{read::PktRead, write::PktWrite};
use packet::Packet;
use std::io::Cursor;
use tokio::time::{timeout, Duration, Instant};

fn char_info_request(character_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::CharInfoRequest as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_int(character_id).unwrap();
    p
}

/// Ask for `character_id`, then return the (id, level) of every CHAR_INFO
/// that arrives within half a second.
async fn ask(c: &mut MapleTestConnection, character_id: i32) -> Vec<(i32, u8)> {
    c.send_packet(char_info_request(character_id), "char info request")
        .await
        .unwrap();
    let deadline = Instant::now() + Duration::from_millis(500);
    let mut answers = Vec::new();
    while let Ok(Ok(envelope)) = timeout(
        deadline.saturating_duration_since(Instant::now()),
        c.read_packet("char info"),
    )
    .await
    {
        let p = envelope.packet;
        if p.opcode() == SendOpcode::CharInfo as i16 {
            let mut r = Cursor::new(&p.bytes);
            r.read_short().unwrap();
            answers.push((r.read_int().unwrap(), r.read_byte().unwrap()));
        }
    }
    answers
}

#[tokio::test]
async fn players_on_a_map_can_see_each_others_info() {
    let config = load_multi_harness_config_or_fail().await;
    let (mut first, mut second) = login_two_players_to_world(&config)
        .await
        .expect("two-player login-to-world flow failed");

    assert_eq!(
        ask(&mut first.connection, second.character_id).await,
        [(second.character_id, second.level)]
    );
    assert_eq!(
        ask(&mut second.connection, first.character_id).await,
        [(first.character_id, first.level)]
    );
    assert_eq!(
        ask(&mut first.connection, first.character_id).await,
        [(first.character_id, first.level)]
    );
    // Nobody with this id is on the map.
    assert_eq!(ask(&mut first.connection, i32::MAX).await, []);
}
