//! Victoria Island's cabs: a Regular Cab takes a beginner to another town
//! for a tenth of the fare, and the VIP Cab turns away a player short of
//! its fare before taking them to Ant Tunnel Park.

use game_data::GameData;
use integration_harness::database::{set_job, set_meso};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{stat, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use std::sync::OnceLock;
use tokio::time::Duration;

const PERION: i32 = 102_000_000;
const ELLINIA: i32 = 101_000_000;
const PERION_CAB: i32 = 1022001;
const ELLINIA_VIP_CAB: i32 = 1032005;
const ANT_TUNNEL_PARK: i32 = 105_070_001;

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

/// Log the character behind `config` in as job `job` with `meso` mesos, at
/// `map`.
async fn rider(config: &HarnessConfig, job: i16, meso: i32, map: i32) -> WorldSession {
    let id = login_to_world_session(config)
        .await
        .expect("first login")
        .character_id;
    drop(relogin(config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, job, 10, 50).await.expect("job");
    set_meso(id, meso).await.expect("mesos");
    let mut session = relogin(config).await;
    let mut t = Trace::default();
    assert_eq!(warp(&mut session.connection, map, "", &mut t).await, map);
    session
}

#[tokio::test]
async fn a_regular_cab_takes_a_beginner_for_a_tenth_of_the_fare() {
    let config = load_harness_config_or_fail().await;
    let mut session = rider(&config, 0, 1000, PERION).await;
    // Barrier lines carry the speaker's name, since everyone on the map
    // hears them.
    let m = session.character_name.clone();
    let c = &mut session.connection;

    // A greeting, then the five other towns.
    let t = step(c, &format!("{m} talk"), talk(PERION, PERION_CAB)).await;
    assert_eq!(t.prompts, [(PERION_CAB, 0)]);
    let t = step(c, &format!("{m} next"), answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(PERION_CAB, 4)]);
    assert_eq!(t.prompt_texts[0].matches("#L").count(), 5);
    assert!(!t.prompt_texts[0].contains("#m102000000#"));

    // Ellinia is 800 mesos, 80 for a beginner.
    let t = step(c, &format!("{m} ellinia"), answer(4, 1, Some(2))).await;
    assert_eq!(t.prompts, [(PERION_CAB, 1)]);
    let t = step(c, &format!("{m} yes"), answer(1, 1, None)).await;
    assert_eq!(t.stat(stat::MESO), Some(920));
    assert_eq!(t.warps, [ELLINIA]);
}

#[tokio::test]
async fn the_vip_cab_wants_its_whole_fare() {
    let config = load_harness_config_or_fail().await;
    let mut session = rider(&config, 100, 9999, ELLINIA).await;
    let m = session.character_name.clone();
    let c = &mut session.connection;

    // A mesos short: yes is answered with why not, and nothing is taken.
    let t = step(c, &format!("{m} talk"), talk(ELLINIA, ELLINIA_VIP_CAB)).await;
    assert_eq!(t.prompts, [(ELLINIA_VIP_CAB, 0)]);
    let t = step(c, &format!("{m} next"), answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(ELLINIA_VIP_CAB, 1)]);
    let t = step(c, &format!("{m} yes"), answer(1, 1, None)).await;
    assert_eq!(t.prompts, [(ELLINIA_VIP_CAB, 0)]);
    assert_eq!(t.stat(stat::MESO), None);
    assert!(t.warps.is_empty());
    step(c, &format!("{m} close"), answer(0, -1, None)).await;
    let id = session.character_id;
    drop(session);

    // With the whole 10,000 the cab goes to Ant Tunnel Park.
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(id, 10000).await.expect("mesos");
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, ELLINIA, "", &mut t).await, ELLINIA);
    step(c, &format!("{m} talk"), talk(ELLINIA, ELLINIA_VIP_CAB)).await;
    step(c, &format!("{m} next"), answer(0, 1, None)).await;
    let t = step(c, &format!("{m} yes"), answer(1, 1, None)).await;
    assert_eq!(t.stat(stat::MESO), Some(0));
    assert_eq!(t.warps, [ANT_TUNNEL_PARK]);
}
