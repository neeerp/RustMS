//! Chico in Ludibrium Village makes minigame sets: Match Cards of 99
//! Monster Cards and 10,000 mesos, and Ludibrium's Omok sets.

use integration_harness::database::{grant_stack, set_meso};
use integration_harness::events::{talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::CHICO;
use tokio::time::Duration;

const LUDIBRIUM_VILLAGE: i32 = 220000300;
const MONSTER_CARD: i32 = 4030012;
const MATCH_CARDS: i32 = 4080100;
const ETC: i16 = 4;

#[tokio::test]
async fn chico_makes_match_cards_of_99_monster_cards() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(id, 10_000).await.unwrap();
    grant_stack(id, ETC, 1, MONSTER_CARD, 99).await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    let at = warp(c, LUDIBRIUM_VILLAGE, "", &mut seen).await;
    assert_eq!(at, LUDIBRIUM_VILLAGE);
    let menu = told(&mut session, talk(LUDIBRIUM_VILLAGE, CHICO)).await;
    assert_eq!(menu.prompts, [(CHICO, 4)], "{menu:?}");
    let asked = told(&mut session, answer(4, 1, Some(0))).await;
    assert_eq!(asked.prompts, [(CHICO, 1)], "{asked:?}");
    let mut seen = told(&mut session, answer(1, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.items.contains(&(MATCH_CARDS, 1))).await;
    assert!(seen.items.contains(&(MONSTER_CARD, -99)), "{seen:?}");
}
