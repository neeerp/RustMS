//! Rewards the player picks (`Act/1/item` with `prop` -1): Hongbu's gourd
//! (3608) in Korean Folk Town takes the Box of Jewelry and gives a choice
//! of two pairs of shoes, sent as the pick's place in the list.

use integration_harness::database::{grant_item, set_job, set_quest};
use integration_harness::hunting::walk_to;
use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::build_quest_completion;
use integration_harness::relogin;
use integration_harness::trace::{barrier, send, warp, Trace};

const HONGBU: i32 = 2071001;
const HONGBU_X: i16 = -3198;
const HONGBU_Y: i16 = 28;
const FOLK_TOWN: i32 = 222000000;
const GOURD: i16 = 3608;
const JEWELRY: i32 = 4031223;
const HILDON_BOOTS: i32 = 1072133;
const GONI_SHOES: i32 = 1072129;
const ETC: i16 = 4;

#[tokio::test]
async fn the_picked_shoes_are_given() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let session = login_to_world_session(&config)
        .await
        .expect("first login failed");
    let id = session.character_id;
    drop(session);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    set_job(id, 0, 37, 50).await.expect("level up");
    set_quest(id, GOURD, 1, "").await.expect("start 3608");
    grant_item(id, ETC, 1, JEWELRY)
        .await
        .expect("hold the jewelry");

    let mut trace = Trace::default();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, FOLK_TOWN, "", &mut trace).await, FOLK_TOWN);
    send(c, walk_to(HONGBU_X, HONGBU_Y)).await;
    // There are only two pairs to pick from.
    send(c, build_quest_completion(GOURD, HONGBU, 2)).await;
    barrier(c, &m, &mut trace).await;
    assert_eq!(trace.states.get(&GOURD), None);
    assert!(trace.items.is_empty());

    send(c, build_quest_completion(GOURD, HONGBU, 1)).await;
    barrier(c, &m, &mut trace).await;
    assert_eq!(trace.states.get(&GOURD), Some(&2));
    assert!(trace.items.contains(&(GONI_SHOES, 1)));
    assert!(trace.items.contains(&(JEWELRY, -1)));
    assert!(!trace.items.iter().any(|&(item, _)| item == HILDON_BOOTS));
}
