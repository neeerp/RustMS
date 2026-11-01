//! Cloy in Henesys Park tells of pets and sells the Pet AP Reset Scroll for
//! 250,000 mesos.

use integration_harness::database::set_meso;
use integration_harness::events::{talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::{CLOY, PET_AP_RESET_PRICE, PET_AP_RESET_SCROLL};
use tokio::time::Duration;

const HENESYS_PARK: i32 = 100000200;

#[tokio::test]
async fn cloy_sells_a_pet_ap_reset_scroll() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(id, PET_AP_RESET_PRICE).await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, HENESYS_PARK, "", &mut seen).await, HENESYS_PARK);
    let hello = told(&mut session, talk(HENESYS_PARK, CLOY)).await;
    assert_eq!(hello.prompts, [(CLOY, 0)], "{hello:?}");
    let menu = told(&mut session, answer(0, 1, None)).await;
    assert_eq!(menu.prompts, [(CLOY, 4)], "{menu:?}");
    let about = told(&mut session, answer(4, 1, Some(4))).await;
    assert_eq!(about.prompts, [(CLOY, 0)], "{about:?}");
    let asked = told(&mut session, answer(0, 1, None)).await;
    assert_eq!(asked.prompts, [(CLOY, 1)], "{asked:?}");
    let mut seen = told(&mut session, answer(1, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| {
        t.items.contains(&(PET_AP_RESET_SCROLL, 1))
    })
    .await;
}
