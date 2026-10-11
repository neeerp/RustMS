use game_data::GameData;
use integration_harness::packets::{
    build_all_chat, build_change_channel, build_change_map, build_player_logged_in_for_channel,
    decode_channel_change, decode_set_field,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::{
    connection::MapleTestConnection, login_to_world_session, protocol::WorldSession,
};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::{
    io::{read::PktRead, write::PktWrite},
    Packet,
};
use std::io::Cursor;
use std::path::Path;
use tokio::time::{timeout, Duration};

#[derive(Default)]
struct Trace {
    progress: Vec<(i16, u8)>,
    gains: Vec<i32>,
    stats: Option<(u8, i16, i32)>,
}
struct Prompt {
    kind: u8,
    text: String,
    previous: bool,
    next: bool,
}

fn talk() -> Packet {
    let maps = GameData::load_from_nx_map(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx"),
    )
    .unwrap();
    let index = maps
        .field(1_000_000)
        .unwrap()
        .map_npcs
        .iter()
        .position(|n| n.npc_id == 12101)
        .unwrap();
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(1_000_000_000 + index as i32).unwrap();
    p
}
fn response(kind: u8, value: i8, selection: Option<i32>) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::NpcTalkMore as i16).unwrap();
    p.write_byte(kind).unwrap();
    p.write_byte(value as u8).unwrap();
    if let Some(selected) = selection {
        p.write_int(selected).unwrap();
    }
    p
}
async fn send(conn: &mut MapleTestConnection, p: Packet) {
    conn.send_packet(p, "Rain quiz").await.unwrap();
}
async fn read(conn: &mut MapleTestConnection) -> Packet {
    timeout(Duration::from_secs(5), conn.read_packet("Rain quiz"))
        .await
        .expect("packet timeout")
        .expect("read packet")
        .packet
}
fn observe(p: &Packet, trace: &mut Trace) {
    let mut r = Cursor::new(&p.bytes);
    r.read_short().unwrap();
    if p.opcode() == SendOpcode::ShowStatusInfo as i16 {
        match r.read_byte().unwrap() {
            1 => trace
                .progress
                .push((r.read_short().unwrap(), r.read_byte().unwrap())),
            3 => {
                r.read_byte().unwrap();
                trace.gains.push(r.read_int().unwrap());
            }
            mode => panic!("unexpected status mode {mode}"),
        }
    } else if p.opcode() == SendOpcode::StatChange as i16 {
        r.read_byte().unwrap();
        let mask = r.read_int().unwrap();
        if mask != 0 {
            assert_eq!(mask, 0x12010);
            trace.stats = Some((
                r.read_byte().unwrap(),
                r.read_short().unwrap(),
                r.read_int().unwrap(),
            ));
        }
    }
}
async fn prompt(conn: &mut MapleTestConnection, trace: &mut Trace) -> Prompt {
    loop {
        let p = read(conn).await;
        observe(&p, trace);
        if p.opcode() != SendOpcode::NpcDialogue as i16 {
            continue;
        }
        let mut r = Cursor::new(&p.bytes);
        r.read_short().unwrap();
        assert_eq!(r.read_byte().unwrap(), 4);
        assert_eq!(r.read_int().unwrap(), 12101);
        let kind = r.read_byte().unwrap();
        r.read_byte().unwrap();
        let text = r.read_str_with_length().unwrap();
        let (previous, next) = if kind == 0 {
            (r.read_byte().unwrap() != 0, r.read_byte().unwrap() != 0)
        } else {
            (false, false)
        };
        return Prompt {
            kind,
            text,
            previous,
            next,
        };
    }
}
async fn question(conn: &mut MapleTestConnection, trace: &mut Trace) -> Prompt {
    let mut p = prompt(conn, trace).await;
    for _ in 0..12 {
        if p.kind == 4 {
            return p;
        }
        assert!(matches!(p.kind, 0 | 1));
        send(conn, response(p.kind, 1, None)).await;
        p = prompt(conn, trace).await;
    }
    panic!("did not reach quiz question");
}
async fn finish_feedback(conn: &mut MapleTestConnection, trace: &mut Trace) {
    let mut p = prompt(conn, trace).await;
    loop {
        assert_eq!(p.kind, 0);
        let next = p.next;
        send(conn, response(0, 1, None)).await;
        if !next {
            break;
        }
        p = prompt(conn, trace).await;
    }
}
async fn barrier(conn: &mut MapleTestConnection, marker: &str, trace: &mut Trace) {
    send(conn, build_all_chat(marker, 0).unwrap()).await;
    loop {
        let p = read(conn).await;
        assert_ne!(
            p.opcode(),
            SendOpcode::NpcDialogue as i16,
            "unexpected dialogue after cancellation/stale response"
        );
        observe(&p, trace);
        if p.opcode() == SendOpcode::ChatText as i16 {
            let mut r = Cursor::new(&p.bytes);
            r.read_short().unwrap();
            r.read_int().unwrap();
            r.read_byte().unwrap();
            if r.read_str_with_length().unwrap() == marker {
                break;
            }
        }
    }
}

#[tokio::test]
async fn seven_quizzes_award_once_and_survive_map_changes_and_relogin() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config).await.unwrap();
    let mut trace = Trace::default();
    assert_eq!((session.level, session.exp, session.ap), (1, 0, 0));
    for (index, answer) in [0, 0, 0, 3, 1, 2, 0].into_iter().enumerate() {
        send(&mut session.connection, talk()).await;
        let q = question(&mut session.connection, &mut trace).await;
        assert!(q.text.contains(&format!("#L{answer}#")));
        let id = 1009 + index as i16;
        assert!(trace.progress.contains(&(id, 1)));
        if index == 0 {
            // Wrong answer explains why, returns to the same question, and pays nothing.
            send(&mut session.connection, response(4, 1, Some(1))).await;
            let wrong = prompt(&mut session.connection, &mut trace).await;
            assert!(wrong.text.contains("Skill Window"));
            assert!(trace.gains.is_empty());
            send(&mut session.connection, response(0, 1, None)).await;
            assert_eq!(
                prompt(&mut session.connection, &mut trace).await.text,
                q.text
            );
        }
        send(&mut session.connection, response(4, 1, Some(answer))).await;
        finish_feedback(&mut session.connection, &mut trace).await;
        assert!(trace.progress.contains(&(id, 2)));
        // Replayed choice after completion must not grant EXP a second time.
        send(&mut session.connection, response(4, 1, Some(answer))).await;
        barrier(
            &mut session.connection,
            &format!("{}-quiz-{index}", config.character_name),
            &mut trace,
        )
        .await;
        assert_eq!(trace.gains.len(), index + 1);
    }
    assert_eq!(trace.gains, [2, 4, 6, 10, 10, 10, 20]);
    assert_eq!(trace.stats, Some((3, 10, 13)));
    send(
        &mut session.connection,
        build_change_map(10_000, "").unwrap(),
    )
    .await;
    barrier(
        &mut session.connection,
        &format!("{}-warp", config.character_name),
        &mut trace,
    )
    .await;
    send(
        &mut session.connection,
        build_change_map(1_000_000, "").unwrap(),
    )
    .await;
    barrier(
        &mut session.connection,
        &format!("{}-return", config.character_name),
        &mut trace,
    )
    .await;
    drop(session);
    let restored = login_to_world_session(&config).await.unwrap();
    assert_eq!((restored.level, restored.ap, restored.exp), (3, 10, 13));
    assert_eq!(restored.quests.len(), 7);
    assert!(restored.quests.values().all(|state| *state == 2));
}

#[tokio::test]
async fn decline_cancel_invalid_answers_and_player_isolation() {
    let config = load_harness_config_or_fail().await;
    let other = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config).await.unwrap();
    let mut trace = Trace::default();
    send(&mut session.connection, talk()).await;
    assert_eq!(prompt(&mut session.connection, &mut trace).await.kind, 1);
    send(&mut session.connection, response(1, 0, None)).await;
    finish_feedback(&mut session.connection, &mut trace).await;
    assert!(trace.progress.is_empty());
    send(&mut session.connection, talk()).await;
    prompt(&mut session.connection, &mut trace).await;
    // A forged answer while still at the offer cannot start or complete anything.
    send(&mut session.connection, response(4, 1, Some(0))).await;
    barrier(&mut session.connection, &config.character_name, &mut trace).await;
    assert!(trace.progress.is_empty());
    send(&mut session.connection, talk()).await;
    question(&mut session.connection, &mut trace).await;
    send(&mut session.connection, response(4, 1, Some(99))).await;
    barrier(&mut session.connection, &config.character_name, &mut trace).await;
    assert!(trace.gains.is_empty());
    drop(session);
    let mut restored = login_to_world_session(&config).await.unwrap();
    assert_eq!(restored.quests.get(&1009), Some(&1));
    send(&mut restored.connection, talk()).await;
    assert_eq!(prompt(&mut restored.connection, &mut trace).await.kind, 4);
    send(&mut restored.connection, response(4, -1, None)).await;
    barrier(&mut restored.connection, &config.character_name, &mut trace).await;
    assert!(trace.gains.is_empty());
    let isolated: WorldSession = login_to_world_session(&other).await.unwrap();
    assert!(isolated.quests.is_empty());
    assert_eq!(isolated.exp, 0);
}

#[tokio::test]
async fn started_quiz_survives_channel_migration() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config).await.unwrap();
    let mut trace = Trace::default();
    send(&mut session.connection, talk()).await;
    question(&mut session.connection, &mut trace).await;
    assert!(trace.progress.contains(&(1009, 1)));
    send(&mut session.connection, build_change_channel(1).unwrap()).await;
    let redirect = loop {
        let packet = read(&mut session.connection).await;
        if packet.opcode() == SendOpcode::ChangeChannel as i16 {
            break decode_channel_change(&packet).unwrap();
        }
    };
    assert_eq!(redirect.port, config.world_addr.port());
    let mut connection = MapleTestConnection::connect(config.world_addr, "quest channel migration")
        .await
        .unwrap();
    send(
        &mut connection,
        build_player_logged_in_for_channel(session.character_id, 1).unwrap(),
    )
    .await;
    let snapshot = loop {
        let packet = read(&mut connection).await;
        if packet.opcode() == SendOpcode::SetField as i16 {
            break decode_set_field(&packet).unwrap();
        }
    };
    assert_eq!(snapshot.quests.get(&1009), Some(&1));
    assert_eq!(snapshot.exp, 0);
    send(&mut connection, talk()).await;
    assert_eq!(prompt(&mut connection, &mut trace).await.kind, 4);
    assert!(trace.gains.is_empty());
}
