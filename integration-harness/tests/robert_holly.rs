//! Robert Holly in Ludibrium adds five slots to a buddy list for 240,000
//! mesos, up to 50; the capacity is kept across logins.

use game_data::server_content::keys;
use integration_harness::database::{set_buddy_capacity, set_meso};
use integration_harness::events::{said, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::npc_scripts::{BUDDY_SLOTS_PRICE, ROBERT_HOLLY};
use tokio::time::Duration;

const LUDIBRIUM: i32 = 220000000;

/// Two yeses to Robert Holly, and what followed.
async fn buy(session: &mut WorldSession) -> Trace {
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, LUDIBRIUM, "", &mut seen).await, LUDIBRIUM);
    let offer = told(session, talk(LUDIBRIUM, ROBERT_HOLLY)).await;
    assert!(said(&offer, keys::HOLLY_OFFER), "{offer:?}");
    told(session, answer(1, 1, None)).await;
    told(session, answer(1, 1, None)).await
}

#[tokio::test]
async fn robert_holly_adds_buddy_slots_that_last() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(id, 2 * BUDDY_SLOTS_PRICE).await.unwrap();

    // 20 becomes 25 for the price.
    let mut session = relogin(&config).await;
    let mut seen = buy(&mut session).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.buddy_capacities.contains(&25)).await;
    assert!(said(&seen, keys::HOLLY_DONE), "{seen:?}");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // The 25 was kept: the next five make 30.
    let mut session = relogin(&config).await;
    let mut seen = buy(&mut session).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.buddy_capacities.contains(&30)).await;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // At 50, no more: the mesos stay.
    set_buddy_capacity(id, 50).await.unwrap();
    set_meso(id, BUDDY_SLOTS_PRICE).await.unwrap();
    let mut session = relogin(&config).await;
    let seen = buy(&mut session).await;
    assert!(seen.buddy_capacities.is_empty(), "{seen:?}");
    assert!(
        seen.prompt_texts.iter().any(|t| t.contains("240000")),
        "{seen:?}"
    );
}
