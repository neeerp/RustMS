//! Hikari in Showa Town lets one into the bathhouse's locker room for 300
//! mesos.

use integration_harness::database::set_meso;
use integration_harness::events::{talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::{BATHHOUSE_PRICE, HIKARI, LOCKER_ROOMS};
use tokio::time::Duration;

const SHOWA_TOWN: i32 = 801000000;

#[tokio::test]
async fn hikari_lets_one_into_the_locker_room_for_300_mesos() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(id, BATHHOUSE_PRICE).await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, SHOWA_TOWN, "", &mut seen).await, SHOWA_TOWN);
    let asked = told(&mut session, talk(SHOWA_TOWN, HIKARI)).await;
    assert_eq!(asked.prompts, [(HIKARI, 1)], "{asked:?}");
    let mut seen = told(&mut session, answer(1, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| {
        t.warps.iter().any(|map| LOCKER_ROOMS.contains(map))
    })
    .await;
}
