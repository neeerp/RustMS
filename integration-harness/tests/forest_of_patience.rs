//! Shane, in Ellinia, lets players from level 25 into the Forest of
//! Patience for 5000 mesos, and Louis inside sees them back to town.

use game_data::GameData;
use integration_harness::database::{set_job, set_meso};
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
const STEP_1: i32 = 101000100;
const SHANE: i32 = 1032003;
const LOUIS: i32 = 1032004;

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
async fn shane_lets_players_into_the_forest_of_patience_and_louis_out() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, ELLINIA, "", &mut trace).await, ELLINIA);
    barrier(c, &m, &mut trace).await;

    // Below level 25 he only says no.
    let t = step(c, &m, talk(ELLINIA, SHANE)).await;
    assert_eq!(t.prompts, [(SHANE, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.prompts.is_empty() && t.warps.is_empty());

    let id = session.character_id;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 30, 50).await.unwrap();
    set_meso(id, 5000).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, ELLINIA, "", &mut trace).await, ELLINIA);
    barrier(c, &m, &mut trace).await;

    // At level 30, yes pays the fare and goes to the first step.
    let t = step(c, &m, talk(ELLINIA, SHANE)).await;
    assert_eq!(t.prompts, [(SHANE, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.stat(stat::MESO), Some(0));
    assert_eq!(t.arrivals, [(STEP_1, 0, None)]);

    let t = step(c, &m, talk(STEP_1, LOUIS)).await;
    assert_eq!(t.prompts, [(LOUIS, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.arrivals, [(ELLINIA, 0, None)]);
}
