//! A 3x EXP card (5211060, all day every day) in the Cash inventory triples
//! a kill's EXP, and Hunter's Luck (2450000) doubles it; the gain shows as
//! plain EXP.

use integration_harness::database::grant_item;
use integration_harness::hunting::{attack, use_item};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const USE: i16 = 2;
const CASH: i16 = 5;
const FOREST: i32 = 40_000;
const SNAIL: i32 = 100100;
const TRIPLE_EXP_CARD: i32 = 5211060;
const HUNTERS_LUCK: i32 = 2450000;

#[tokio::test]
async fn an_exp_card_multiplies_kill_exp() {
    let config = load_harness_config_or_fail().await;
    let character_id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_item(character_id, CASH, 1, TRIPLE_EXP_CARD)
        .await
        .expect("grant the card");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, FOREST, "", &mut t).await, FOREST);
    until(c, &mut t, |t| !t.field.alive(SNAIL).is_empty()).await;

    // A snail is worth 3 EXP.
    let snail = t.field.alive(SNAIL)[0];
    send(c, attack(snail, 4)).await;
    send(c, attack(snail, 4)).await;
    until(c, &mut t, |t| !t.gains.is_empty()).await;
    assert_eq!(t.gains[0], 9);
}

#[tokio::test]
async fn hunters_luck_doubles_kill_exp() {
    let config = load_harness_config_or_fail().await;
    let character_id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_item(character_id, USE, 1, HUNTERS_LUCK)
        .await
        .expect("grant Hunter's Luck");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, FOREST, "", &mut t).await, FOREST);
    send(c, use_item(1, HUNTERS_LUCK)).await;
    until(c, &mut t, |t| {
        t.buffs.contains(&(-HUNTERS_LUCK, 200, 1_800_000))
    })
    .await;
    until(c, &mut t, |t| t.field.alive(SNAIL).len() > 1).await;

    // The other test hunts the same map at the same time and takes the first
    // snail, so this one takes the last.
    let snail = *t.field.alive(SNAIL).last().unwrap();
    send(c, attack(snail, 4)).await;
    send(c, attack(snail, 4)).await;
    until(c, &mut t, |t| !t.gains.is_empty()).await;
    assert_eq!(t.gains[0], 6);
}
