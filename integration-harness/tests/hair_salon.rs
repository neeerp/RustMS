//! Henesys's stylists. At Natalie's hair salon (100000104) a haircut coupon
//! buys one of her styles, picked on a preview of the player (NPC dialogue
//! type 7); the coupon goes, others see the new hair, and it is saved. Dr.
//! Feeble's face is rolled after a yes, and Ms. Tan's skins are picked.

use game_data::GameData;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::grant_item;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{read, send, stat, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::{read::PktRead, write::PktWrite};
use packet::Packet;
use std::io::Cursor;
use std::path::Path;
use tokio::time::{timeout, Duration};

const SALON: i32 = 100_000_104;
const NATALIE: i32 = 1_012_103;
const HAIRCUT_COUPON: i32 = 5_150_001;
const SURGERY: i32 = 100_000_103;
const DR_FEEBLE: i32 = 1_052_005;
const FACE_COUPON: i32 = 5_152_000;
const SKIN_CARE: i32 = 100_000_105;
const MS_TAN: i32 = 1_012_105;
const SKIN_COUPON: i32 = 5_153_000;
const CASH: i16 = 5;

fn talk(map: i32, npc: i32) -> Packet {
    let maps = GameData::load_from_nx_map(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx"),
    )
    .unwrap();
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
    p
}

/// The answer to a choice of styles: the index of the one picked.
fn pick(index: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::NpcTalkMore as i16).unwrap();
    p.write_byte(7).unwrap();
    p.write_byte(1).unwrap();
    p.write_byte(index).unwrap();
    p
}

/// The next NPC prompt: its type, and the styles of a choice of styles.
async fn prompt(c: &mut MapleTestConnection, t: &mut Trace, npc: i32) -> (u8, Vec<i32>) {
    loop {
        let p = read(c, t).await;
        if p.opcode() != SendOpcode::NpcDialogue as i16 {
            continue;
        }
        let mut r = Cursor::new(&p.bytes);
        r.read_short().unwrap();
        r.read_byte().unwrap();
        assert_eq!(r.read_int().unwrap(), npc);
        let kind = r.read_byte().unwrap();
        r.read_byte().unwrap();
        r.read_str_with_length().unwrap();
        let mut styles = Vec::new();
        if kind == 7 {
            for _ in 0..r.read_byte().unwrap() {
                styles.push(r.read_int().unwrap());
            }
        }
        return (kind, styles);
    }
}

/// Whose look the next UPDATE_CHAR_LOOK an observer sees changes.
async fn look_update(c: &mut MapleTestConnection) -> i32 {
    loop {
        let p = timeout(Duration::from_secs(5), c.read_packet("look update"))
            .await
            .expect("no look update")
            .unwrap()
            .packet;
        if p.opcode() == SendOpcode::UpdateCharLook as i16 {
            let mut r = Cursor::new(&p.bytes);
            r.read_short().unwrap();
            return r.read_int().unwrap();
        }
    }
}

#[tokio::test]
async fn a_haircut_coupon_buys_a_new_hairstyle() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_item(id, CASH, 1, HAIRCUT_COUPON)
        .await
        .expect("give a coupon");
    let mut session = relogin(&config).await;
    let mut observer = login_to_world_session(&load_harness_config_or_fail().await)
        .await
        .unwrap();
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut observer.connection, SALON, "", &mut seen).await,
        SALON
    );
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, SALON, "", &mut t).await, SALON);

    send(c, talk(SALON, NATALIE)).await;
    assert_eq!(prompt(c, &mut t, NATALIE).await.0, 4);
    send(c, answer(4, 1, Some(0))).await;
    let (kind, styles) = prompt(c, &mut t, NATALIE).await;
    assert_eq!(kind, 7);
    assert_eq!(styles.len(), 7, "{styles:?}");
    assert!(styles.iter().all(|s| s % 10 == session.hair % 10));
    send(c, pick(1)).await;
    assert_eq!(prompt(c, &mut t, NATALIE).await.0, 0);
    until(c, &mut t, |t| !t.stat_values(stat::HAIR).is_empty()).await;
    assert_eq!(t.stat_values(stat::HAIR), [i64::from(styles[1])]);
    assert_eq!(t.removed, [(CASH as u8, 1)]);
    assert_eq!(look_update(&mut observer.connection).await, id);
    drop(observer);
    drop(session);

    tokio::time::sleep(Duration::from_millis(500)).await;
    let back = relogin(&config).await;
    assert_eq!(back.hair, styles[1]);
    assert!(!back.items.contains_key(&HAIRCUT_COUPON));
}

#[tokio::test]
async fn a_face_is_rolled_and_a_skin_picked_for_coupons() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_item(id, CASH, 1, FACE_COUPON)
        .await
        .expect("give a face coupon");
    grant_item(id, CASH, 2, SKIN_COUPON)
        .await
        .expect("give a skin coupon");
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();

    assert_eq!(warp(c, SURGERY, "", &mut t).await, SURGERY);
    send(c, talk(SURGERY, DR_FEEBLE)).await;
    assert_eq!(prompt(c, &mut t, DR_FEEBLE).await.0, 4);
    send(c, answer(4, 1, Some(0))).await;
    assert_eq!(prompt(c, &mut t, DR_FEEBLE).await.0, 1);
    send(c, answer(1, 1, None)).await;
    assert_eq!(prompt(c, &mut t, DR_FEEBLE).await.0, 0);
    until(c, &mut t, |t| !t.stat_values(stat::FACE).is_empty()).await;
    let face = t.stat_values(stat::FACE)[0];
    assert_eq!(face / 1_000, 20, "a man's face, {face}");
    assert_eq!(t.removed, [(CASH as u8, 1)]);
    send(c, answer(0, 1, None)).await;

    let mut t = Trace::default();
    assert_eq!(warp(c, SKIN_CARE, "", &mut t).await, SKIN_CARE);
    send(c, talk(SKIN_CARE, MS_TAN)).await;
    assert_eq!(prompt(c, &mut t, MS_TAN).await.0, 4);
    send(c, answer(4, 1, Some(0))).await;
    let (kind, skins) = prompt(c, &mut t, MS_TAN).await;
    assert_eq!((kind, skins), (7, vec![0, 1, 2, 3, 4]));
    send(c, pick(3)).await;
    assert_eq!(prompt(c, &mut t, MS_TAN).await.0, 0);
    until(c, &mut t, |t| !t.stat_values(stat::SKIN).is_empty()).await;
    assert_eq!(t.stat_values(stat::SKIN), [3]);
    assert_eq!(t.removed, [(CASH as u8, 2)]);
    drop(session);

    tokio::time::sleep(Duration::from_millis(500)).await;
    let back = relogin(&config).await;
    assert!(!back.items.contains_key(&FACE_COUPON));
    assert!(!back.items.contains_key(&SKIN_COUPON));
}
