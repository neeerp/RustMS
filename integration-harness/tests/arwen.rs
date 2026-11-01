//! Arwen the Fairy in Ellinia makes, from level 40, the Moon Rock, the Star
//! Rock and the Black Feather from their makings and mesos.

use integration_harness::database::{grant_stack, set_job, set_meso};
use integration_harness::events::{talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::ARWEN;
use tokio::time::Duration;

const ELLINIA: i32 = 101000000;
const FLAMING_FEATHER: i32 = 4001006;
const MOON_ROCK: i32 = 4011007;
const BLACK_CRYSTAL: i32 = 4021008;
const BLACK_FEATHER: i32 = 4031042;
const ETC: i16 = 4;

#[tokio::test]
async fn arwen_makes_a_black_feather_for_its_makings_and_mesos() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 40, 50).await.unwrap();
    set_meso(id, 30_000).await.unwrap();
    for (slot, item) in [(1, FLAMING_FEATHER), (2, MOON_ROCK), (3, BLACK_CRYSTAL)] {
        grant_stack(id, ETC, slot, item, 1).await.unwrap();
    }

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, ELLINIA, "", &mut seen).await, ELLINIA);
    let hello = told(&mut session, talk(ELLINIA, ARWEN)).await;
    assert_eq!(hello.prompts, [(ARWEN, 0)], "{hello:?}");
    let menu = told(&mut session, answer(0, 1, None)).await;
    assert_eq!(menu.prompts, [(ARWEN, 4)], "{menu:?}");
    let asked = told(&mut session, answer(4, 1, Some(2))).await;
    assert_eq!(asked.prompts, [(ARWEN, 1)], "{asked:?}");
    let mut seen = told(&mut session, answer(1, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.items.contains(&(BLACK_FEATHER, 1))).await;
    for item in [FLAMING_FEATHER, MOON_ROCK, BLACK_CRYSTAL] {
        assert!(seen.items.contains(&(item, -1)), "{seen:?}");
    }
}
