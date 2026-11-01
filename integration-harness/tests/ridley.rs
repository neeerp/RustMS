//! Ridley in Crimsonwood Keep tells one of the Keep's party quest's level
//! of its expeditions.

use game_data::server_content::keys;
use integration_harness::events::{said, talk, told, waiting_as};
use net::events::CWK;
use net::npc_scripts::RIDLEY;

const CRIMSONWOOD_KEEP: i32 = 610020006;

#[tokio::test]
async fn ridley_tells_one_of_the_keeps_level_of_its_expeditions() {
    let level = i16::from(*CWK.levels.start());
    let mut player = waiting_as(CRIMSONWOOD_KEEP, level, 0).await;
    let trace = told(&mut player, talk(CRIMSONWOOD_KEEP, RIDLEY)).await;
    assert!(said(&trace, keys::RIDLEY_EXPEDITIONS), "{trace:?}");
}
