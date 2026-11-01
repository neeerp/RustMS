//! The two Sages on the way up Crimsonwood Mountain each have a word for the
//! map they stand on, and elsewhere a cheer on to the Keep.

use game_data::server_content::keys;
use integration_harness::events::{said, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::{SAGE, UPPER_SAGE};
use tokio::time::Duration;

const VALLEY_OF_HEROES: i32 = 610020000;
const UPPER_ASCENT: i32 = 610020005;

#[tokio::test]
async fn the_sages_warn_of_the_menhirs_cheer_on_and_point_to_the_keep() {
    let config = load_harness_config_or_fail().await;
    login_to_world_session(&config).await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    let at = warp(c, VALLEY_OF_HEROES, "", &mut seen).await;
    assert_eq!(at, VALLEY_OF_HEROES);
    let trace = told(&mut session, talk(VALLEY_OF_HEROES, SAGE)).await;
    assert!(said(&trace, keys::SAGE_MENHIRS), "{trace:?}");
    let trace = told(&mut session, talk(VALLEY_OF_HEROES, UPPER_SAGE)).await;
    assert!(said(&trace, keys::SAGE_ORDEAL), "{trace:?}");

    let c = &mut session.connection;
    assert_eq!(warp(c, UPPER_ASCENT, "", &mut seen).await, UPPER_ASCENT);
    let trace = told(&mut session, talk(UPPER_ASCENT, UPPER_SAGE)).await;
    assert!(said(&trace, keys::SAGE_KEEP), "{trace:?}");
}
