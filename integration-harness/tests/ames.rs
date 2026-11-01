//! Ames the Wise in Amoria tells how to get engaged, marry and break it
//! off.

use game::wedding::AMORIA;
use game_data::server_content::keys;
use integration_harness::events::{said, talk, told};
use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{warp, Trace};
use net::npc_scripts::AMES;

#[tokio::test]
async fn ames_tells_how_to_marry() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config).await.unwrap();
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, AMORIA, "", &mut seen).await, AMORIA);
    let menu = told(&mut session, talk(AMORIA, AMES)).await;
    assert_eq!(menu.prompts, [(AMES, 4)], "{menu:?}");
    let trace = told(&mut session, answer(4, 1, Some(1))).await;
    assert!(said(&trace, keys::AMES_MARRY), "{trace:?}");
}
