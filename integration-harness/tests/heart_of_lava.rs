//! Adobis, at the Door to Zakum, sends a Lava Mug's holder to the Heart
//! of Lava without a word, approved for Zakum or not; dropped on the frame
//! there, the mug gives the book of Fire Demon.

use game_data::GameData;
use integration_harness::database::grant_item;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{barrier, read_for, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use std::sync::OnceLock;
use tokio::time::Duration;

const ETC: i16 = 4;
const DOOR_TO_ZAKUM: i32 = 211042300;
const ADOBIS: i32 = 2030008;
const HEART_OF_LAVA: i32 = 921100000;
const FRAME: i32 = 2112015;
const LAVA_MUG: i32 = 4001109;
const FIRE_DEMON_BOOK: i32 = 2280000;

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

/// Drop one of what lies in the Etc tab's `slot`.
fn throw(slot: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemMove as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(ETC as u8).unwrap();
    p.write_short(slot).unwrap();
    p.write_short(0).unwrap();
    p.write_short(1).unwrap();
    p
}

#[tokio::test]
async fn adobis_sends_a_lava_mug_to_the_heart_of_lava_and_the_frame_takes_it() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let session = login_to_world_session(&config).await.unwrap();
    let id = session.character_id;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_item(id, ETC, 1, LAVA_MUG).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, DOOR_TO_ZAKUM, "", &mut trace).await, DOOR_TO_ZAKUM);
    barrier(c, &m, &mut trace).await;

    // Unapproved, he says nothing and sends the mug to `out00`.
    let mut trace = step(c, &m, talk(DOOR_TO_ZAKUM, ADOBIS)).await;
    assert!(trace.prompts.is_empty(), "{:?}", trace.prompts);
    assert_eq!(trace.arrivals, [(HEART_OF_LAVA, 1, None)]);
    until(c, &mut trace, |t| {
        t.field.reactors.iter().any(|r| r.reactor_id == FRAME)
    })
    .await;
    let frame = *trace
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == FRAME)
        .unwrap();

    // Dropped by the frame, the mug goes, and the book comes.
    send(c, walk_to(frame.x, frame.y)).await;
    read_for(c, &mut trace, 300).await;
    send(c, throw(1)).await;
    until(c, &mut trace, |t| t.field.item_drop(LAVA_MUG).is_some()).await;
    let mug = trace.field.item_drop(LAVA_MUG).unwrap().object_id;
    until(c, &mut trace, |t| {
        t.field
            .reactors_removed
            .iter()
            .any(|&(o, _)| o == frame.object_id)
    })
    .await;
    assert_eq!(trace.field.removals(mug), [None]);
    until(c, &mut trace, |t| {
        t.field.item_drop(FIRE_DEMON_BOOK).is_some()
    })
    .await;
}
