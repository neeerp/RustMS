//! Two fourth-job skill-quest items made by talking to an object: Orbis
//! Tower's Maple Leaf Marble makes a Glass Marble into the Maple Marble that
//! Maple Marble (6230) ends on, and the Dragon Forest's Fountain of Life fills
//! the Holy Cup with the Holy Water of Life for The Holy Knight's Sword (6280).

use game_data::server_content::keys;
use integration_harness::database::{grant_item, start_quest};
use integration_harness::events::{said, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const ORBIS_TOWER: i32 = 200000300;
const MAPLE_LEAF_MARBLE: i32 = 2012023;
const GLASS_MARBLE: i32 = 4031476;
const MAPLE_MARBLE: i32 = 4031456;
const FOUNTAIN_FIELD: i32 = 240050400;
const FOUNTAIN_OF_LIFE: i32 = 2083005;
const HOLY_KNIGHTS_SWORD: i16 = 6280;
const HOLY_CUP: i32 = 4031454;
const HOLY_WATER_OF_LIFE: i32 = 4031455;
const ETC: i16 = 4;

#[tokio::test]
async fn the_maple_leaf_marble_makes_a_glass_marble_into_a_maple_marble() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_item(id, ETC, 1, GLASS_MARBLE).await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, ORBIS_TOWER, "", &mut seen).await, ORBIS_TOWER);
    let mut seen = told(&mut session, talk(ORBIS_TOWER, MAPLE_LEAF_MARBLE)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.items.contains(&(MAPLE_MARBLE, 1))).await;
    assert!(seen.items.contains(&(GLASS_MARBLE, -1)), "{seen:?}");
    assert!(seen.prompts.is_empty(), "{seen:?}");
}

#[tokio::test]
async fn the_fountain_of_life_fills_the_holy_cup_on_the_holy_knights_sword() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_item(id, ETC, 1, HOLY_CUP).await.unwrap();
    start_quest(id, HOLY_KNIGHTS_SWORD).await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, FOUNTAIN_FIELD, "", &mut seen).await, FOUNTAIN_FIELD);
    let mut seen = told(&mut session, talk(FOUNTAIN_FIELD, FOUNTAIN_OF_LIFE)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.items.contains(&(HOLY_WATER_OF_LIFE, 1))).await;
    assert!(seen.items.contains(&(HOLY_CUP, -1)), "{seen:?}");
    assert!(said(&seen, keys::FOUNTAIN_OF_LIFE_POUR), "{seen:?}");
}
