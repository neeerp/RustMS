//! Eurek the Alchemist makes five Magic Rocks or Summoning Rocks of one of
//! five sets of makings and 4,000 mesos.

use integration_harness::database::{grant_stack, set_meso};
use integration_harness::events::{talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::EUREK;
use tokio::time::Duration;

const LUDIBRIUM: i32 = 220000000;
const SUMMONING_ROCK: i32 = 4006001;
const MAKINGS: [(i32, i16); 3] = [(4000080, 7), (4000079, 7), (4011004, 1)];
const ETC: i16 = 4;

#[tokio::test]
async fn eurek_makes_five_summoning_rocks() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(id, 4_000).await.unwrap();
    for (slot, (item, quantity)) in (1..).zip(MAKINGS) {
        grant_stack(id, ETC, slot, item, quantity).await.unwrap();
    }

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, LUDIBRIUM, "", &mut seen).await, LUDIBRIUM);
    let hello = told(&mut session, talk(LUDIBRIUM, EUREK)).await;
    assert_eq!(hello.prompts, [(EUREK, 0)], "{hello:?}");
    let offer = told(&mut session, answer(0, 1, None)).await;
    assert_eq!(offer.prompts, [(EUREK, 4)], "{offer:?}");
    let ways = told(&mut session, answer(4, 1, Some(1))).await;
    assert_eq!(ways.prompts, [(EUREK, 4)], "{ways:?}");
    let asked = told(&mut session, answer(4, 1, Some(4))).await;
    assert_eq!(asked.prompts, [(EUREK, 1)], "{asked:?}");
    let mut seen = told(&mut session, answer(1, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.items.contains(&(SUMMONING_ROCK, 5))).await;
    for (item, quantity) in MAKINGS {
        assert!(
            seen.items.contains(&(item, -i32::from(quantity))),
            "{seen:?}"
        );
    }
}
