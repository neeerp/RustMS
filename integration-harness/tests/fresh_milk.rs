//! Find Fresh Milk (2180): Tangyoon hands over an empty bottle and sends
//! the player to the stable; each mother cow fills it by a third, but not
//! twice running; the way out stays shut until the bottle is full, and the
//! full bottle completes the quest back in the kitchen.

use game_data::GameData;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::start_quest;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_completion};
use integration_harness::trace::{barrier, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use std::sync::OnceLock;
use tokio::time::Duration;

const FRESH_MILK: i16 = 2180;
const KITCHEN: i32 = 120000103;
const TANGYOON: i32 = 1092000;
const STABLE: i32 = 912000100;
const COWS: [i32; 2] = [1092090, 1092091];
const BOTTLES: [i32; 4] = [4031847, 4031848, 4031849, 4031850];
const PINK: u8 = 5;

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

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// Milk `cow`; returns the bottles that changed hands, sorted.
async fn milk(c: &mut MapleTestConnection, m: &str, cow: i32) -> Vec<(i32, i32)> {
    let t = step(c, m, talk(STABLE, cow)).await;
    assert_eq!(t.prompts, [(cow, 0)]);
    step(c, m, answer(0, 1, None)).await;
    let mut items = t.items;
    items.sort();
    items
}

#[tokio::test]
async fn cows_fill_the_bottle_and_the_stable_lets_it_out_full() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    start_quest(id, FRESH_MILK).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, KITCHEN, "", &mut trace).await, KITCHEN);
    barrier(c, &m, &mut trace).await;

    // Two pages, then an empty bottle and off to the stable.
    let t = step(c, &m, talk(KITCHEN, TANGYOON)).await;
    assert_eq!(t.prompts, [(TANGYOON, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(TANGYOON, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.items, [(BOTTLES[0], 1)]);
    assert_eq!(t.warps, [STABLE]);

    // A third from the first cow; she won't give more straight away.
    assert_eq!(
        milk(c, &m, COWS[0]).await,
        [(BOTTLES[0], -1), (BOTTLES[1], 1)]
    );
    assert_eq!(milk(c, &m, COWS[0]).await, []);
    // The way out stays shut on a bottle a third full.
    let t = step(c, &m, enter("ntq2")).await;
    assert!(t.warps.is_empty(), "{:?}", t.warps);
    assert!(matches!(&t.broadcasts[..], [(PINK, text)] if !text.is_empty()));
    assert_eq!(
        milk(c, &m, COWS[1]).await,
        [(BOTTLES[1], -1), (BOTTLES[2], 1)]
    );
    assert_eq!(
        milk(c, &m, COWS[0]).await,
        [(BOTTLES[2], -1), (BOTTLES[3], 1)]
    );

    // Full: back to the kitchen, where the bottle completes the quest.
    let t = step(c, &m, enter("ntq2")).await;
    assert_eq!(t.portal_sounds, 1);
    assert_eq!(t.warps, [KITCHEN]);
    let t = step(c, &m, build_quest_completion(FRESH_MILK, TANGYOON, -1)).await;
    assert!(t.items.contains(&(BOTTLES[3], -1)), "{:?}", t.items);
}
