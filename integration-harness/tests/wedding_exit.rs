//! Ames's room, where every wedding ends, has no portal out: Ames the Wise
//! there sends whoever asks back to Amoria.

use game::wedding::{AMORIA, WEDDING_EXIT};
use game_data::server_content::keys;
use integration_harness::events::{said, talk, told};
use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use net::npc_scripts::AMES_AT_THE_EXIT;

#[tokio::test]
async fn ames_sends_one_in_the_wedding_exit_back_to_amoria() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config).await.unwrap();
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, WEDDING_EXIT, "", &mut seen).await, WEDDING_EXIT);
    let trace = told(&mut session, talk(WEDDING_EXIT, AMES_AT_THE_EXIT)).await;
    assert!(said(&trace, keys::WEDDING_EXIT_BACK), "{trace:?}");
    let mut seen = told(&mut session, answer(0, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.warps.contains(&AMORIA)).await;
}
