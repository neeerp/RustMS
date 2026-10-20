//! Quests the client's quest alert offers: the player starts QuestInfo
//! autoStart quests and completes autoComplete ones wherever they are, and
//! v83 sends `QUEST_ACTION` without a position for them.

use integration_harness::database::{set_job, start_quest};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{COMPLETE, START};
use integration_harness::trace::{barrier, send, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

// Blake's Dream: autoStart from level 35, started with Blake in Kerning City
// and completed with Nella.
const BLAKES_DREAM: i16 = 2292;
const BLAKE: i32 = 1052006;
const NELLA: i32 = 1052117;
// The Story Behind the Case: autoComplete, for levels 30-38.
const STORY: i16 = 2313;
const STORY_START: i32 = 1300005;
const STORY_END: i32 = 1300003;

/// `QUEST_ACTION` as v83 sends it for autoStart and autoComplete quests.
fn without_position(action: u8, quest_id: i16, npc_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::QuestAction as i16).unwrap();
    p.write_byte(action).unwrap();
    p.write_short(quest_id).unwrap();
    p.write_int(npc_id).unwrap();
    p
}

#[tokio::test]
async fn the_alert_quests_need_no_npc_near() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 110, 35, 200).await.expect("a fighter");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    send(c, without_position(START, BLAKES_DREAM, BLAKE)).await;
    barrier(c, &m, &mut trace).await;
    assert_eq!(trace.states.get(&BLAKES_DREAM), Some(&1));

    // It isn't autoComplete, so Nella must be near to complete it.
    send(c, without_position(COMPLETE, BLAKES_DREAM, NELLA)).await;
    barrier(c, &m, &mut trace).await;
    assert_eq!(trace.states.get(&BLAKES_DREAM), Some(&1));

    // An autoComplete quest is the other way round.
    send(c, without_position(START, STORY, STORY_START)).await;
    barrier(c, &m, &mut trace).await;
    assert_eq!(trace.states.get(&STORY), None);
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    start_quest(id, STORY).await.expect("story started");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    send(c, without_position(COMPLETE, STORY, STORY_END)).await;
    barrier(c, &m, &mut trace).await;
    assert_eq!(trace.states.get(&STORY), Some(&2));
}
