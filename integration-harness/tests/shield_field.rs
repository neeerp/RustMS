//! Two fourth-job skill-quest stops: Moose sends a Paladin on A Shield's
//! True Power (6180), wearing the Skill-Earning Shield, into the practice
//! field, whose Practice Dummies count toward the quest, and his twin there
//! offers the way back; Ice Valley's Ancient Icy Stone trades the Orihalcon
//! Hammer for Ancient Ice Powder.

use game_data::server_content::keys;
use integration_harness::database::{
    grant_item, replace_item, set_job, set_str, set_weapon_attack, start_quest, wear_equip,
};
use integration_harness::events::{said, talk, told};
use integration_harness::hunting::attack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const CROSSROAD: i32 = 240010400;
const SHIELD_FIELD: i32 = 924000001;
const MOOSE: i32 = 2081009;
const MOOSE_IN_THE_FIELD: i32 = 2081010;
const SHIELDS_TRUE_POWER: i16 = 6180;
const SKILL_EARNING_SHIELD: i32 = 1092041;
const PRACTICE_DUMMY: i32 = 9300096;
const ICE_VALLEY: i32 = 921100100;
const ANCIENT_ICY_STONE: i32 = 2030014;
const ORIHALCON_HAMMER: i32 = 4031450;
const ANCIENT_ICE_POWDER: i32 = 2280011;
const ETC: i16 = 4;

#[tokio::test]
async fn moose_sends_a_paladin_with_the_shield_into_the_practice_field_and_back() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 122, 120, 50).await.unwrap();
    replace_item(id, -1, -11, 1302000).await.unwrap();
    set_weapon_attack(id, 1_999).await.unwrap();
    set_str(id, 32_000).await.unwrap();
    start_quest(id, SHIELDS_TRUE_POWER).await.unwrap();

    // Without the shield on, Moose only says to wear it.
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, CROSSROAD, "", &mut seen).await, CROSSROAD);
    let trace = told(&mut session, talk(CROSSROAD, MOOSE)).await;
    assert!(said(&trace, keys::SHIELD_FIELD_WEAR), "{trace:?}");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    wear_equip(id, -10, SKILL_EARNING_SHIELD).await.unwrap();

    // With it, a yes takes the Paladin in.
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, CROSSROAD, "", &mut seen).await, CROSSROAD);
    let trace = told(&mut session, talk(CROSSROAD, MOOSE)).await;
    assert!(said(&trace, keys::SHIELD_FIELD_ASK), "{trace:?}");
    let mut seen = told(&mut session, answer(1, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.warps.contains(&SHIELD_FIELD)).await;

    // A dummy there counts toward the quest.
    until(c, &mut seen, |t| !t.field.alive(PRACTICE_DUMMY).is_empty()).await;
    let dummy = seen.field.alive(PRACTICE_DUMMY)[0];
    send(c, attack(dummy, 5_000_000)).await;
    until(c, &mut seen, |t| {
        t.kills.get(&SHIELDS_TRUE_POWER).map(String::as_str) == Some("001")
    })
    .await;

    // The twin leads back to the crossroad.
    let trace = told(&mut session, talk(SHIELD_FIELD, MOOSE_IN_THE_FIELD)).await;
    assert!(said(&trace, keys::SHIELD_FIELD_LEAVE), "{trace:?}");
    let mut seen = told(&mut session, answer(1, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.warps.contains(&CROSSROAD)).await;
}

#[tokio::test]
async fn the_ancient_icy_stone_trades_the_orihalcon_hammer_for_ice_powder() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_item(id, ETC, 1, ORIHALCON_HAMMER).await.unwrap();

    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, ICE_VALLEY, "", &mut seen).await, ICE_VALLEY);
    let mut seen = told(&mut session, talk(ICE_VALLEY, ANCIENT_ICY_STONE)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.items.contains(&(ANCIENT_ICE_POWDER, 1))).await;
    assert!(seen.items.contains(&(ORIHALCON_HAMMER, -1)), "{seen:?}");
    assert!(seen.prompts.is_empty(), "{seen:?}");
}
