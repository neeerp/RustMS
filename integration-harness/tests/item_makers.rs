//! The item makers: Serryl makes a knuckler from Leather and mesos, and
//! refuses without them; Mos forges a weapon with a stimulator, which is
//! taken whether the weapon comes out or is lost.

use game_data::GameData;
use integration_harness::connection::MapleTestConnection;
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

const NAUTILUS_HALLWAY: i32 = 120000200;
const SERRYL: i32 = 1091003;
const LEAFRE: i32 = 240000000;
const MOS: i32 = 2080000;
const LEATHER: i32 = 4000021;
const LEATHER_ARMS: i32 = 1482001;
const SPARTA: i32 = 1302056;
const DRAGON_CARABELLA: i32 = 1302059;
const STIMULATOR: i32 = 4130002;
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

/// Ask Serryl for Leather Arms and say yes.
async fn make_leather_arms(c: &mut MapleTestConnection, m: &str) -> Trace {
    let t = step(c, m, talk(NAUTILUS_HALLWAY, SERRYL)).await;
    assert_eq!(t.prompts, [(SERRYL, 4)]);
    let t = step(c, m, answer(4, 1, Some(0))).await;
    assert_eq!(t.prompts, [(SERRYL, 4)]);
    let t = step(c, m, answer(4, 1, Some(0))).await;
    assert_eq!(t.prompts, [(SERRYL, 1)]);
    step(c, m, answer(1, 1, None)).await
}

#[tokio::test]
async fn serryl_makes_a_knuckler_and_mos_a_stimulated_weapon() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, ETC, 1, LEATHER, 20).await.unwrap();
    grant_item(id, EQUIP, 1, SPARTA).await.unwrap();
    for (slot, item, quantity) in [(2, 4000244, 20), (3, 4000245, 25), (4, 4005000, 8)] {
        grant_stack(id, ETC, slot, item, quantity).await.unwrap();
    }
    grant_item(id, ETC, 5, STIMULATOR).await.unwrap();
    set_meso(id, 121_000).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(
        warp(c, NAUTILUS_HALLWAY, "", &mut trace).await,
        NAUTILUS_HALLWAY
    );
    barrier(c, &m, &mut trace).await;

    // Knucklers, Leather Arms, yes: the Leather and 1,000 mesos for one.
    let t = make_leather_arms(c, &m).await;
    assert_eq!(t.items, [(LEATHER, -20), (LEATHER_ARMS, 1)]);
    assert_eq!(t.stat(stat::MESO), Some(120_000));
    assert_eq!(t.prompts, [(SERRYL, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    // Without the Leather, a word and nothing taken.
    let t = make_leather_arms(c, &m).await;
    assert!(t.items.is_empty());
    assert_eq!(t.prompts, [(SERRYL, 0)]);
    step(c, &m, answer(0, 1, None)).await;

    // Mos: a warrior's weapon with a stimulator, the Dragon Carabella.
    assert_eq!(warp(c, LEAFRE, "", &mut trace).await, LEAFRE);
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, talk(LEAFRE, MOS)).await;
    assert_eq!(t.prompts, [(MOS, 4)]);
    let t = step(c, &m, answer(4, 1, Some(6))).await;
    assert_eq!(t.prompts, [(MOS, 4)]);
    let t = step(c, &m, answer(4, 1, Some(0))).await;
    assert_eq!(t.prompts, [(MOS, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.stat(stat::MESO), Some(0));
    assert_eq!(t.prompts, [(MOS, 0)]);
    let taken = [
        (SPARTA, -1),
        (4000244, -20),
        (4000245, -25),
        (4005000, -8),
        (STIMULATOR, -1),
    ];
    assert_eq!(t.items[..5], taken);
    // One in ten is lost.
    assert!(t.items[5..].is_empty() || t.items[5..] == [(DRAGON_CARABELLA, 1)]);
}
