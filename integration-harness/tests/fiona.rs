//! Fiona in Dead Man's Gorge forges Crimsonwood weapons and upgrades the
//! Raven weapons, for those who finished The Right Path (8225).

use integration_harness::database::{grant_item, grant_stack, set_meso, set_quest};
use integration_harness::events::{talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::FIONA;
use tokio::time::Duration;

const DEAD_MANS_GORGE: i32 = 610010004;
const RAVENS_CLAW: i32 = 1472072;
const DUSK_RAVENS_CLAW: i32 = 1472075;
const TAO_OF_SIGHT: i32 = 4032016;
const POWER_CRYSTAL: i32 = 4005000;
const BLACK_CRYSTAL: i32 = 4021008;
const EQUIP: i16 = 1;
const ETC: i16 = 4;

#[tokio::test]
async fn fiona_upgrades_a_ravens_claw_into_a_dusk_ravens_claw() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_quest(id, 8225, 2, "").await.unwrap();
    set_meso(id, 50_000).await.unwrap();
    grant_item(id, EQUIP, 1, RAVENS_CLAW).await.unwrap();
    grant_item(id, ETC, 1, TAO_OF_SIGHT).await.unwrap();
    grant_stack(id, ETC, 2, POWER_CRYSTAL, 5).await.unwrap();
    grant_stack(id, ETC, 3, BLACK_CRYSTAL, 20).await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(
        warp(c, DEAD_MANS_GORGE, "", &mut seen).await,
        DEAD_MANS_GORGE
    );
    let menu = told(&mut session, talk(DEAD_MANS_GORGE, FIONA)).await;
    assert_eq!(menu.prompts, [(FIONA, 4)], "{menu:?}");
    let upgrades = told(&mut session, answer(4, 1, Some(1))).await;
    assert_eq!(upgrades.prompts, [(FIONA, 4)], "{upgrades:?}");
    let asked = told(&mut session, answer(4, 1, Some(2))).await;
    assert_eq!(asked.prompts, [(FIONA, 1)], "{asked:?}");
    let mut seen = told(&mut session, answer(1, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.items.contains(&(DUSK_RAVENS_CLAW, 1))).await;
    assert!(seen.items.contains(&(RAVENS_CLAW, -1)), "{seen:?}");
    assert!(seen.items.contains(&(TAO_OF_SIGHT, -1)), "{seen:?}");
}
