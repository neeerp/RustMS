//! Casey in Henesys's Game Park makes minigame sets: a set of Match Cards
//! for 15 Monster Cards, and an Omok set for 99 each of two kinds of piece
//! and an Omok Table.

use integration_harness::database::grant_stack;
use integration_harness::events::{talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::CASEY;
use tokio::time::Duration;

const GAME_PARK: i32 = 100000203;
const MONSTER_CARD: i32 = 4030012;
const MATCH_CARDS: i32 = 4080100;
const ETC: i16 = 4;

#[tokio::test]
async fn casey_makes_match_cards_of_fifteen_monster_cards() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, ETC, 1, MONSTER_CARD, 15).await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, GAME_PARK, "", &mut seen).await, GAME_PARK);
    let menu = told(&mut session, talk(GAME_PARK, CASEY)).await;
    assert_eq!(menu.prompts, [(CASEY, 4)], "{menu:?}");
    let asked = told(&mut session, answer(4, 1, Some(1))).await;
    assert_eq!(asked.prompts, [(CASEY, 1)], "{asked:?}");
    let mut seen = told(&mut session, answer(1, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.items.contains(&(MATCH_CARDS, 1))).await;
    assert!(seen.items.contains(&(MONSTER_CARD, -15)), "{seen:?}");
}
