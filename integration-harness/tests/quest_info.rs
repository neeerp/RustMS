//! Info quests: Sejan's Test starts with the info string "2222", each of
//! Ariant's four houses takes one delivery and turns its digit to '3', the
//! string survives a relogin, and only "3333" completes the quest.

use game_data::GameData;
use integration_harness::database::{set_job, set_quest};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{build_quest_action, build_quest_completion, START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use std::sync::OnceLock;
use tokio::time::Duration;

const RESIDENTIAL_AREA: i32 = 260000200;
const SEJAN: i32 = 2101011;
const SEJANS_TEST: i16 = 3929;
const FOOD: i32 = 4031580;
/// The houses in delivery order with their slot in the string.
const HOUSES: [(i32, i32, usize); 4] = [
    (260000202, 2103003, 0),
    (260000205, 2103005, 1),
    (260000203, 2103004, 2),
    (260000207, 2103006, 3),
];
const SCREWING_THE_SCORPIONS: i16 = 3926;
const HASSAN: i32 = 2101000;

fn maps() -> &'static GameData {
    static MAPS: OnceLock<GameData> = OnceLock::new();
    MAPS.get_or_init(|| {
        GameData::load_from_nx_map(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx"),
        )
        .unwrap()
    })
}

/// The NPC's object id on `map`, and a walk to where it stands.
fn npc(map: i32, npc: i32) -> (i32, Packet) {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let index = npcs.iter().position(|n| n.npc_id == npc).unwrap();
    let at = &npcs[index];
    (1_000_000_000 + index as i32, walk_to(at.x, at.y))
}

fn talk(object_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(object_id).unwrap();
    p
}

#[tokio::test]
async fn ariant_houses_fill_in_sejans_test() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 30, 50).await.unwrap();
    set_quest(id, 3928, 2, "").await.unwrap();
    // A string one delivery short never completes.
    set_quest(id, SCREWING_THE_SCORPIONS, 1, "3323")
        .await
        .unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(
        warp(c, RESIDENTIAL_AREA, "", &mut trace).await,
        RESIDENTIAL_AREA
    );
    send(c, npc(RESIDENTIAL_AREA, HASSAN).1).await;
    barrier(c, &m, &mut trace).await;

    let t = step(
        c,
        &m,
        build_quest_completion(SCREWING_THE_SCORPIONS, HASSAN, -1),
    )
    .await;
    assert_eq!(t.states.get(&SCREWING_THE_SCORPIONS), None);

    send(c, npc(RESIDENTIAL_AREA, SEJAN).1).await;
    // Sejan hands over four parcels of food and the string "2222".
    let t = step(c, &m, build_quest_action(START, SEJANS_TEST, SEJAN)).await;
    assert_eq!(t.states.get(&SEJANS_TEST), Some(&1));
    assert_eq!(t.kills[&SEJANS_TEST], "2222");
    assert_eq!(t.items, [(FOOD, 4)]);
    let t = step(c, &m, build_quest_completion(SEJANS_TEST, SEJAN, -1)).await;
    assert_eq!(t.states.get(&SEJANS_TEST), None);

    let mut info = *b"2222";
    for (map, house, slot) in HOUSES {
        assert_eq!(warp(c, map, "", &mut trace).await, map);
        let (house, walk) = npc(map, house);
        send(c, walk).await;
        barrier(c, &m, &mut trace).await;
        let t = step(c, &m, talk(house)).await;
        info[slot] = b'3';
        assert_eq!(t.kills[&SEJANS_TEST].as_bytes(), info);
        assert_eq!(t.items, [(FOOD, -1)]);
        assert!(t.prompts.is_empty());
        // The house has had its delivery.
        let t = step(c, &m, talk(house)).await;
        assert!(t.kills.is_empty() && t.items.is_empty());
    }

    // The string is kept across a relogin, and completes the test.
    drop(session);
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(
        warp(c, RESIDENTIAL_AREA, "", &mut trace).await,
        RESIDENTIAL_AREA
    );
    send(c, npc(RESIDENTIAL_AREA, SEJAN).1).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_completion(SEJANS_TEST, SEJAN, -1)).await;
    assert_eq!(t.states.get(&SEJANS_TEST), Some(&2));
}
