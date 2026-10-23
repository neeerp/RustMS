//! Line 3's construction sites under Kerning City's station: Jake sells a
//! ticket once Shumi's quest is begun, the gate takes it to the site, the
//! way out goes back to the ticketing booth, and the box at the site's end
//! holds Shumi's coin.

use game_data::GameData;
use integration_harness::database::{set_meso, set_quest};
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

const TICKETING: i32 = 103000100;
const JAKE: i32 = 1052006;
const GATE: i32 = 1052007;
const B1: i32 = 103000900;
const B1_END: i32 = 103000902;
const EXIT: i32 = 1052011;
const BOX: i32 = 1052008;
const B1_TICKET: i32 = 4031036;
const SHUMIS_COIN: i32 = 4031039;

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
async fn jake_sells_a_site_ticket_and_the_gate_takes_it_to_the_site() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_quest(id, 2055, 1, "").await.unwrap();
    set_meso(id, 1000).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, TICKETING, "", &mut trace).await, TICKETING);
    barrier(c, &m, &mut trace).await;

    // On Shumi's first quest, Jake sells the ticket to B1.
    let t = step(c, &m, talk(TICKETING, JAKE)).await;
    assert_eq!(t.prompts, [(JAKE, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(JAKE, 4)]);
    let t = step(c, &m, answer(4, 1, Some(0))).await;
    assert_eq!(t.items, [(B1_TICKET, 1)]);
    assert_eq!(t.stat(stat::MESO), Some(0));

    // The gate's first choice, then the ticket, goes straight in.
    let t = step(c, &m, talk(TICKETING, GATE)).await;
    assert_eq!(t.prompts, [(GATE, 4)]);
    let t = step(c, &m, answer(4, 1, Some(0))).await;
    assert_eq!(t.prompts, [(GATE, 4)]);
    let t = step(c, &m, answer(4, 1, Some(0))).await;
    assert_eq!(t.items, [(B1_TICKET, -1)]);
    assert_eq!(t.arrivals, [(B1, 1, None)]);

    // The way out asks, then goes back to the ticketing booth.
    let t = step(c, &m, talk(B1, EXIT)).await;
    assert_eq!(t.prompts, [(EXIT, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(EXIT, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.arrivals, [(TICKETING, 0, None)]);

    // The box at B1's end holds Shumi's coin, and sends the player out
    // without a word.
    assert_eq!(warp(c, B1_END, "", &mut trace).await, B1_END);
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, talk(B1_END, BOX)).await;
    assert!(t.prompts.is_empty());
    assert_eq!(t.items, [(SHUMIS_COIN, 1)]);
    assert_eq!(t.arrivals, [(TICKETING, 0, None)]);
}
