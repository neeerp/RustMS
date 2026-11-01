//! The pipes in Magatia's Home of the Missing Alchemist, on Dr. De Lang's
//! Underground Study (3339): turned in turn, the last asks for Phyllia's
//! password, and the right one leads down to the study.

use integration_harness::database::start_quest;
use integration_harness::events::{talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::{STUDY_PASSWORD, STUDY_PIPES};
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const ALCHEMISTS_HOME: i32 = 261000001;
const UNDERGROUND_STUDY: i16 = 3339;

/// The answer to a text prompt.
fn type_text(text: &str) -> Packet {
    let mut p = answer(2, 1, None);
    p.write_str_with_length(text).unwrap();
    p
}

#[tokio::test]
async fn the_pipes_turned_in_turn_and_the_password_lead_down_to_the_study() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    start_quest(id, UNDERGROUND_STUDY).await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(
        warp(c, ALCHEMISTS_HOME, "", &mut seen).await,
        ALCHEMISTS_HOME
    );
    let [first, second, third] = STUDY_PIPES;
    let mut seen = told(&mut session, talk(ALCHEMISTS_HOME, first)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| {
        t.kills.get(&UNDERGROUND_STUDY).map(String::as_str) == Some("1")
    })
    .await;
    let mut seen = told(&mut session, talk(ALCHEMISTS_HOME, second)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| {
        t.kills.get(&UNDERGROUND_STUDY).map(String::as_str) == Some("2")
    })
    .await;
    let asked = told(&mut session, talk(ALCHEMISTS_HOME, third)).await;
    assert_eq!(asked.prompts, [(third, 2)], "{asked:?}");
    let mut seen = told(&mut session, type_text(STUDY_PASSWORD)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.warps.contains(&ALCHEMISTS_HOME)).await;
    assert_eq!(seen.kills[&UNDERGROUND_STUDY], "4", "{seen:?}");
}
