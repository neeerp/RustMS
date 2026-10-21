//! Jeff, in El Nath's Ice Valley II, lets players from level 50 down to
//! Sharp Cliff I, the top of the dungeon below, and sends an Orihalcon
//! Hammer's holder to Ice Valley without a word.

use game_data::GameData;
use integration_harness::database::{grant_stack, set_job};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{barrier, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use std::sync::OnceLock;
use tokio::time::Duration;

const ICE_VALLEY: i32 = 211040200;
const SHARP_CLIFF: i32 = 211040300;
const JEFF: i32 = 2030000;
const HAMMERS_ICE_VALLEY: i32 = 921100100;
const ORIHALCON_HAMMER: i32 = 4031450;

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
async fn jeff_lets_players_from_level_50_down_to_sharp_cliff() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, ICE_VALLEY, "", &mut trace).await, ICE_VALLEY);
    barrier(c, &m, &mut trace).await;

    // At level 1 he says why not, and nobody moves.
    let t = step(c, &m, talk(ICE_VALLEY, JEFF)).await;
    assert_eq!(t.prompts, [(JEFF, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(JEFF, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.prompts.is_empty() && t.warps.is_empty());

    // At level 50 he asks, and yes goes down to Sharp Cliff's `under00`.
    let id = session.character_id;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 50, 50).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, ICE_VALLEY, "", &mut trace).await, ICE_VALLEY);
    barrier(c, &m, &mut trace).await;
    step(c, &m, talk(ICE_VALLEY, JEFF)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(JEFF, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.arrivals, [(SHARP_CLIFF, 5, None)]);

    // With the hammer, straight to Ice Valley's `west00`.
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, 4, 1, ORIHALCON_HAMMER, 1).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, ICE_VALLEY, "", &mut trace).await, ICE_VALLEY);
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, talk(ICE_VALLEY, JEFF)).await;
    assert!(t.prompts.is_empty(), "{:?}", t.prompts);
    assert_eq!(t.arrivals, [(HAMMERS_ICE_VALLEY, 1, None)]);
}
