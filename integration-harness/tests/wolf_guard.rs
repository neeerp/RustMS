//! The Wolf Guard at the gate in Rien lets an Aran riding the Werewolf
//! (1902016) into the Field of Wolves, where A Visit from the Wolf Herd
//! (21612) ends, and turns anyone else away.

use game_data::server_content::keys;
use integration_harness::database::{set_job, wear_equip};
use integration_harness::events::{said, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::WOLF_GUARD;
use tokio::time::Duration;

const GATE: i32 = 140010200;
const WOLVES_DEN: i32 = 140010210;
const WEREWOLF: i32 = 1902016;

#[tokio::test]
async fn the_wolf_guard_lets_in_only_an_aran_on_the_werewolf() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 2112, 150, 5_000).await.unwrap();

    // Without the Werewolf, the guard turns the Aran away.
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, GATE, "", &mut seen).await, GATE);
    let trace = told(&mut session, talk(GATE, WOLF_GUARD)).await;
    assert!(said(&trace, keys::WOLF_GUARD_AWAY), "{trace:?}");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    wear_equip(id, -18, WEREWOLF).await.unwrap();

    // Riding it, the Aran goes in.
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, GATE, "", &mut seen).await, GATE);
    let mut seen = told(&mut session, talk(GATE, WOLF_GUARD)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.warps.contains(&WOLVES_DEN)).await;
}
