//! Rides with no timetable: the ferries' pilots and the crane sell a ride,
//! the ride map counts down, and the player arrives at the other end. The
//! stack runs the route clock 60 times as fast (`RUSTMS_CONTI_SCALE`), so
//! a minute's ride takes a second.

use game_data::GameData;
use integration_harness::database::{set_job, set_meso};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{stat, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use std::sync::OnceLock;
use tokio::time::Duration;

const LITH_HARBOR: i32 = 104_000_000;
const RIEN_DOCK: i32 = 140_020_300;
const LITH_TO_RIEN: i32 = 200_090_060;
const RIEN_TO_LITH: i32 = 200_090_070;
const ORBIS_FERRY_PLATFORM: i32 = 200_000_161;
const ORBIS_TO_EREVE: i32 = 200_090_020;
const CRANE_PLATFORM: i32 = 200_000_141;
const ORBIS_TO_MU_LUNG: i32 = 200_090_300;
const MU_LUNG_TEMPLE: i32 = 250_000_100;
const HERB_TOWN: i32 = 251_000_000;
const PURO_IN_LITH: i32 = 1200004;
const PURO_IN_RIEN: i32 = 1200003;
const KIRU_IN_ORBIS: i32 = 1100008;
const CRANE: i32 = 2090005;

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

/// Log the character behind `config` in with `meso` mesos at `map`.
async fn traveller(config: &HarnessConfig, meso: i32, map: i32) -> WorldSession {
    let id = login_to_world_session(config)
        .await
        .expect("first login")
        .character_id;
    drop(relogin(config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 10, 50).await.expect("job");
    set_meso(id, meso).await.expect("mesos");
    let mut session = relogin(config).await;
    let mut t = Trace::default();
    assert_eq!(warp(&mut session.connection, map, "", &mut t).await, map);
    session
}

#[tokio::test]
async fn puro_ferries_a_player_to_rien_and_back_on_a_countdown() {
    let config = load_harness_config_or_fail().await;
    let mut session = traveller(&config, 1600, LITH_HARBOR).await;
    let m = session.character_name.clone();
    let c = &mut session.connection;
    for (from, puro, ride, to, meso) in [
        (LITH_HARBOR, PURO_IN_LITH, LITH_TO_RIEN, RIEN_DOCK, 800),
        (RIEN_DOCK, PURO_IN_RIEN, RIEN_TO_LITH, LITH_HARBOR, 0),
    ] {
        let t = step(c, &format!("{m} talk"), talk(from, puro)).await;
        assert_eq!(t.prompts, [(puro, 4)]);
        assert!(t.prompt_texts[0].contains(&format!("#m{to}#")));
        let t = step(c, &format!("{m} pick"), answer(4, 1, Some(0))).await;
        assert_eq!(t.prompts, [(puro, 1)]);
        let mut t = step(c, &format!("{m} yes"), answer(1, 1, None)).await;
        assert_eq!(t.stat(stat::MESO), Some(meso));
        // Aboard, the minute's ride counts down a second, then arrives.
        until(c, &mut t, |t| t.warps.contains(&to)).await;
        assert_eq!(t.warps, [ride, to]);
        assert_eq!(t.countdowns, [1]);
    }
}

#[tokio::test]
async fn the_crane_flies_to_mu_lung_and_walks_on_to_herb_town_and_back() {
    let config = load_harness_config_or_fail().await;
    let mut session = traveller(&config, 2500, CRANE_PLATFORM).await;
    let m = session.character_name.clone();
    let c = &mut session.connection;

    step(c, &format!("{m} talk"), talk(CRANE_PLATFORM, CRANE)).await;
    step(c, &format!("{m} pick"), answer(4, 1, Some(0))).await;
    let mut t = step(c, &format!("{m} yes"), answer(1, 1, None)).await;
    assert_eq!(t.stat(stat::MESO), Some(1000));
    until(c, &mut t, |t| t.warps.contains(&MU_LUNG_TEMPLE)).await;
    assert_eq!(t.warps, [ORBIS_TO_MU_LUNG, MU_LUNG_TEMPLE]);

    // On to Herb Town from the temple's menu, and back on a plain question.
    let t = step(c, &format!("{m} talk"), talk(MU_LUNG_TEMPLE, CRANE)).await;
    assert!(t.prompt_texts[0].contains(&format!("#m{HERB_TOWN}#")));
    step(c, &format!("{m} pick"), answer(4, 1, Some(1))).await;
    let t = step(c, &format!("{m} yes"), answer(1, 1, None)).await;
    assert_eq!(t.stat(stat::MESO), Some(500));
    assert_eq!(t.warps, [HERB_TOWN]);
    let t = step(c, &format!("{m} talk"), talk(HERB_TOWN, CRANE)).await;
    assert_eq!(t.prompts, [(CRANE, 1)]);
    let t = step(c, &format!("{m} yes"), answer(1, 1, None)).await;
    assert_eq!(t.stat(stat::MESO), Some(0));
    assert_eq!(t.warps, [MU_LUNG_TEMPLE]);
}

/// Log the character out mid-ride, and back in.
async fn relogin_aboard(config: &HarnessConfig, session: WorldSession) -> WorldSession {
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    relogin(config).await
}

#[tokio::test]
async fn a_rider_who_logs_out_mid_ride_is_back_on_the_dock() {
    let config = load_harness_config_or_fail().await;
    let mut session = traveller(&config, 1000, ORBIS_FERRY_PLATFORM).await;
    let m = session.character_name.clone();
    let c = &mut session.connection;
    step(
        c,
        &format!("{m} talk"),
        talk(ORBIS_FERRY_PLATFORM, KIRU_IN_ORBIS),
    )
    .await;
    step(c, &format!("{m} pick"), answer(4, 1, Some(0))).await;
    let mut t = step(c, &format!("{m} yes"), answer(1, 1, None)).await;
    until(c, &mut t, |t| !t.countdowns.is_empty()).await;
    assert_eq!(t.warps, [ORBIS_TO_EREVE]);
    // Eight minutes take eight seconds; the relogin comes well before.
    assert_eq!(t.countdowns, [8]);
    let session = relogin_aboard(&config, session).await;
    assert_eq!(session.map_id, ORBIS_FERRY_PLATFORM);
}
