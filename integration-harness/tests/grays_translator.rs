//! The Endangered Lives of Grays: held up to the Alien Gray, Dr. Kim's
//! faulty Wave Translator gives only noise (3455, a start script that
//! uses it up), and the working one starts 3456.

use integration_harness::database::{grant_stack, set_job};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_START, START,
};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const KULAN_FIELD: i32 = 221040100;
const ALIEN_GRAY: i32 = 2050002;
const FAULTY_TRANSLATOR: i32 = 4031927;
const WORKING_TRANSLATOR: i32 = 4031928;

/// A walk to the Alien Gray.
fn to_gray() -> Packet {
    let npcs = &maps().field(KULAN_FIELD).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == ALIEN_GRAY).unwrap();
    walk_to(at.x, at.y)
}

#[tokio::test]
async fn the_faulty_translator_gives_noise_and_the_working_one_starts_3456() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 50, 50).await.unwrap();
    grant_stack(id, 4, 1, FAULTY_TRANSLATOR, 1).await.unwrap();
    grant_stack(id, 4, 2, WORKING_TRANSLATOR, 1).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(warp(c, KULAN_FIELD, "", &mut trace).await, KULAN_FIELD);
    send(c, to_gray()).await;
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, build_quest_action(SCRIPT_START, 3455, ALIEN_GRAY)).await;
    assert_eq!(t.prompts, [(ALIEN_GRAY, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.progress, [(3455, 1)]);
    assert_eq!(t.items, [(FAULTY_TRANSLATOR, -1)]);
    let t = step(c, &m, build_quest_completion(3455, ALIEN_GRAY, -1)).await;
    assert_eq!(t.states.get(&3455), Some(&2));

    let t = step(c, &m, build_quest_action(START, 3456, ALIEN_GRAY)).await;
    assert_eq!(t.states.get(&3456), Some(&1));
    assert_eq!(t.items, [(WORKING_TRANSLATOR, -1)]);
}
