//! Fourth-job skill-quest stops: Orbis Tower's Maple Leaf Marble makes a
//! Glass Marble into the Maple Marble that Maple Marble (6230) ends on, the
//! Dragon Forest's Fountain of Life fills the Holy Cup with the Holy Water of
//! Life for The Holy Knight's Sword (6280), the Dark Lord sends one on The
//! Night Warrior (6141) into the disciples' room, and Carta takes a Miniature
//! Pianus to send one on Strong Will (6301) to its field.

use game_data::server_content::keys;
use integration_harness::database::{
    grant_item, replace_item, set_job, set_str, set_weapon_attack, start_quest,
};
use integration_harness::events::{said, talk, told};
use integration_harness::hunting::attack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, until, warp, Trace};
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
const DARK_LORDS_HIDEOUT: i32 = 103000003;
const DARK_LORD: i32 = 1052001;
const THE_NIGHT_WARRIOR: i16 = 6141;
const DISCIPLES_ROOM: i32 = 910300000;
const DISCIPLE: i32 = 9300088;
const CARTAS_CAVE: i32 = 230040001;
const CARTA: i32 = 2060100;
const STRONG_WILL: i16 = 6301;
const MINIATURE_PIANUS: i32 = 4000175;
const STRONG_WILL_FIELD: i32 = 923000000;

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

#[tokio::test]
async fn the_dark_lord_sends_a_night_warrior_among_the_disciples() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    // A Hero's swing settles the damage cap simply; the quest counts any kill.
    set_job(id, 122, 120, 50).await.unwrap();
    replace_item(id, -1, -11, 1302000).await.unwrap();
    set_weapon_attack(id, 1_999).await.unwrap();
    set_str(id, 32_000).await.unwrap();
    start_quest(id, THE_NIGHT_WARRIOR).await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    let at = warp(c, DARK_LORDS_HIDEOUT, "", &mut seen).await;
    assert_eq!(at, DARK_LORDS_HIDEOUT);
    let mut seen = told(&mut session, talk(DARK_LORDS_HIDEOUT, DARK_LORD)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.warps.contains(&DISCIPLES_ROOM)).await;
    assert!(seen.prompts.is_empty(), "{seen:?}");

    // A disciple there counts toward the quest.
    until(c, &mut seen, |t| !t.field.alive(DISCIPLE).is_empty()).await;
    let disciple = seen.field.alive(DISCIPLE)[0];
    send(c, attack(disciple, 5_000_000)).await;
    until(c, &mut seen, |t| {
        t.kills.get(&THE_NIGHT_WARRIOR).map(String::as_str) == Some("001")
    })
    .await;
}

#[tokio::test]
async fn carta_takes_a_miniature_pianus_to_send_one_on_strong_will() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    start_quest(id, STRONG_WILL).await.unwrap();

    // Without a Miniature Pianus, Carta says to bring one.
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, CARTAS_CAVE, "", &mut seen).await, CARTAS_CAVE);
    let trace = told(&mut session, talk(CARTAS_CAVE, CARTA)).await;
    assert!(said(&trace, keys::CARTA_NEED_PIANUS), "{trace:?}");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_item(id, ETC, 1, MINIATURE_PIANUS).await.unwrap();

    // With one, Carta takes it and opens the way.
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, CARTAS_CAVE, "", &mut seen).await, CARTAS_CAVE);
    let mut seen = told(&mut session, talk(CARTAS_CAVE, CARTA)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.warps.contains(&STRONG_WILL_FIELD)).await;
    assert!(seen.items.contains(&(MINIATURE_PIANUS, -1)), "{seen:?}");
    assert!(seen.prompts.is_empty(), "{seen:?}");
}
