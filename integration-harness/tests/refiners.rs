//! The refiners ask how many: Mr. Thunder refines five batches of Steel
//! Ore at once, and ends the talk on a number out of range; Perzen in
//! Altaire Camp trades each hundred Rubble for a Perfect Pitch.

use game_data::GameData;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_stack, set_meso};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{barrier, stat, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use std::sync::OnceLock;
use tokio::time::Duration;

const PERION: i32 = 102000000;
const MR_THUNDER: i32 = 1022003;
const ALTAIRE_CAMP: i32 = 300000000;
const PERZEN: i32 = 2131001;
const STEEL_ORE: i32 = 4010001;
const STEEL_PLATE: i32 = 4011001;
const RUBBLE: i32 = 4000439;
const PERFECT_PITCH: i32 = 4310000;
const ETC: i16 = 4;

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

/// Ask Mr. Thunder for Steel Plates, up to the quantity prompt.
async fn ask_for_steel_plates(c: &mut MapleTestConnection, m: &str) {
    let t = step(c, m, talk(PERION, MR_THUNDER)).await;
    assert_eq!(t.prompts, [(MR_THUNDER, 4)]);
    let t = step(c, m, answer(4, 1, Some(0))).await;
    assert_eq!(t.prompts, [(MR_THUNDER, 4)]);
    let t = step(c, m, answer(4, 1, Some(1))).await;
    assert_eq!(t.prompts, [(MR_THUNDER, 3)]);
}

#[tokio::test]
async fn refiners_make_as_many_as_asked() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, ETC, 1, STEEL_ORE, 60).await.unwrap();
    grant_stack(id, ETC, 2, RUBBLE, 200).await.unwrap();
    grant_stack(id, ETC, 3, RUBBLE, 50).await.unwrap();
    set_meso(id, 10_000).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, PERION, "", &mut trace).await, PERION);
    barrier(c, &m, &mut trace).await;

    // Five batches: 50 ore and 1,500 mesos for five plates.
    ask_for_steel_plates(c, &m).await;
    let t = step(c, &m, answer(3, 1, Some(5))).await;
    assert_eq!(t.prompts, [(MR_THUNDER, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.items, [(STEEL_PLATE, 5), (STEEL_ORE, -50)]);
    assert_eq!(t.stat(stat::MESO), Some(8_500));
    assert_eq!(t.prompts, [(MR_THUNDER, 0)]);
    step(c, &m, answer(0, 1, None)).await;

    // More than a hundred ends the talk, nothing made.
    ask_for_steel_plates(c, &m).await;
    let t = step(c, &m, answer(3, 1, Some(101))).await;
    assert!(t.prompts.is_empty() && t.items.is_empty());
    // One more batch than the ore left: a word, nothing taken.
    ask_for_steel_plates(c, &m).await;
    step(c, &m, answer(3, 1, Some(2))).await;
    let t = step(c, &m, answer(1, 1, None)).await;
    assert!(t.items.is_empty());
    assert_eq!(t.prompts, [(MR_THUNDER, 0)]);
    step(c, &m, answer(0, 1, None)).await;

    // Perzen offers as many Perfect Pitches as the Rubble buys.
    assert_eq!(warp(c, ALTAIRE_CAMP, "", &mut trace).await, ALTAIRE_CAMP);
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, talk(ALTAIRE_CAMP, PERZEN)).await;
    assert_eq!(t.prompts, [(PERZEN, 4)]);
    let t = step(c, &m, answer(4, 1, Some(0))).await;
    assert_eq!(t.prompts, [(PERZEN, 3)]);
    let t = step(c, &m, answer(3, 1, Some(2))).await;
    assert_eq!(t.items, [(RUBBLE, -200), (PERFECT_PITCH, 2)]);
    assert_eq!(t.prompts, [(PERZEN, 0)]);
}
