//! Chief Tatamo in Leafre sells Magic Seeds, any number up to 99 at once,
//! at 30,000 mesos each.

use integration_harness::database::set_meso;
use integration_harness::events::{talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::{CHIEF_TATAMO, MAGIC_SEED, MAGIC_SEED_PRICE};
use tokio::time::Duration;

const LEAFRE: i32 = 240000000;

#[tokio::test]
async fn chief_tatamo_sells_two_magic_seeds() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(id, 2 * MAGIC_SEED_PRICE).await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, LEAFRE, "", &mut seen).await, LEAFRE);
    let menu = told(&mut session, talk(LEAFRE, CHIEF_TATAMO)).await;
    assert_eq!(menu.prompts, [(CHIEF_TATAMO, 4)], "{menu:?}");
    let how_many = told(&mut session, answer(4, 1, Some(0))).await;
    assert_eq!(how_many.prompts, [(CHIEF_TATAMO, 3)], "{how_many:?}");
    let asked = told(&mut session, answer(3, 1, Some(2))).await;
    assert_eq!(asked.prompts, [(CHIEF_TATAMO, 1)], "{asked:?}");
    let mut seen = told(&mut session, answer(1, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.items.contains(&(MAGIC_SEED, 2))).await;
}
