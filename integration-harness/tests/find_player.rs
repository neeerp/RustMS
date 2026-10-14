use integration_harness::connection::MapleTestConnection;
use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::io::Cursor;
use tokio::time::{timeout, Duration};

/// The server's answer to a find, decoded.
#[derive(Debug, PartialEq, Eq)]
enum Found {
    Map { name: String, map_id: i32 },
    Channel { name: String, channel: i32 },
    NotOnline { name: String },
}

fn find(name: &str) -> Packet {
    let mut packet = Packet::new_empty();
    packet.write_short(RecvOpcode::Whisper as i16).unwrap();
    packet.write_byte(0x05).unwrap();
    packet.write_str_with_length(name).unwrap();
    packet
}

/// The next find answer, skipping field traffic.
async fn next_found(connection: &mut MapleTestConnection, phase: &'static str) -> Found {
    for _ in 0..20 {
        let envelope = timeout(Duration::from_secs(5), connection.read_packet(phase))
            .await
            .unwrap_or_else(|_| panic!("timed out: {phase}"))
            .unwrap_or_else(|e| panic!("{phase}: {e}"));
        if envelope.opcode() != SendOpcode::Whisper as i16 {
            continue;
        }
        let mut r = Cursor::new(&envelope.packet.bytes);
        r.read_short().unwrap();
        let mode = r.read_byte().unwrap();
        let name = r.read_str_with_length().unwrap();
        return match (mode, r.read_byte().unwrap()) {
            (0x09, 1) => {
                let map_id = r.read_int().unwrap();
                r.read_int().unwrap();
                r.read_int().unwrap();
                Found::Map { name, map_id }
            }
            (0x09, 3) => Found::Channel {
                name,
                channel: r.read_int().unwrap(),
            },
            (0x0A, 0) => Found::NotOnline { name },
            other => panic!("{phase}: unexpected whisper reply {other:?}"),
        };
    }
    panic!("no find answer: {phase}");
}

#[tokio::test]
async fn find_player() {
    let asker = load_harness_config_or_fail().await.with_channel(0, 0);
    let neighbour = load_harness_config_or_fail().await.with_channel(0, 0);
    let elsewhere = load_harness_config_or_fail().await.with_channel(0, 1);
    let mut asker = login_to_world_session(&asker)
        .await
        .expect("asker failed to login");
    let neighbour = login_to_world_session(&neighbour)
        .await
        .expect("neighbour failed to login");
    let elsewhere = login_to_world_session(&elsewhere)
        .await
        .expect("elsewhere failed to login");

    // On the asker's channel: their map.
    asker
        .connection
        .send_packet(find(&neighbour.character_name), "find neighbour")
        .await
        .unwrap();
    assert_eq!(
        next_found(&mut asker.connection, "neighbour found").await,
        Found::Map {
            name: neighbour.character_name.clone(),
            map_id: neighbour.map_id
        }
    );

    // On another channel: that channel, zero-based.
    asker
        .connection
        .send_packet(find(&elsewhere.character_name), "find elsewhere")
        .await
        .unwrap();
    assert_eq!(
        next_found(&mut asker.connection, "elsewhere found").await,
        Found::Channel {
            name: elsewhere.character_name.clone(),
            channel: 1
        }
    );

    // Not online: the failed whisper result.
    asker
        .connection
        .send_packet(find("NobodyHere"), "find offline")
        .await
        .unwrap();
    assert_eq!(
        next_found(&mut asker.connection, "offline not found").await,
        Found::NotOnline {
            name: "NobodyHere".into()
        }
    );
}
