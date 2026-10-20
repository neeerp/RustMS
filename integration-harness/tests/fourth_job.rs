//! Fourth job advancement: Harmonia makes a level-120 Crusader who finished
//! the warriors' trials a Hero, teaching the skills with a master level, and
//! a Leafre quest teaches another.

use game_data::GameData;
use integration_harness::database::{set_job, set_quest};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{act, answer, COMPLETE};
use integration_harness::trace::{barrier, stat, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use std::sync::OnceLock;
use tokio::time::Duration;

const HARMONIA: i32 = 2081100;
const INSTRUCTORS: i32 = 240010501;
const WARRIOR_TRIALS: i16 = 6904;
const MAPLE_HERO: i32 = 2280003;
const MOOSE: i32 = 2081009;
const MOOSE_MAP: i32 = 240010400;
const GUARDIAN_QUEST: i16 = 6180;
const GUARDIAN: i32 = 1120005;
const JOB: i32 = 0x20;

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
async fn a_crusader_becomes_a_hero_and_learns_guardian() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 111, 119, 50).await.expect("level 119 Crusader");
    set_quest(id, WARRIOR_TRIALS, 2, "")
        .await
        .expect("the trials passed");

    // Below level 120 Harmonia only talks about the trials.
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    warp(c, INSTRUCTORS, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(INSTRUCTORS, HARMONIA)).await;
    assert_eq!(t.prompts, [(HARMONIA, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    set_job(id, 111, 120, 50).await.expect("level 120 Crusader");
    let mut session = relogin(&config).await;
    let ap = session.ap;
    let c = &mut session.connection;
    let mut t = Trace::default();
    warp(c, INSTRUCTORS, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    let t = step(c, &m, talk(INSTRUCTORS, HARMONIA)).await;
    assert_eq!(t.prompts, [(HARMONIA, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.stat_values(JOB), [112]);
    assert_eq!(t.stat_values(stat::SP), [3]);
    assert_eq!(t.stat_values(stat::AP), [i64::from(ap) + 5]);
    assert_eq!(t.items, [(MAPLE_HERO, 1)]);
    let mut taught = t.skills.clone();
    taught.sort();
    assert_eq!(taught, [(1120004, 0), (1121001, 0), (1121008, 0)]);
    assert_eq!(t.prompts, [(HARMONIA, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // The Hero hunted the Moose's 200 Cold Sharks: Guardian at master
    // level 10.
    set_quest(id, GUARDIAN_QUEST, 1, "200")
        .await
        .expect("the hunt done");
    let mut session = relogin(&config).await;
    assert_eq!(session.job, 112);
    let c = &mut session.connection;
    let mut t = Trace::default();
    warp(c, MOOSE_MAP, "", &mut t).await;
    barrier(c, &m, &mut t).await;
    act(c, &mut t, &m, COMPLETE, GUARDIAN_QUEST, MOOSE).await;
    assert_eq!(t.states.get(&GUARDIAN_QUEST), Some(&2));
    assert_eq!(t.skills, [(GUARDIAN, 0)]);
}
