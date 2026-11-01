//! Icebyrd Slimm, New Leaf City's mayor, tells of the city, and starts the
//! quiz about it (Welcome to New Leaf City Quiz 1, 4900) for one of level
//! 10 or more, which the quest's own level bounds keep the client from
//! starting.

use game_data::server_content::keys;
use integration_harness::database::set_job;
use integration_harness::events::{said, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::{ICEBYRD_SLIMM, NLC_QUIZ_FIRST};
use tokio::time::Duration;

const NEW_LEAF_CITY: i32 = 600000000;

#[tokio::test]
async fn icebyrd_slimm_tells_of_the_city_and_starts_the_quiz() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 10, 50).await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, NEW_LEAF_CITY, "", &mut seen).await, NEW_LEAF_CITY);

    // A word on the city ends the talk.
    let menu = told(&mut session, talk(NEW_LEAF_CITY, ICEBYRD_SLIMM)).await;
    assert_eq!(menu.prompts, [(ICEBYRD_SLIMM, 4)], "{menu:?}");
    let trace = told(&mut session, answer(4, 1, Some(2))).await;
    assert!(said(&trace, keys::ICEBYRD_DOORS_TOLD), "{trace:?}");

    // The last choice starts the quiz.
    told(&mut session, talk(NEW_LEAF_CITY, ICEBYRD_SLIMM)).await;
    let mut seen = told(&mut session, answer(4, 1, Some(10))).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.progress.contains(&(NLC_QUIZ_FIRST, 1))).await;

    // Once on it, Icebyrd Slimm only keeps to the quiz.
    let trace = told(&mut session, talk(NEW_LEAF_CITY, ICEBYRD_SLIMM)).await;
    assert!(said(&trace, keys::ICEBYRD_QUIZZING), "{trace:?}");
}
