//! Claudia in Amoria's salon cuts once for the coupon her quest gives,
//! rolling the style; once the quest is done and the coupon spent, she
//! says she has cut.

use game_data::server_content::keys;
use integration_harness::database::{grant_item, set_quest};
use integration_harness::events::{said, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::{BEAUTY_OR_BEAST, CLAUDIA, CLAUDIAS_COUPON};
use tokio::time::Duration;

const SALON: i32 = 680000002;
const ETC: i16 = 4;

#[tokio::test]
async fn claudia_cuts_for_the_coupon_and_then_says_she_has() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_item(id, ETC, 1, CLAUDIAS_COUPON).await.unwrap();
    set_quest(id, BEAUTY_OR_BEAST, 2, "").await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, SALON, "", &mut seen).await, SALON);
    let menu = told(&mut session, talk(SALON, CLAUDIA)).await;
    assert_eq!(menu.prompts, [(CLAUDIA, 4)], "{menu:?}");
    let ask = told(&mut session, answer(4, 1, Some(0))).await;
    assert_eq!(ask.prompts, [(CLAUDIA, 1)], "{ask:?}");
    let mut seen = told(&mut session, answer(1, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.items.contains(&(CLAUDIAS_COUPON, -1))).await;
    told(&mut session, answer(0, 1, None)).await;

    let trace = told(&mut session, talk(SALON, CLAUDIA)).await;
    assert!(said(&trace, keys::CLAUDIA_DONE), "{trace:?}");
}
