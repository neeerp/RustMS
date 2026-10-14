//! Roger's Apple (quest 1021, scripted), using items, natural recovery,
//! Robin's Q&A and Shanks's ship.

use game_data::GameData;
use integration_harness::hunting::{use_item, walk_to};
use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END, SCRIPT_START};
use integration_harness::trace::{barrier, send, stat, step, warp, Trace};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use std::sync::OnceLock;
const ROGER: i32 = 2000;
const ROGERS_APPLE: i32 = 2010007;
const SOUTHPERRY: i32 = 60_000;
const SHANKS: i32 = 22000;

fn maps() -> &'static GameData {
    static MAPS: OnceLock<GameData> = OnceLock::new();
    MAPS.get_or_init(|| {
        GameData::load_from_nx_map(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx"),
        )
        .unwrap()
    })
}

fn talk(map: i32, npc: i32) -> Packet {
    let index = maps()
        .field(map)
        .unwrap()
        .map_npcs
        .iter()
        .position(|n| n.npc_id == npc)
        .unwrap();
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(1_000_000_000 + index as i32).unwrap();
    p
}

fn heal(hp: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::HealOverTime as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_int(0).unwrap();
    p.write_short(hp).unwrap();
    p.write_short(0).unwrap();
    p
}

#[tokio::test]
async fn rogers_apple_teaches_using_items_and_pays_once() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, -1, "out00", &mut trace).await, 10_000);
    barrier(c, &m, &mut trace).await;

    // Plain QUEST_ACTION start is refused for a scripted quest.
    let t = step(c, &m, build_quest_action(1, 1021, ROGER)).await;
    assert!(t.progress.is_empty());

    // q1021s: two pages, then yes/no.
    let t = step(c, &m, build_quest_action(SCRIPT_START, 1021, ROGER)).await;
    assert_eq!(t.prompts, [(ROGER, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(ROGER, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.progress, [(1021, 1)]);
    assert_eq!(t.stat_values(stat::HP), [25]);
    let &(kind, slot, item) = t.added.first().expect("Roger's apple");
    assert_eq!((kind, item), (2, ROGERS_APPLE));
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.images, ["UI/tutorial.img/28"]);

    // Can't finish while holding the apple.
    let t = step(c, &m, build_quest_action(SCRIPT_END, 1021, ROGER)).await;
    assert!(t.prompts.is_empty() && t.progress.is_empty());

    // Eat it: +30 HP, consumed.
    let t = step(c, &m, use_item(slot, ROGERS_APPLE)).await;
    assert_eq!(t.stat_values(stat::HP), [50]);
    let t = step(c, &m, use_item(slot, ROGERS_APPLE)).await;
    assert!(t.stat_values(stat::HP).is_empty(), "the apple is gone");

    // q1021e: three pages, then the rewards and completion.
    step(c, &m, build_quest_action(SCRIPT_END, 1021, ROGER)).await;
    step(c, &m, answer(0, 1, None)).await;
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.progress, [(1021, 2)]);
    assert_eq!(t.gains, [10]);
    let rewards: Vec<i32> = t.added.iter().map(|a| a.2).collect();
    assert!(
        rewards.contains(&2010000) || t.added.len() == 1,
        "apples were given: {rewards:?}"
    );

    // Replays do nothing.
    let t = step(c, &m, build_quest_action(SCRIPT_END, 1021, ROGER)).await;
    assert!(t.prompts.is_empty() && t.gains.is_empty());
    drop(session);

    let restored = integration_harness::relogin(&config).await;
    assert_eq!(restored.quests.get(&1021), Some(&2));
    assert_eq!(restored.exp, 10);
}

#[tokio::test]
async fn natural_recovery_is_rate_limited() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, -1, "out00", &mut trace).await, 10_000);
    barrier(c, &m, &mut trace).await;
    step(c, &m, build_quest_action(SCRIPT_START, 1021, ROGER)).await;
    step(c, &m, answer(0, 1, None)).await;
    step(c, &m, answer(0, 1, None)).await;
    step(c, &m, answer(1, 1, None)).await; // HP 25
    let t = step(c, &m, heal(10)).await;
    assert_eq!(t.stat_values(stat::HP), [35]);
    let t = step(c, &m, heal(10)).await;
    assert!(t.stat_values(stat::HP).is_empty(), "too soon");
    let t = step(c, &m, heal(500)).await;
    assert!(t.stat_values(stat::HP).is_empty(), "too much");
}

#[tokio::test]
async fn natural_recovery_waits_for_the_player_to_rest() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, 50_000, "", &mut trace).await, 50_000);
    barrier(c, &m, &mut trace).await;

    // The first heal of a session would count, but not right after moving.
    send(c, walk_to(100, 0)).await;
    let t = step(c, &m, heal(10)).await;
    assert!(t.stat_values(stat::HP).is_empty(), "still moving");
}

#[tokio::test]
async fn robin_answers_questions_about_the_island() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(warp(c, 40_000, "", &mut trace).await, 40_000);
    barrier(c, &m, &mut trace).await;
    let t = step(c, &m, talk(40_000, 2003)).await;
    assert_eq!(t.prompts, [(2003, 4)]);
    // "Tell me more about this island!" has four pages.
    let t = step(c, &m, answer(4, 1, Some(5))).await;
    assert_eq!(t.prompts, [(2003, 0)]);
    for _ in 0..3 {
        let t = step(c, &m, answer(0, 1, None)).await;
        assert_eq!(t.prompts, [(2003, 0)]);
    }
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.prompts.is_empty(), "the conversation ends");
}

#[tokio::test]
async fn shanks_wont_sail_for_a_new_beginner() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, SOUTHPERRY, "", &mut trace).await, SOUTHPERRY);
    barrier(c, &m, &mut trace).await;

    // No: a single parting line.
    let t = step(c, &m, talk(SOUTHPERRY, SHANKS)).await;
    assert_eq!(t.prompts, [(SHANKS, 1)]);
    let t = step(c, &m, answer(1, 0, None)).await;
    assert_eq!(t.prompts, [(SHANKS, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.prompts.is_empty());

    // Yes: the fare, then level 1 is too weak, and he stays in port.
    step(c, &m, talk(SOUTHPERRY, SHANKS)).await;
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.prompts, [(SHANKS, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(SHANKS, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.prompts.is_empty() && t.warps.is_empty());
    assert!(t.stat_values(stat::MESO).is_empty());
}
