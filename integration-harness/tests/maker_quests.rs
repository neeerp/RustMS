//! Meren's Maker lessons in Magatia: three paid classes with Carson,
//! Hughes and Meren complete "Meren's Lessons" (6029), which teaches the
//! Maker; Meren's crystal test (6033) raises it to level 2.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_stack, set_job, set_meso};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_END, START,
};
use integration_harness::trace::{stat, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use std::time::Duration;

const LESSONS: i16 = 6029;
const MEREN: i32 = 2110004;
const CARSON: i32 = 2111000;
const HUGHES: i32 = 2012017;
/// The NPCs' maps and places.
const MEREN_AT: (i32, i16, i16) = (261_000_002, 56, -103);
const CARSON_AT: (i32, i16, i16) = (261_000_010, -304, -109);
const HUGHES_AT: (i32, i16, i16) = (200_082_301, 89, 78);
/// The explorers' Maker.
const MAKER: i32 = 1007;
/// A level 75 monster crystal, which the crystal test asks to see.
const CRYSTAL: i32 = 4_260_003;
const ETC: i16 = 4;

async fn go(c: &mut MapleTestConnection, m: &str, (map, x, y): (i32, i16, i16)) {
    let mut trace = Trace::default();
    assert_eq!(warp(c, map, "", &mut trace).await, map);
    step(c, m, walk_to(x, y)).await;
}

/// End a class or lesson with `npc`: two pages, then the effects.
async fn end(c: &mut MapleTestConnection, m: &str, quest: i16, npc: i32) -> Trace {
    let t = step(c, m, build_quest_action(SCRIPT_END, quest, npc)).await;
    assert_eq!(t.prompts, [(npc, 0)], "{quest} has its end script");
    step(c, m, answer(0, 1, None)).await;
    step(c, m, answer(0, 1, None)).await
}

#[tokio::test]
async fn merens_lessons_teach_the_maker() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 110, 75, 100).await.expect("a level 75 fighter");
    set_meso(id, 100_000).await.expect("mesos");
    grant_stack(id, ETC, 1, CRYSTAL, 1)
        .await
        .expect("a crystal");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    go(c, &m, MEREN_AT).await;

    // No class before the lessons start.
    let t = step(c, &m, build_quest_action(START, 6032, MEREN)).await;
    assert!(t.progress.is_empty());
    let t = step(c, &m, build_quest_action(START, LESSONS, MEREN)).await;
    assert_eq!(t.progress, [(LESSONS, 1)]);

    // Each class costs 10,000 mesos.
    let mut meso: i64 = 100_000;
    for class in [6030, 6031, 6032] {
        let t = step(c, &m, build_quest_action(START, class, MEREN)).await;
        assert_eq!(t.progress, [(class, 1)]);
        meso -= 10_000;
        assert_eq!(t.stat_values(stat::MESO), [meso]);
    }

    // Meren's own class ends with him; the lessons need all three.
    let t = end(c, &m, 6032, MEREN).await;
    assert_eq!(t.progress, [(6032, 2)]);
    let t = step(c, &m, build_quest_completion(LESSONS, MEREN, -1)).await;
    assert!(t.progress.is_empty() && t.skills.is_empty());
    // A class is taken once.
    let t = step(c, &m, build_quest_action(START, 6032, MEREN)).await;
    assert!(t.progress.is_empty());

    go(c, &m, CARSON_AT).await;
    let t = end(c, &m, 6030, CARSON).await;
    assert_eq!(t.progress, [(6030, 2)]);
    go(c, &m, HUGHES_AT).await;
    let t = end(c, &m, 6031, HUGHES).await;
    assert_eq!(t.progress, [(6031, 2)]);

    go(c, &m, MEREN_AT).await;
    let t = step(c, &m, build_quest_completion(LESSONS, MEREN, -1)).await;
    assert_eq!(t.progress, [(LESSONS, 2)]);
    assert_eq!(t.gains, [50_000]);
    assert_eq!(t.skills, [(MAKER, 1)]);

    // The crystal test: Meren looks at the crystal and teaches level 2.
    let t = step(c, &m, build_quest_action(START, 6033, MEREN)).await;
    assert_eq!(t.progress, [(6033, 1)]);
    let t = end(c, &m, 6033, MEREN).await;
    assert_eq!(t.progress, [(6033, 2)]);
    assert_eq!(t.gains, [230_000]);
    assert_eq!(t.skills, [(MAKER, 2)]);
    drop(session);

    let restored = relogin(&config).await;
    assert_eq!(restored.skills.get(&MAKER), Some(&2));
    for quest in [LESSONS, 6030, 6031, 6032, 6033] {
        assert_eq!(restored.quests.get(&quest), Some(&2), "quest {quest}");
    }
}
