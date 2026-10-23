//! The makers of Ellinia and Ludibrium: Francois upgrades a Bronze Pride
//! to a Steel Pride, taking the hat; Sarah upgrades a Bronze Missel with a
//! stimulator, which is taken whether the gloves come out or are lost.

use game_data::GameData;
use integration_harness::database::{grant_item, grant_stack, set_meso};
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

const ELLINIA: i32 = 101000000;
const FRANCOIS: i32 = 1032002;
const TARA_AND_SARAHS_HOUSE: i32 = 220000303;
const SARAH: i32 = 2040020;
const BRONZE_PRIDE: i32 = 1002064;
const STEEL_PRIDE: i32 = 1002065;
const BRONZE_MISSEL: i32 = 1082007;
const STEEL_MISSEL: i32 = 1082005;
const STEEL_PLATE: i32 = 4011001;
const GLOVES_STIMULATOR: i32 = 4130000;
const EQUIP: i16 = 1;
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

#[tokio::test]
async fn francois_upgrades_a_hat_and_sarah_stimulated_gloves() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_item(id, EQUIP, 1, BRONZE_PRIDE).await.unwrap();
    grant_item(id, EQUIP, 2, BRONZE_MISSEL).await.unwrap();
    grant_stack(id, ETC, 1, STEEL_PLATE, 4).await.unwrap();
    grant_item(id, ETC, 2, GLOVES_STIMULATOR).await.unwrap();
    set_meso(id, 58_000).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, ELLINIA, "", &mut trace).await, ELLINIA);
    barrier(c, &m, &mut trace).await;

    // Upgrade a hat, Steel Pride: the Bronze Pride, 3 plates and 40,000.
    let t = step(c, &m, talk(ELLINIA, FRANCOIS)).await;
    assert_eq!(t.prompts, [(FRANCOIS, 4)]);
    let t = step(c, &m, answer(4, 1, Some(2))).await;
    assert_eq!(t.prompts, [(FRANCOIS, 4)]);
    let t = step(c, &m, answer(4, 1, Some(0))).await;
    assert_eq!(t.prompts, [(FRANCOIS, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(
        t.items,
        [(BRONZE_PRIDE, -1), (STEEL_PLATE, -3), (STEEL_PRIDE, 1)]
    );
    assert_eq!(t.stat(stat::MESO), Some(18_000));
    assert_eq!(t.prompts, [(FRANCOIS, 0)]);
    step(c, &m, answer(0, 1, None)).await;

    // Sarah: a warrior's gloves upgraded with a stimulator, Steel Missel.
    assert_eq!(
        warp(c, TARA_AND_SARAHS_HOUSE, "", &mut trace).await,
        TARA_AND_SARAHS_HOUSE
    );
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, talk(TARA_AND_SARAHS_HOUSE, SARAH)).await;
    assert_eq!(t.prompts, [(SARAH, 4)]);
    let t = step(c, &m, answer(4, 1, Some(5))).await;
    assert_eq!(t.prompts, [(SARAH, 4)]);
    let t = step(c, &m, answer(4, 1, Some(0))).await;
    assert_eq!(t.prompts, [(SARAH, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.stat(stat::MESO), Some(0));
    assert_eq!(t.prompts, [(SARAH, 0)]);
    let taken = [
        (BRONZE_MISSEL, -1),
        (STEEL_PLATE, -1),
        (GLOVES_STIMULATOR, -1),
    ];
    assert_eq!(t.items[..3], taken);
    // One in ten is lost.
    assert!(t.items[3..].is_empty() || t.items[3..] == [(STEEL_MISSEL, 1)]);
}
