//! What a quest step does besides giving items. Nella's Don Hwang's Request
//! (2029) takes a 1,000 meso fee to start (`Act/0/money`), and v83's client
//! leaves it to the server to refuse someone who can't pay; Cheng's 3406
//! gives EXP along with the boxes (`Act/0/exp`). Finishing the dragon potion
//! (7301) transforms the player (`Act/1/buffItemID`).

use integration_harness::database::{grant_stack, set_job, set_meso, set_quest};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{act, COMPLETE, START};
use integration_harness::trace::{send, stat, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use std::time::Duration;

const NELLA: i32 = 1052103;
const KERNING_CITY: i32 = 103000000;
const FEE_QUEST: i16 = 2029;
const CHENG: i32 = 2040003;
const TOY_FACTORY: i32 = 220020000;
const EXP_QUEST: i16 = 3406;
const PREREQUISITE: i16 = 3405;
const MOIRA: i32 = 2081006;
const MOIRA_MAP: i32 = 240040700;
const POTION_QUEST: i16 = 7301;
const ETC: i16 = 4;
/// The dragon transformation potion; its buff shows as minus its id.
const DRAGON_POTION: i32 = 2210003;
/// QUEST_RESULT 11: "You do not have enough mesos."
const SHORT_OF_MESOS: u8 = 11;

#[tokio::test]
async fn a_start_fee_is_refused_unpaid_and_taken_once_paid() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let session = login_to_world_session(&config)
        .await
        .expect("first login failed");
    let id = session.character_id;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 30, 50).await.expect("level up");
    set_meso(id, 999).await.expect("set mesos");

    let mut trace = Trace::default();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, KERNING_CITY, "", &mut trace).await, KERNING_CITY);
    send(c, walk_to(71, -54)).await;
    act(c, &mut trace, &m, START, FEE_QUEST, NELLA).await;
    assert_eq!(trace.quest_refusals, [(SHORT_OF_MESOS, 0)]);
    assert_eq!(trace.states.get(&FEE_QUEST), None);
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(id, 1500).await.expect("set mesos");

    let mut trace = Trace::default();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, KERNING_CITY, "", &mut trace).await, KERNING_CITY);
    send(c, walk_to(71, -54)).await;
    act(c, &mut trace, &m, START, FEE_QUEST, NELLA).await;
    assert_eq!(trace.states.get(&FEE_QUEST), Some(&1));
    assert_eq!(trace.stat(stat::MESO), Some(500));
}

#[tokio::test]
async fn a_start_gives_its_exp() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let session = login_to_world_session(&config)
        .await
        .expect("first login failed");
    let id = session.character_id;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 30, 50).await.expect("level up");
    set_quest(id, PREREQUISITE, 2, "")
        .await
        .expect("complete 3405");

    let mut trace = Trace::default();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, TOY_FACTORY, "", &mut trace).await, TOY_FACTORY);
    send(c, walk_to(-265, -1098)).await;
    act(c, &mut trace, &m, START, EXP_QUEST, CHENG).await;
    assert_eq!(trace.states.get(&EXP_QUEST), Some(&1));
    assert_eq!(trace.gains, [1600]);
}

#[tokio::test]
async fn a_completion_gives_the_buff_item_s_effect() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let session = login_to_world_session(&config)
        .await
        .expect("first login failed");
    let id = session.character_id;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_quest(id, POTION_QUEST, 1, "")
        .await
        .expect("start 7301");
    let materials = [
        (4001077, 10),
        (4001078, 1),
        (4001075, 1),
        (4001076, 1),
        (4000267, 1),
    ];
    for (slot, (item_id, count)) in (1..).zip(materials) {
        grant_stack(id, ETC, slot, item_id, count)
            .await
            .expect("hold the materials");
    }

    let mut trace = Trace::default();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, MOIRA_MAP, "", &mut trace).await, MOIRA_MAP);
    send(c, walk_to(217, 184)).await;
    act(c, &mut trace, &m, COMPLETE, POTION_QUEST, MOIRA).await;
    assert_eq!(trace.states.get(&POTION_QUEST), Some(&2));
    assert!(
        trace
            .buffs
            .iter()
            .any(|&(skill, _, ms)| (skill, ms) == (-DRAGON_POTION, 3_600_000)),
        "{:?}",
        trace.buffs
    );
}
