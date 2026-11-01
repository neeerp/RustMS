//! The Small Tree Stump atop the Tree That Grew bottles Clear Tree Sap for a
//! Knight on Maybe it's Arwen! (20716), and only shows its sap to others.

use game_data::server_content::keys;
use integration_harness::database::{set_job, start_quest};
use integration_harness::events::{said, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::SMALL_TREE_STUMP;
use tokio::time::Duration;

const TREE_TOP: i32 = 101010103;
const MAYBE_ITS_ARWEN: i16 = 20716;
const CLEAR_TREE_SAP: i32 = 4032142;

#[tokio::test]
async fn the_small_tree_stump_bottles_sap_for_a_knight_on_maybe_its_arwen() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 1100, 20, 50).await.unwrap();

    // Off the quest, there is only the sap to see.
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, TREE_TOP, "", &mut seen).await, TREE_TOP);
    let trace = told(&mut session, talk(TREE_TOP, SMALL_TREE_STUMP)).await;
    assert!(said(&trace, keys::TREE_STUMP_SAP), "{trace:?}");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    start_quest(id, MAYBE_ITS_ARWEN).await.unwrap();

    // On it, the stump fills a bottle.
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, TREE_TOP, "", &mut seen).await, TREE_TOP);
    let mut seen = told(&mut session, talk(TREE_TOP, SMALL_TREE_STUMP)).await;
    assert!(said(&seen, keys::TREE_STUMP_BOTTLED), "{seen:?}");
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.items.contains(&(CLEAR_TREE_SAP, 1))).await;
}
