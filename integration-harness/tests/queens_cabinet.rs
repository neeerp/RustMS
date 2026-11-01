//! The Queen's cabinet in Ariant's palace holds the ring the third of the
//! Sand Bandits' requests (3923) ends on, and gives it up while the quest
//! is under way.

use game_data::server_content::keys;
use integration_harness::database::start_quest;
use integration_harness::events::{said, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::QUEENS_CABINET;
use tokio::time::Duration;

const QUEENS_ROOM: i32 = 260000303;
const SAND_BANDITS_REQUEST: i16 = 3923;
const QUEENS_RING: i32 = 4031578;

#[tokio::test]
async fn the_queens_cabinet_gives_up_the_ring() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    start_quest(id, SAND_BANDITS_REQUEST).await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, QUEENS_ROOM, "", &mut seen).await, QUEENS_ROOM);
    let mut seen = told(&mut session, talk(QUEENS_ROOM, QUEENS_CABINET)).await;
    assert!(said(&seen, keys::QUEENS_CABINET_SWIPED), "{seen:?}");
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.items.contains(&(QUEENS_RING, 1))).await;
}
