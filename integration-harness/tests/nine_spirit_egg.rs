//! The Remnants of Horned Tail (3714): in Nine Spirit's nest, each egg
//! (4001094) hatches into a Dragon Stone (2041200) and 42,000 EXP through
//! the baby dragon's start script, which completes the quest at once. It
//! repeats for another egg, but not while a Dragon Stone is held.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_stack, replace_item, set_job, set_quest};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_START};
use integration_harness::trace::{step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use std::time::Duration;

const QUEST: i16 = 3714;
const BABY_DRAGON: i32 = 2_081_011;
const NEST: i32 = 240_040_612;
const EGG: i32 = 4_001_094;
const DRAGON_STONE: i32 = 2_041_200;
const USE: u8 = 2;
const ETC: u8 = 4;

async fn hatch(c: &mut MapleTestConnection, m: &str) -> Trace {
    let mut trace = Trace::default();
    assert_eq!(warp(c, NEST, "", &mut trace).await, NEST);
    step(c, m, walk_to(-8, -227)).await;
    let t = step(c, m, build_quest_action(SCRIPT_START, QUEST, BABY_DRAGON)).await;
    assert_eq!(t.prompts, [(BABY_DRAGON, 0)], "the baby dragon speaks");
    step(c, m, answer(0, 1, None)).await
}

#[tokio::test]
async fn each_egg_hatches_into_a_dragon_stone() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 110, 80, 100).await.expect("a level 80 fighter");
    set_quest(id, 3706, 2, "").await.expect("the nest open");
    grant_stack(id, ETC as i16, 1, EGG, 2)
        .await
        .expect("two eggs");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let t = hatch(c, &m).await;
    assert_eq!(t.progress, [(QUEST, 2)], "done at once");
    assert_eq!(t.gains, [42_000]);
    assert_eq!(t.quantities, [(ETC, 1, 1)], "one egg taken");
    assert_eq!(t.added.len(), 1);
    let (inventory, slot, item) = t.added[0];
    assert_eq!((inventory, item), (USE, DRAGON_STONE));
    // Not while the stone is held.
    let t = hatch(c, &m).await;
    assert!(t.progress.is_empty() && t.gains.is_empty() && t.quantities.is_empty());
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // With the stone gone, the second egg hatches too.
    replace_item(id, USE as i16, slot, 2_000_000)
        .await
        .expect("the stone gone");
    let mut session = relogin(&config).await;
    assert_eq!(session.quests.get(&QUEST), Some(&2));
    let c = &mut session.connection;
    let t = hatch(c, &m).await;
    assert_eq!(t.progress, [(QUEST, 2)], "{QUEST} repeats");
    assert_eq!(t.gains, [42_000]);
    assert_eq!(t.removed, [(ETC, 1)], "the last egg taken");
}
