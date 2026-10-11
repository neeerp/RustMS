use game_data::GameData;
use integration_harness::{
    connection::MapleTestConnection,
    login_to_world_session,
    packets::{build_all_chat, build_change_map},
    preconditions::load_harness_config_or_fail,
};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::{
    io::{read::PktRead, write::PktWrite},
    Packet,
};
use std::{collections::BTreeMap, io::Cursor, path::Path};
use tokio::time::{timeout, Duration};

#[derive(Default)]
struct Trace {
    states: BTreeMap<i16, u8>,
    gains: Vec<i32>,
}
struct Prompt {
    kind: u8,
    text: String,
    next: bool,
}
fn response(kind: u8, value: i8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::NpcTalkMore as i16).unwrap();
    p.write_byte(kind).unwrap();
    p.write_byte(value as u8).unwrap();
    if kind == 4 && value == 1 {
        p.write_int(0).unwrap();
    }
    p
}
async fn send(c: &mut MapleTestConnection, p: Packet) {
    c.send_packet(p, "beginner contact quest").await.unwrap();
}
async fn read(c: &mut MapleTestConnection, trace: &mut Trace) -> Packet {
    let p = timeout(
        Duration::from_secs(5),
        c.read_packet("beginner contact quest"),
    )
    .await
    .unwrap()
    .unwrap()
    .packet;
    if p.opcode() == SendOpcode::ShowStatusInfo as i16 {
        let mut r = Cursor::new(&p.bytes);
        r.read_short().unwrap();
        match r.read_byte().unwrap() {
            1 => {
                trace
                    .states
                    .insert(r.read_short().unwrap(), r.read_byte().unwrap());
            }
            3 => {
                r.read_byte().unwrap();
                trace.gains.push(r.read_int().unwrap());
            }
            other => panic!("unexpected status {other}"),
        }
    }
    p
}
async fn prompt(c: &mut MapleTestConnection, npc: i32, trace: &mut Trace) -> Prompt {
    loop {
        let p = read(c, trace).await;
        if p.opcode() != SendOpcode::NpcDialogue as i16 {
            continue;
        }
        let mut r = Cursor::new(&p.bytes);
        r.read_short().unwrap();
        r.read_byte().unwrap();
        assert_eq!(r.read_int().unwrap(), npc);
        let kind = r.read_byte().unwrap();
        r.read_byte().unwrap();
        let text = r.read_str_with_length().unwrap();
        let next = if kind == 0 {
            r.read_byte().unwrap();
            r.read_byte().unwrap() != 0
        } else {
            false
        };
        return Prompt { kind, text, next };
    }
}
async fn talk(c: &mut MapleTestConnection, maps: &GameData, map: i32, npc: i32) {
    let index = maps
        .field(map)
        .unwrap()
        .map_npcs
        .iter()
        .position(|n| n.npc_id == npc)
        .unwrap();
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(1_000_000_000 + index as i32).unwrap();
    send(c, p).await;
}
async fn offer(c: &mut MapleTestConnection, npc: i32, trace: &mut Trace) -> Prompt {
    for _ in 0..10 {
        let p = prompt(c, npc, trace).await;
        if p.kind == 1 {
            return p;
        }
        assert_eq!(p.kind, 0);
        assert!(p.next);
        send(c, response(0, 1)).await;
    }
    panic!("did not reach offer");
}
async fn finish(c: &mut MapleTestConnection, npc: i32, trace: &mut Trace) {
    loop {
        let p = prompt(c, npc, trace).await;
        assert_eq!(p.kind, 0);
        send(c, response(0, 1)).await;
        if !p.next {
            return;
        }
    }
}
async fn barrier(c: &mut MapleTestConnection, text: &str, trace: &mut Trace) {
    send(c, build_all_chat(text, 0).unwrap()).await;
    loop {
        let p = read(c, trace).await;
        assert_ne!(
            p.opcode(),
            SendOpcode::NpcDialogue as i16,
            "stale response produced dialogue"
        );
        if p.opcode() == SendOpcode::ChatText as i16 {
            let mut r = Cursor::new(&p.bytes);
            r.read_short().unwrap();
            r.read_int().unwrap();
            r.read_byte().unwrap();
            if r.read_str_with_length().unwrap() == text {
                return;
            }
        }
    }
}
async fn warp(c: &mut MapleTestConnection, map: i32, portal: &str, trace: &mut Trace) -> i32 {
    send(c, build_change_map(map, portal).unwrap()).await;
    loop {
        let p = read(c, trace).await;
        if p.opcode() == SendOpcode::SetField as i16 {
            return integration_harness::packets::decode_set_field_warp(&p)
                .unwrap()
                .map_id;
        }
    }
}

#[tokio::test]
async fn existing_beginner_visits_follow_dialogue_portals_and_persist_rewards_once() {
    let config = load_harness_config_or_fail().await;
    let maps = GameData::load_from_nx_map(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx"),
    )
    .unwrap();
    let mut session = login_to_world_session(&config).await.unwrap();
    let mut trace = Trace::default();
    let c = &mut session.connection;
    assert_eq!(session.map_id, 10_000);
    talk(c, &maps, 10_000, 2101).await;
    assert!(offer(c, 2101, &mut trace).await.text.contains("Sera"));
    send(c, response(1, 0)).await;
    finish(c, 2101, &mut trace).await;
    assert!(trace.states.is_empty());
    talk(c, &maps, 10_000, 2101).await;
    offer(c, 2101, &mut trace).await;
    // Wrong protocol type must not accept the quest.
    send(c, response(4, 1)).await;
    barrier(c, &config.character_name, &mut trace).await;
    assert!(trace.states.is_empty());
    talk(c, &maps, 10_000, 2101).await;
    offer(c, 2101, &mut trace).await;
    send(c, response(1, 1)).await;
    finish(c, 2101, &mut trace).await;
    assert_eq!(trace.states.get(&1031), Some(&1));
    assert!(trace.gains.is_empty());
    // The original NPC gives directions rather than completing its own quest.
    talk(c, &maps, 10_000, 2101).await;
    assert!(prompt(c, 2101, &mut trace).await.text.contains("Sera"));
    send(c, response(0, -1)).await;
    talk(c, &maps, 10_000, 2100).await;
    finish(c, 2100, &mut trace).await;
    assert_eq!(trace.states.get(&1031), Some(&2));
    assert_eq!(trace.gains, [5]);
    send(c, response(0, 1)).await;
    barrier(c, &config.character_name, &mut trace).await;
    assert_eq!(trace.gains, [5]);
    assert_eq!(warp(c, -1, "out00", &mut trace).await, 20_000);
    assert_eq!(warp(c, -1, "out00", &mut trace).await, 30_000);
    talk(c, &maps, 30_000, 2102).await;
    offer(c, 2102, &mut trace).await;
    send(c, response(1, 1)).await;
    finish(c, 2102, &mut trace).await;
    assert_eq!(trace.states.get(&1032), Some(&1));
    drop(session);
    let mut session = login_to_world_session(&config).await.unwrap();
    assert_eq!(session.map_id, 30_000);
    assert_eq!(session.exp, 5);
    assert_eq!(session.quests.get(&1032), Some(&1));
    let c = &mut session.connection;
    assert_eq!(warp(c, -1, "in01", &mut trace).await, 30_001);
    talk(c, &maps, 30_001, 2001).await;
    // Act/1032/1/nextQuest starts Sen's existing offer, not a fabricated
    // completion page. Cancelling it still preserves the completed first leg.
    assert!(offer(c, 2001, &mut trace)
        .await
        .text
        .contains("Mushroom Candy"));
    assert_eq!(trace.gains, [5, 7]);
    assert_eq!(trace.states.get(&1032), Some(&2));
    send(c, response(1, -1)).await;
    barrier(c, &config.character_name, &mut trace).await;
    assert!(!trace.states.contains_key(&1033));
    drop(session);
    let mut session = login_to_world_session(&config).await.unwrap();
    assert_eq!(session.exp, 12);
    assert_eq!(session.quests.get(&1032), Some(&2));
    assert!(!session.quests.contains_key(&1033));
    let c = &mut session.connection;
    talk(c, &maps, 30_001, 2001).await;
    offer(c, 2001, &mut trace).await;
    send(c, response(1, 1)).await;
    finish(c, 2001, &mut trace).await;
    assert_eq!(trace.states.get(&1033), Some(&1));
    assert_eq!(trace.gains, [5, 7]);
    assert_eq!(warp(c, -1, "out00", &mut trace).await, 30_000);
    talk(c, &maps, 30_000, 2102).await;
    // No completion speech exists for 1033; normal asset-backed Nina speech
    // follows the turn-in. Unsupported quest 1034 must not be started.
    let p = prompt(c, 2102, &mut trace).await;
    send(c, response(p.kind, -1)).await;
    barrier(c, &config.character_name, &mut trace).await;
    assert_eq!(trace.gains, [5, 7, 7]);
    assert_eq!(trace.states.get(&1033), Some(&2));
    assert!(!trace.states.contains_key(&1034));
    send(c, response(0, 1)).await;
    barrier(c, &config.character_name, &mut trace).await;
    assert_eq!(trace.gains, [5, 7, 7]);
    drop(session);
    let restored = login_to_world_session(&config).await.unwrap();
    assert_eq!((restored.level, restored.exp, restored.ap), (2, 4, 5));
    for id in [1031, 1032, 1033] {
        assert_eq!(restored.quests.get(&id), Some(&2));
    }
    assert_eq!(restored.quests.len(), 3);
}
