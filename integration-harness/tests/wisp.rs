//! Wisp at Eos Tower's entrance in Ludibrium answers questions about pets.

use game_data::server_content::keys;
use integration_harness::events::{said, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::WISP;

const EOS_TOWER_ENTRANCE: i32 = 220000400;

#[tokio::test]
async fn wisp_answers_questions_about_pets() {
    let config = load_harness_config_or_fail().await;
    login_to_world_session(&config).await.unwrap();
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    let at = warp(c, EOS_TOWER_ENTRANCE, "", &mut seen).await;
    assert_eq!(at, EOS_TOWER_ENTRANCE);
    let menu = told(&mut session, talk(EOS_TOWER_ENTRANCE, WISP)).await;
    assert_eq!(menu.prompts, [(WISP, 4)], "{menu:?}");
    assert!(said(&menu, keys::WISP_HELLO), "{menu:?}");
    let told_of = told(&mut session, answer(4, 1, Some(2))).await;
    assert_eq!(told_of.prompts, [(WISP, 0)], "{told_of:?}");
    assert!(said(&told_of, keys::CLOY_RAISING_TOLD), "{told_of:?}");
}
