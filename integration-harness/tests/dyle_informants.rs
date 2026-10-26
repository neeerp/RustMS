//! JM's informants in Kerning City (2216-2219): with his list (4031894)
//! from 2214, each informant's start script starts and completes their
//! quest at once, and the last of the four takes the list.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_stack, set_job, set_quest};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_START};
use integration_harness::trace::{step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use std::time::Duration;

const LIST: i32 = 4_031_894;
const ETC: u8 = 4;
/// Each informant's quest, NPC, map and place.
const INFORMANTS: [(i16, i32, i32, i16, i16); 4] = [
    (2216, 9_000_008, 103_000_000, 1112, 400),
    (2217, 1_052_102, 103_000_000, 445, 426),
    (2218, 1_052_103, 103_000_000, 71, -54),
    (2219, 1_052_006, 103_000_100, 71, 187),
];

async fn go(c: &mut MapleTestConnection, m: &str, map: i32, x: i16, y: i16) {
    let mut trace = Trace::default();
    assert_eq!(warp(c, map, "", &mut trace).await, map);
    step(c, m, walk_to(x, y)).await;
}

#[tokio::test]
async fn each_informant_completes_their_quest_at_once() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 400, 30, 100).await.expect("a level 30 thief");
    set_quest(id, 2214, 2, "").await.expect("2214 done");
    grant_stack(id, ETC as i16, 1, LIST, 1)
        .await
        .expect("JM's list");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    for (i, (quest, npc, map, x, y)) in INFORMANTS.into_iter().enumerate() {
        go(c, &m, map, x, y).await;
        let t = step(c, &m, build_quest_action(SCRIPT_START, quest, npc)).await;
        assert_eq!(t.prompts, [(npc, 0)], "{quest} has its start script");
        assert!(t.progress.is_empty(), "{quest} waits for the dialogue");
        let t = step(c, &m, answer(0, 1, None)).await;
        assert_eq!(t.progress, [(quest, 2)], "{quest} is done at once");
        assert_eq!(t.gains, [7_000]);
        if i < 3 {
            assert!(t.removed.is_empty(), "{quest} leaves the list");
        } else {
            assert_eq!(t.removed, [(ETC, 1)], "the last informant takes it");
        }
        // Done once.
        let t = step(c, &m, build_quest_action(SCRIPT_START, quest, npc)).await;
        assert!(t.prompts.is_empty() && t.progress.is_empty());
    }
    drop(session);

    let restored = relogin(&config).await;
    for (quest, ..) in INFORMANTS {
        assert_eq!(restored.quests.get(&quest), Some(&2), "quest {quest}");
    }
}
