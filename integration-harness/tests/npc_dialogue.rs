use game_data::{GameData, NpcDialogueData};
use integration_harness::connection::MapleTestConnection;
use integration_harness::packets::{build_all_chat, build_change_map};
use integration_harness::preconditions::load_harness_config_or_fail;
use net::dialogue::DialoguePage;
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::{read::PktRead, write::PktWrite};
use packet::Packet;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::time::{timeout, Duration};

const START_MAP: i32 = 1_010_000;
const PIO: i32 = 10000;
const OBJECT_BASE: i32 = 1_000_000_000;

async fn login_to_world_session(
    config: &integration_harness::config::HarnessConfig,
) -> Result<integration_harness::protocol::WorldSession, integration_harness::error::HarnessError> {
    let mut session = integration_harness::login_to_world_session(config).await?;
    if session.map_id != START_MAP {
        session
            .connection
            .send_packet(
                build_change_map(START_MAP, "").unwrap(),
                "visit Pio in Amherst",
            )
            .await?;
        assert_no_dialogue(&mut session.connection).await;
        session.map_id = START_MAP;
    }
    Ok(session)
}

fn assets() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data")
}

fn pio_object_id() -> i32 {
    let maps = GameData::load_from_nx_map(assets().join("Map.nx")).unwrap();
    let index = maps
        .field(START_MAP)
        .unwrap()
        .map_npcs
        .iter()
        .position(|npc| npc.npc_id == PIO)
        .expect("Pio in Amherst");
    OBJECT_BASE + index as i32
}

fn talk(object_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(object_id).unwrap();
    p
}

fn response(message_type: u8, response: i8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::NpcTalkMore as i16).unwrap();
    p.write_byte(message_type).unwrap();
    p.write_byte(response as u8).unwrap();
    p
}

async fn read(connection: &mut MapleTestConnection) -> Packet {
    timeout(
        Duration::from_secs(5),
        connection.read_packet("NPC dialogue"),
    )
    .await
    .expect("packet timeout")
    .expect("read packet")
    .packet
}

async fn page(connection: &mut MapleTestConnection) -> DialoguePage {
    for _ in 0..128 {
        let packet = read(connection).await;
        match packet.opcode() {
            op if op == SendOpcode::SpawnPlayer as i16
                || op == SendOpcode::RemovePlayerFromMap as i16 =>
            {
                continue
            }
            op if op == SendOpcode::ChatText as i16 => continue,
            // NPC control passes on as other tests' players leave.
            op if op == SendOpcode::SpawnNpcRequestController as i16 => continue,
            op => assert_eq!(op, SendOpcode::NpcDialogue as i16),
        }
        let mut r = Cursor::new(&packet.bytes);
        r.read_short().unwrap();
        assert_eq!(r.read_byte().unwrap(), 4);
        let npc_id = r.read_int().unwrap();
        assert_eq!(r.read_byte().unwrap(), 0);
        assert_eq!(r.read_byte().unwrap(), 0);
        let text = r.read_str_with_length().unwrap();
        let has_previous = r.read_byte().unwrap() != 0;
        let has_next = r.read_byte().unwrap() != 0;
        assert_eq!(r.position() as usize, packet.bytes.len());
        return DialoguePage {
            npc_id,
            text,
            has_previous,
            has_next,
            player_speaks: false,
        };
    }
    panic!("too many packets before dialogue");
}

/// Local chat is routed through the same actors, so its echo is a barrier:
/// a previous NPC request must be processed before this packet. This avoids
/// timing-based negative assertions and cancelling an encrypted read mid-packet.
async fn assert_no_dialogue(connection: &mut MapleTestConnection) {
    static NEXT_BARRIER: AtomicU64 = AtomicU64::new(0);
    let marker = format!(
        "npc-dialogue-barrier-{}",
        NEXT_BARRIER.fetch_add(1, Ordering::Relaxed)
    );
    connection
        .send_packet(build_all_chat(&marker, 0).unwrap(), "dialogue barrier")
        .await
        .unwrap();
    for _ in 0..128 {
        let packet = read(connection).await;
        assert_ne!(
            packet.opcode(),
            SendOpcode::NpcDialogue as i16,
            "unexpected conversation response"
        );
        if packet.opcode() == SendOpcode::ChatText as i16 {
            let mut r = Cursor::new(&packet.bytes);
            r.read_short().unwrap();
            r.read_int().unwrap();
            r.read_byte().unwrap();
            if r.read_str_with_length().unwrap() == marker {
                return;
            }
        }
    }
    panic!("missing dialogue barrier");
}

#[tokio::test]
async fn asset_dialogue_supports_next_back_finish_cancel_and_restart() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config).await.unwrap();
    let conn = &mut session.connection;
    let data =
        NpcDialogueData::load_from_nx(assets().join("Npc.nx"), assets().join("String.nx")).unwrap();
    let text = &data.dialogue(PIO).unwrap().pages;
    conn.send_packet(talk(pio_object_id()), "talk to Pio")
        .await
        .unwrap();
    let first = page(conn).await;
    assert_eq!(first.npc_id, PIO);
    assert_eq!(first.text, text[0]);
    assert!(!first.has_previous && first.has_next);
    conn.send_packet(response(0, 1), "next").await.unwrap();
    let second = page(conn).await;
    assert_eq!(second.text, text[1]);
    assert!(second.has_previous && second.has_next);
    conn.send_packet(response(0, 0), "back").await.unwrap();
    assert_eq!(page(conn).await, first);
    for (index, expected) in text.iter().enumerate().skip(1) {
        conn.send_packet(response(0, 1), "next").await.unwrap();
        let current = page(conn).await;
        assert_eq!(&current.text, expected);
        assert_eq!(current.has_next, index + 1 < text.len());
    }
    // Finish and then send a stale response: neither may produce a new page.
    conn.send_packet(response(0, 1), "finish").await.unwrap();
    conn.send_packet(response(0, 1), "stale response")
        .await
        .unwrap();
    assert_no_dialogue(conn).await;
    conn.send_packet(talk(pio_object_id()), "restart")
        .await
        .unwrap();
    assert_eq!(page(conn).await, first);
    conn.send_packet(response(0, -1), "cancel").await.unwrap();
    conn.send_packet(response(0, 1), "stale response after cancel")
        .await
        .unwrap();
    assert_no_dialogue(conn).await;
    conn.send_packet(talk(pio_object_id()), "restart after cancel")
        .await
        .unwrap();
    assert_eq!(page(conn).await, first);
    conn.send_packet(response(1, 1), "wrong response type")
        .await
        .unwrap();
    conn.send_packet(response(0, 1), "stale after wrong type")
        .await
        .unwrap();
    assert_no_dialogue(conn).await;
}

#[tokio::test]
async fn players_have_independent_conversations() {
    let first_config = load_harness_config_or_fail().await;
    let second_config = load_harness_config_or_fail().await;
    let mut first = login_to_world_session(&first_config).await.unwrap();
    let mut second = login_to_world_session(&second_config).await.unwrap();
    let object_id = pio_object_id();
    first
        .connection
        .send_packet(talk(object_id), "first player starts")
        .await
        .unwrap();
    let opening = page(&mut first.connection).await;
    first
        .connection
        .send_packet(response(0, 1), "first player advances")
        .await
        .unwrap();
    let next = page(&mut first.connection).await;
    assert_ne!(next.text, opening.text);
    second
        .connection
        .send_packet(response(0, 1), "second player has no conversation")
        .await
        .unwrap();
    assert_no_dialogue(&mut second.connection).await;
    second
        .connection
        .send_packet(talk(object_id), "second player starts")
        .await
        .unwrap();
    assert_eq!(page(&mut second.connection).await, opening);
    second
        .connection
        .send_packet(response(0, -1), "second player cancels")
        .await
        .unwrap();
    assert_no_dialogue(&mut second.connection).await;
    first
        .connection
        .send_packet(response(0, 0), "first player goes back")
        .await
        .unwrap();
    assert_eq!(page(&mut first.connection).await, opening);
}

#[tokio::test]
async fn npc_requests_are_field_scoped_and_warp_clears_conversation() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config).await.unwrap();
    let conn = &mut session.connection;
    conn.send_packet(response(0, 1), "response without conversation")
        .await
        .unwrap();
    conn.send_packet(talk(-1), "invalid object").await.unwrap();
    // Template IDs are not map object IDs.
    conn.send_packet(talk(PIO), "forged template id")
        .await
        .unwrap();
    assert_no_dialogue(conn).await;
    conn.send_packet(talk(pio_object_id()), "start before warp")
        .await
        .unwrap();
    page(conn).await;
    conn.send_packet(
        build_change_map(10_000, "").unwrap(),
        "warp during conversation",
    )
    .await
    .unwrap();
    conn.send_packet(response(0, 1), "stale response after warp")
        .await
        .unwrap();
    assert_no_dialogue(conn).await;
    conn.send_packet(
        build_change_map(START_MAP, "").unwrap(),
        "return to Amherst",
    )
    .await
    .unwrap();
    conn.send_packet(response(0, 1), "stale response after return")
        .await
        .unwrap();
    assert_no_dialogue(conn).await;
    conn.send_packet(talk(pio_object_id()), "restart in Amherst")
        .await
        .unwrap();
    assert!(!page(conn).await.has_previous);
}
