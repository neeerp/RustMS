//! Trips that end at an NPC rather than a portal: Spinel's world tour to
//! the Mushroom Shrine and back, which outlives a relogin, the Florina
//! Beach guides and Pison, and Thomas Swift between Henesys and Amoria.

use game_data::GameData;
use integration_harness::database::{grant_item, set_job, set_meso};
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

const HENESYS: i32 = 100_000_000;
const LITH_HARBOR: i32 = 104_000_000;
const MUSHROOM_SHRINE: i32 = 800_000_000;
const FLORINA_BEACH: i32 = 110_000_000;
const AMORIA: i32 = 680_000_000;
const SPINEL: i32 = 9000020;
const PASON: i32 = 1002002;
const PISON: i32 = 1081001;
const THOMAS_SWIFT: i32 = 9201022;
const VIP_TICKET_TO_FLORINA: i32 = 4031134;
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

/// Log the character behind `config` in with `meso` mesos, holding `item`
/// if any, at `map`.
async fn traveller(config: &HarnessConfig, meso: i32, item: Option<i32>, map: i32) -> WorldSession {
    let id = login_to_world_session(config)
        .await
        .expect("first login")
        .character_id;
    drop(relogin(config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 10, 50).await.expect("job");
    set_meso(id, meso).await.expect("mesos");
    if let Some(item) = item {
        grant_item(id, ETC, 1, item).await.expect("item");
    }
    let mut session = relogin(config).await;
    let mut t = Trace::default();
    assert_eq!(warp(&mut session.connection, map, "", &mut t).await, map);
    session
}

#[tokio::test]
async fn spinel_tours_the_mushroom_shrine_and_brings_the_traveller_back() {
    let config = load_harness_config_or_fail().await;
    let (tour, back) = (talk(HENESYS, SPINEL), talk(MUSHROOM_SHRINE, SPINEL));
    let mut session = traveller(&config, 5000, None, HENESYS).await;
    let m = session.character_name.clone();
    let c = &mut session.connection;

    // A word on the tour, the one destination, two pages about it, and off.
    let t = step(c, &format!("{m} talk"), tour).await;
    assert_eq!(t.prompts, [(SPINEL, 0)]);
    let t = step(c, &format!("{m} next"), answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(SPINEL, 4)]);
    assert!(t.prompt_texts[0].contains("#m800000000#"));
    let t = step(c, &format!("{m} japan"), answer(4, 1, Some(0))).await;
    assert_eq!(t.prompts, [(SPINEL, 0)]);
    step(c, &format!("{m} next"), answer(0, 1, None)).await;
    let t = step(c, &format!("{m} go"), answer(0, 1, None)).await;
    assert_eq!(t.stat(stat::MESO), Some(2000));
    assert_eq!(t.warps, [MUSHROOM_SHRINE]);
    drop(session);

    // The way back is kept over a relogin, which starts at the shrine, and
    // used once.
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    for home in [HENESYS, LITH_HARBOR] {
        if home == LITH_HARBOR {
            let mut t = Trace::default();
            assert_eq!(warp(c, MUSHROOM_SHRINE, "", &mut t).await, MUSHROOM_SHRINE);
        }
        let t = step(c, &format!("{m} talk"), back.clone()).await;
        assert_eq!(t.prompts, [(SPINEL, 4)]);
        assert!(t.prompt_texts[0].contains(&format!("#m{home}#")));
        let t = step(c, &format!("{m} back"), answer(4, 1, Some(0))).await;
        assert_eq!(t.prompts, [(SPINEL, 0)]);
        let t = step(c, &format!("{m} go"), answer(0, 1, None)).await;
        assert_eq!(t.warps, [home]);
    }
}

#[tokio::test]
async fn florina_beach_costs_a_fare_or_the_ticket_and_pison_sees_players_home() {
    let config = load_harness_config_or_fail().await;
    let (guide, pison) = (talk(LITH_HARBOR, PASON), talk(FLORINA_BEACH, PISON));
    let mut session = traveller(&config, 1500, Some(VIP_TICKET_TO_FLORINA), LITH_HARBOR).await;
    let m = session.character_name.clone();
    let c = &mut session.connection;

    // Pason's fare.
    let t = step(c, &format!("{m} talk"), guide.clone()).await;
    assert_eq!(t.prompts, [(PASON, 4)]);
    let t = step(c, &format!("{m} pay"), answer(4, 1, Some(0))).await;
    assert_eq!(t.prompts, [(PASON, 1)]);
    let t = step(c, &format!("{m} yes"), answer(1, 1, None)).await;
    assert_eq!(t.stat(stat::MESO), Some(0));
    assert_eq!(t.warps, [FLORINA_BEACH]);

    // Pison takes the player back to Lith Harbor.
    let t = step(c, &format!("{m} talk"), pison.clone()).await;
    assert_eq!(t.prompts, [(PISON, 0)]);
    let t = step(c, &format!("{m} next"), answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(PISON, 1)]);
    let t = step(c, &format!("{m} yes"), answer(1, 1, None)).await;
    assert_eq!(t.warps, [LITH_HARBOR]);

    // Without mesos, the VIP ticket goes, and is kept.
    step(c, &format!("{m} talk"), guide).await;
    let t = step(c, &format!("{m} ticket"), answer(4, 1, Some(1))).await;
    assert_eq!(t.prompts, [(PASON, 1)]);
    let t = step(c, &format!("{m} yes"), answer(1, 1, None)).await;
    assert_eq!(t.warps, [FLORINA_BEACH]);
    assert!(t.items.is_empty());
}

#[tokio::test]
async fn thomas_swift_walks_between_henesys_and_amoria() {
    let config = load_harness_config_or_fail().await;
    let (to_amoria, to_henesys) = (talk(HENESYS, THOMAS_SWIFT), talk(AMORIA, THOMAS_SWIFT));
    let mut session = traveller(&config, 0, None, HENESYS).await;
    let m = session.character_name.clone();
    let c = &mut session.connection;
    for (talk, to) in [(to_amoria, AMORIA), (to_henesys, HENESYS)] {
        let t = step(c, &format!("{m} talk"), talk).await;
        assert_eq!(t.prompts, [(THOMAS_SWIFT, 1)]);
        let t = step(c, &format!("{m} yes"), answer(1, 1, None)).await;
        assert_eq!(t.prompts, [(THOMAS_SWIFT, 0)]);
        let t = step(c, &format!("{m} go"), answer(0, 1, None)).await;
        assert_eq!(t.warps, [to]);
    }
}
