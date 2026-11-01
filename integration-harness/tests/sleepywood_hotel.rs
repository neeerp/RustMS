//! The Sleepywood Hotel's receptionist takes a guest into the regular or
//! the VIP sauna for its price, and turns away one short of it.

use game_data::server_content::keys;
use integration_harness::database::set_meso;
use integration_harness::events::{said, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::npc_scripts::{HOTEL_RECEPTIONIST, SAUNAS};
use tokio::time::Duration;

const HOTEL: i32 = 105040400;

/// Ask the receptionist for the VIP sauna and say yes.
async fn ask_for_the_vip_sauna(session: &mut WorldSession) -> Trace {
    let trace = told(session, talk(HOTEL, HOTEL_RECEPTIONIST)).await;
    assert!(said(&trace, keys::HOTEL_WELCOME), "{trace:?}");
    let menu = told(session, answer(0, 1, None)).await;
    assert_eq!(menu.prompts, [(HOTEL_RECEPTIONIST, 4)], "{menu:?}");
    let ask = told(session, answer(4, 1, Some(1))).await;
    assert_eq!(ask.prompts, [(HOTEL_RECEPTIONIST, 1)], "{ask:?}");
    told(session, answer(1, 1, None)).await
}

#[tokio::test]
async fn the_receptionist_takes_a_paying_guest_into_the_vip_sauna() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let (vip, price) = SAUNAS[1];
    set_meso(id, price - 1).await.unwrap();

    // One meso short, the guest is turned away after the yes.
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, HOTEL, "", &mut seen).await, HOTEL);
    let trace = ask_for_the_vip_sauna(&mut session).await;
    assert!(said(&trace, keys::HOTEL_SHORT), "{trace:?}");
    told(&mut session, answer(0, 1, None)).await;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(id, price).await.unwrap();

    // With the price, the guest pays and goes in.
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, HOTEL, "", &mut seen).await, HOTEL);
    let mut seen = ask_for_the_vip_sauna(&mut session).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.warps.contains(&vip)).await;
}
