//! Simon walks players from Happyville to its Shalom Temple and back.

use integration_harness::events::{talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::events::HAPPYVILLE;
use net::npc_scripts::{SHALOM_TEMPLE, SIMON};

#[tokio::test]
async fn simon_walks_to_the_shalom_temple_and_back() {
    let config = load_harness_config_or_fail().await;
    login_to_world_session(&config).await.unwrap();
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, HAPPYVILLE, "", &mut seen).await, HAPPYVILLE);

    for (from, to) in [(HAPPYVILLE, SHALOM_TEMPLE), (SHALOM_TEMPLE, HAPPYVILLE)] {
        let asked = told(&mut session, talk(from, SIMON)).await;
        assert_eq!(asked.prompts, [(SIMON, 1)], "{asked:?}");
        let mut seen = told(&mut session, answer(1, 1, None)).await;
        let c = &mut session.connection;
        until(c, &mut seen, |t| t.warps.contains(&to)).await;
    }
}
