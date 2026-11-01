//! Item traders take a pile of one etc item for a prize drawn from its
//! list: Faito in Showa takes 100 Crow Feathers, and Staff Sergeant Charlie
//! in Orbis takes 100 of an item after a yes, with 500 EXP besides.

use integration_harness::database::grant_stack;
use integration_harness::events::{talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::{FAITO, STAFF_SERGEANT_CHARLIE};
use tokio::time::Duration;

const FAITOS_MAP: i32 = 801000300;
const CROW_FEATHER: i32 = 4000064;
const ORBIS: i32 = 200000000;
const CHARLIES_FIRST: i32 = 4000073;
const ETC: i16 = 4;

#[tokio::test]
async fn faito_trades_a_hundred_crow_feathers_for_a_prize() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, ETC, 1, CROW_FEATHER, 100).await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, FAITOS_MAP, "", &mut seen).await, FAITOS_MAP);
    let deal = told(&mut session, talk(FAITOS_MAP, FAITO)).await;
    assert_eq!(deal.prompts, [(FAITO, 1)], "{deal:?}");
    let menu = told(&mut session, answer(1, 1, None)).await;
    assert_eq!(menu.prompts, [(FAITO, 4)], "{menu:?}");
    let mut seen = told(&mut session, answer(4, 1, Some(0))).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| {
        t.items.contains(&(CROW_FEATHER, -100))
            && t.items
                .iter()
                .any(|&(item, quantity)| item != CROW_FEATHER && quantity > 0)
    })
    .await;
}

#[tokio::test]
async fn charlie_trades_after_a_yes() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, ETC, 1, CHARLIES_FIRST, 100).await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, ORBIS, "", &mut seen).await, ORBIS);
    let intro = told(&mut session, talk(ORBIS, STAFF_SERGEANT_CHARLIE)).await;
    assert_eq!(intro.prompts, [(STAFF_SERGEANT_CHARLIE, 0)], "{intro:?}");
    told(&mut session, answer(0, 1, None)).await;
    let menu = told(&mut session, answer(1, 1, None)).await;
    assert_eq!(menu.prompts, [(STAFF_SERGEANT_CHARLIE, 4)], "{menu:?}");
    let confirm = told(&mut session, answer(4, 1, Some(0))).await;
    assert_eq!(
        confirm.prompts,
        [(STAFF_SERGEANT_CHARLIE, 1)],
        "{confirm:?}"
    );
    let mut seen = told(&mut session, answer(1, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| {
        t.items.contains(&(CHARLIES_FIRST, -100))
            && t.items
                .iter()
                .any(|&(item, quantity)| item != CHARLIES_FIRST && quantity > 0)
    })
    .await;
}
