//! Stealing Queen's Order of Silk (3941): Karcasa sends a stranger off, but
//! takes a player in Tigun's disguise for Tigun and hands over the queen's
//! silk.

use integration_harness::database::{grant_stack, set_job, set_quest};
use integration_harness::events::maps;
use integration_harness::hunting::{use_item, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END, SCRIPT_START};
use integration_harness::trace::{step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const KARCASAS_MAP: i32 = 260010600;
const KARCASA: i32 = 2101013;
const MISSION_COMPLETE: i16 = 3940;
const QUEENS_SILK_QUEST: i16 = 3941;
const TIGUNS_DISGUISE: i32 = 2210005;
const QUEENS_SILK: i32 = 4031571;

#[tokio::test]
async fn karcasa_hands_the_silk_to_tigun() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 40, 100).await.unwrap();
    set_quest(id, MISSION_COMPLETE, 1, "").await.unwrap();
    grant_stack(id, 2, 1, TIGUNS_DISGUISE, 1).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(warp(c, KARCASAS_MAP, "", &mut trace).await, KARCASAS_MAP);
    let npcs = &maps().field(KARCASAS_MAP).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == KARCASA).unwrap();
    step(c, &m, walk_to(at.x, at.y)).await;

    // A stranger is sent off.
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_START, QUEENS_SILK_QUEST, KARCASA),
    )
    .await;
    assert_eq!(t.prompts, [(KARCASA, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.prompts.is_empty() && t.states.is_empty(), "{t:?}");

    // Tigun is let in on it.
    let mut t = step(c, &m, use_item(1, TIGUNS_DISGUISE)).await;
    until(c, &mut t, |t| !t.buffs.is_empty()).await;
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_START, QUEENS_SILK_QUEST, KARCASA),
    )
    .await;
    assert_eq!(t.prompts, [(KARCASA, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(KARCASA, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&QUEENS_SILK_QUEST), Some(&1));

    // And given the silk.
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_END, QUEENS_SILK_QUEST, KARCASA),
    )
    .await;
    assert_eq!(t.prompts, [(KARCASA, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.items, [(QUEENS_SILK, 1)]);
    assert_eq!(t.states.get(&QUEENS_SILK_QUEST), Some(&2));
}
