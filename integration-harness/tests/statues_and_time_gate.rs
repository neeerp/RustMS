//! NPCs that warp straight off a menu choice: the Mysterious Statue into
//! the Deep Forest's steps that John's quests have opened, the Crumbling
//! Statue back out, and the Time Gate to the Neo City eras beaten so far.

use game_data::GameData;
use integration_harness::database::set_quest;
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

const STATUE_FOREST: i32 = 105040300;
const MYSTERIOUS_STATUE: i32 = 1061006;
const CRUMBLING_STATUE: i32 = 1061007;
const STEP_3: i32 = 105040312;
const TIME_GATE_FOREST: i32 = 240070000;
const TIME_GATE: i32 = 2083006;
const YEAR_2099: i32 = 240070200;

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
async fn statues_and_the_time_gate_warp_off_a_menu_choice() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    let mut trace = Trace::default();

    // With no quests the statue only speaks, and the gate is asleep.
    assert_eq!(warp(c, STATUE_FOREST, "", &mut trace).await, STATUE_FOREST);
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, talk(STATUE_FOREST, MYSTERIOUS_STATUE)).await;
    assert_eq!(t.prompts, [(MYSTERIOUS_STATUE, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.prompts.is_empty() && t.warps.is_empty());
    assert_eq!(
        warp(c, TIME_GATE_FOREST, "", &mut trace).await,
        TIME_GATE_FOREST
    );
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, talk(TIME_GATE_FOREST, TIME_GATE)).await;
    assert_eq!(t.prompts, [(TIME_GATE, 0)]);
    step(c, &m, answer(0, 1, None)).await;

    let id = session.character_id;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    for (quest, status) in [(2052, 2), (2053, 1), (3718, 2), (3719, 2), (3724, 2)] {
        set_quest(id, quest, status, "").await.unwrap();
    }
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    // John's second quest opens the forest's third step; the menu choice
    // warps with no further question.
    assert_eq!(warp(c, STATUE_FOREST, "", &mut trace).await, STATUE_FOREST);
    barrier(c, &m, &mut trace).await;
    step(c, &m, talk(STATUE_FOREST, MYSTERIOUS_STATUE)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(MYSTERIOUS_STATUE, 4)]);
    let t = step(c, &m, answer(4, 1, Some(1))).await;
    assert_eq!(t.arrivals, [(STEP_3, 0, None)]);

    // The Crumbling Statue there asks, and yes goes back.
    let t = step(c, &m, talk(STEP_3, CRUMBLING_STATUE)).await;
    assert_eq!(t.prompts, [(CRUMBLING_STATUE, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.arrivals, [(STATUE_FOREST, 0, None)]);

    // Two eras beaten: the gate offers both.
    assert_eq!(
        warp(c, TIME_GATE_FOREST, "", &mut trace).await,
        TIME_GATE_FOREST
    );
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, talk(TIME_GATE_FOREST, TIME_GATE)).await;
    assert_eq!(t.prompts, [(TIME_GATE, 4)]);
    let t = step(c, &m, answer(4, 1, Some(1))).await;
    assert_eq!(t.arrivals, [(YEAR_2099, 1, None)]);
}
