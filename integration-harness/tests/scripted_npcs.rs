//! Roger's Apple (quest 1021, scripted), using items, natural recovery,
//! Robin's Q&A, and the Maple Administrator's tutorial skip.

use game_data::GameData;
use integration_harness::connection::MapleTestConnection;
use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{build_quest_action, read, send, warp, QuestTrace};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::{read::PktRead, write::PktWrite};
use packet::Packet;
use std::io::Cursor;
use std::path::Path;
use std::sync::OnceLock;
use tokio::time::{timeout, Duration};

const SCRIPT_START: u8 = 4;
const SCRIPT_END: u8 = 5;
const ROGER: i32 = 2000;
const ROGERS_APPLE: i32 = 2010007;

fn maps() -> &'static GameData {
    static MAPS: OnceLock<GameData> = OnceLock::new();
    MAPS.get_or_init(|| {
        GameData::load_from_nx_map(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx"),
        )
        .unwrap()
    })
}

fn talk(map: i32, npc: i32) -> Packet {
    let index = maps()
        .field(map)
        .unwrap()
        .map_npcs
        .iter()
        .position(|n| n.npc_id == npc)
        .unwrap();
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(1_000_000_000 + index as i32).unwrap();
    p
}

fn answer(kind: u8, response: i8, selection: Option<i32>) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::NpcTalkMore as i16).unwrap();
    p.write_byte(kind).unwrap();
    p.write_byte(response as u8).unwrap();
    if let Some(selection) = selection {
        p.write_int(selection).unwrap();
    }
    p
}

/// Everything interesting the server sent while a test waited.
#[derive(Debug, Default)]
struct Seen {
    /// NPC dialogue prompts: (npc, message type).
    prompts: Vec<(i32, u8)>,
    /// Items added by INVENTORY_OPERATION: (type, slot, item id).
    added: Vec<(u8, i16, i32)>,
    /// HP from stat updates.
    hp: Vec<i16>,
    images: Vec<String>,
    warped_to: Option<i32>,
}

fn observe(p: &Packet, seen: &mut Seen) {
    let mut r = Cursor::new(&p.bytes);
    r.read_short().unwrap();
    let op = p.opcode();
    if op == SendOpcode::NpcDialogue as i16 {
        r.read_byte().unwrap();
        let npc = r.read_int().unwrap();
        seen.prompts.push((npc, r.read_byte().unwrap()));
    } else if op == SendOpcode::InventoryOperation as i16 {
        r.read_byte().unwrap();
        for _ in 0..r.read_byte().unwrap() {
            let mode = r.read_byte().unwrap();
            let kind = r.read_byte().unwrap();
            let slot = r.read_short().unwrap();
            if mode == 0 {
                r.read_byte().unwrap();
                seen.added.push((kind, slot, r.read_int().unwrap()));
                return; // the rest of the item is not needed
            } else if mode == 1 || mode == 2 {
                r.read_short().unwrap();
            }
        }
    } else if op == SendOpcode::StatChange as i16 {
        r.read_byte().unwrap();
        let mask = r.read_int().unwrap();
        // HP is the only stat these tests read; it is first when present.
        if mask & 0x400 != 0 && mask & !0x400 & 0x3FF == 0 {
            seen.hp.push(r.read_short().unwrap());
        }
    } else if op == SendOpcode::ShowItemGainInChat as i16 && r.read_byte().unwrap() == 0x17 {
        seen.images.push(r.read_str_with_length().unwrap());
    } else if op == SendOpcode::SetField as i16 {
        seen.warped_to = Some(
            integration_harness::packets::decode_set_field_warp(p)
                .unwrap()
                .map_id,
        );
    }
}

/// Read until a chat barrier, collecting what the server sent.
async fn settle(c: &mut MapleTestConnection, marker: &str, seen: &mut Seen) {
    send(
        c,
        integration_harness::packets::build_all_chat(marker, 0).unwrap(),
    )
    .await;
    let mut trace = QuestTrace::default();
    loop {
        let p = timeout(Duration::from_secs(5), read(c, &mut trace))
            .await
            .expect("timeout");
        observe(&p, seen);
        if p.opcode() == SendOpcode::ChatText as i16 {
            let mut r = Cursor::new(&p.bytes);
            r.read_short().unwrap();
            r.read_int().unwrap();
            r.read_byte().unwrap();
            if r.read_str_with_length().unwrap() == marker {
                return;
            }
        }
    }
}

async fn step(c: &mut MapleTestConnection, m: &str, packet: Packet) -> (Seen, QuestTrace) {
    send(c, packet).await;
    let mut seen = Seen::default();
    let mut trace = QuestTrace::default();
    send(
        c,
        integration_harness::packets::build_all_chat(m, 0).unwrap(),
    )
    .await;
    loop {
        let p = timeout(Duration::from_secs(5), read(c, &mut trace))
            .await
            .expect("timeout");
        observe(&p, &mut seen);
        if p.opcode() == SendOpcode::ChatText as i16 {
            let mut r = Cursor::new(&p.bytes);
            r.read_short().unwrap();
            r.read_int().unwrap();
            r.read_byte().unwrap();
            if r.read_str_with_length().unwrap() == m {
                return (seen, trace);
            }
        }
    }
}

fn use_item(slot: i16, item_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseItem as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(item_id).unwrap();
    p
}

fn heal(hp: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::HealOverTime as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_int(0).unwrap();
    p.write_short(hp).unwrap();
    p.write_short(0).unwrap();
    p
}

#[tokio::test]
async fn rogers_apple_teaches_using_items_and_pays_once() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    let mut trace = QuestTrace::default();
    assert_eq!(warp(c, -1, "out00", &mut trace).await, 20_000);
    settle(c, &m, &mut Seen::default()).await;

    // Plain QUEST_ACTION start is refused for a scripted quest.
    let (_, t) = step(c, &m, build_quest_action(1, 1021, ROGER)).await;
    assert!(t.progress.is_empty());

    // q1021s: two pages, then yes/no.
    let (seen, _) = step(c, &m, build_quest_action(SCRIPT_START, 1021, ROGER)).await;
    assert_eq!(seen.prompts, [(ROGER, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    let (seen, _) = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(seen.prompts, [(ROGER, 1)]);
    let (seen, t) = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.progress, [(1021, 1)]);
    assert_eq!(seen.hp, [25]);
    let &(kind, slot, item) = seen.added.first().expect("Roger's apple");
    assert_eq!((kind, item), (2, ROGERS_APPLE));
    step(c, &m, answer(0, 1, None)).await;
    let (seen, _) = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(seen.images, ["UI/tutorial.img/28"]);

    // Can't finish while holding the apple.
    let (seen, t) = step(c, &m, build_quest_action(SCRIPT_END, 1021, ROGER)).await;
    assert!(seen.prompts.is_empty() && t.progress.is_empty());

    // Eat it: +30 HP, consumed.
    let (seen, _) = step(c, &m, use_item(slot, ROGERS_APPLE)).await;
    assert_eq!(seen.hp, [50]);
    let (seen, _) = step(c, &m, use_item(slot, ROGERS_APPLE)).await;
    assert!(seen.hp.is_empty(), "the apple is gone");

    // q1021e: three pages, then the rewards and completion.
    step(c, &m, build_quest_action(SCRIPT_END, 1021, ROGER)).await;
    step(c, &m, answer(0, 1, None)).await;
    step(c, &m, answer(0, 1, None)).await;
    let (seen, t) = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.progress, [(1021, 2)]);
    assert_eq!(t.gains, [10]);
    let rewards: Vec<i32> = seen.added.iter().map(|a| a.2).collect();
    assert!(
        rewards.contains(&2010000) || seen.added.len() == 1,
        "apples were given: {rewards:?}"
    );

    // Replays do nothing.
    let (seen, t) = step(c, &m, build_quest_action(SCRIPT_END, 1021, ROGER)).await;
    assert!(seen.prompts.is_empty() && t.gains.is_empty());
    drop(session);

    let restored = integration_harness::relogin(&config).await;
    assert_eq!(restored.quests.get(&1021), Some(&2));
    assert_eq!(restored.exp, 10);
}

#[tokio::test]
async fn natural_recovery_is_rate_limited() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    let mut trace = QuestTrace::default();
    assert_eq!(warp(c, -1, "out00", &mut trace).await, 20_000);
    settle(c, &m, &mut Seen::default()).await;
    step(c, &m, build_quest_action(SCRIPT_START, 1021, ROGER)).await;
    step(c, &m, answer(0, 1, None)).await;
    step(c, &m, answer(0, 1, None)).await;
    step(c, &m, answer(1, 1, None)).await; // HP 25
    let (seen, _) = step(c, &m, heal(10)).await;
    assert_eq!(seen.hp, [35]);
    let (seen, _) = step(c, &m, heal(10)).await;
    assert!(seen.hp.is_empty(), "too soon");
    let (seen, _) = step(c, &m, heal(500)).await;
    assert!(seen.hp.is_empty(), "too much");
}

#[tokio::test]
async fn robin_answers_and_the_administrator_offers_a_skip() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    let mut trace = QuestTrace::default();

    // Maple Administrator in Mushroom Town: decline, then accept.
    let (seen, _) = step(c, &m, talk(10_000, 2007)).await;
    assert_eq!(seen.prompts, [(2007, 1)]);
    let (seen, _) = step(c, &m, answer(1, 0, None)).await;
    assert_eq!(seen.prompts, [(2007, 0)]);
    step(c, &m, answer(0, 1, None)).await;

    assert_eq!(warp(c, 50_000, "", &mut trace).await, 50_000);
    settle(c, &m, &mut Seen::default()).await;
    let (seen, _) = step(c, &m, talk(50_000, 2003)).await;
    assert_eq!(seen.prompts, [(2003, 4)]);
    // "Tell me more about this island!" has four pages.
    let (seen, _) = step(c, &m, answer(4, 1, Some(5))).await;
    assert_eq!(seen.prompts, [(2003, 0)]);
    for _ in 0..3 {
        let (seen, _) = step(c, &m, answer(0, 1, None)).await;
        assert_eq!(seen.prompts, [(2003, 0)]);
    }
    let (seen, _) = step(c, &m, answer(0, 1, None)).await;
    assert!(seen.prompts.is_empty(), "the conversation ends");

    assert_eq!(warp(c, 10_000, "", &mut trace).await, 10_000);
    settle(c, &m, &mut Seen::default()).await;
    step(c, &m, talk(10_000, 2007)).await;
    let (seen, _) = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(seen.warped_to, Some(104_000_000));
}
