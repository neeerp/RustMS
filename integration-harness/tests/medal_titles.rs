//! The titles of medals won elsewhere (29910 to 29921 and 29933): with the
//! medal held, each start script starts and completes its quest wherever
//! the player is, with a line in the chat log; without it, nothing starts.

use integration_harness::database::{grant_item, set_job};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{build_quest_action, SCRIPT_START};
use integration_harness::trace::{barrier, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const KERNING_CITY: i32 = 103000000;
const DALAIR: i32 = 9000066;
const SO_GONG: i32 = 2091005;
const HENESYS_DONOR: i16 = 29916;
const HENESYS_DONOR_MEDAL: i32 = 1142014;
const DOJO_VANQUISHER: i16 = 29915;
const DOJO_VANQUISHER_MEDAL: i32 = 1142064;
const PERION_DONOR: i16 = 29918;

#[tokio::test]
async fn a_medal_held_finishes_its_title_quest_anywhere() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 30, 50).await.unwrap();
    grant_item(id, 1, 1, HENESYS_DONOR_MEDAL).await.unwrap();
    grant_item(id, 1, 2, DOJO_VANQUISHER_MEDAL).await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    // Far from either NPC.
    assert_eq!(warp(c, KERNING_CITY, "", &mut trace).await, KERNING_CITY);
    barrier(c, &m, &mut trace).await;
    for (quest, npc) in [(HENESYS_DONOR, DALAIR), (DOJO_VANQUISHER, SO_GONG)] {
        let t = step(c, &m, build_quest_action(SCRIPT_START, quest, npc)).await;
        assert!(t.prompts.is_empty(), "{t:?}");
        assert_eq!(t.states.get(&quest), Some(&2), "{quest}");
        assert_eq!(t.info_texts.len(), 1, "{t:?}");
        assert!(t.items.is_empty(), "the medal is kept: {t:?}");
        // Not twice.
        let t = step(c, &m, build_quest_action(SCRIPT_START, quest, npc)).await;
        assert!(t.states.is_empty() && t.info_texts.is_empty(), "{t:?}");
    }
    // Not without the medal.
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_START, PERION_DONOR, DALAIR),
    )
    .await;
    assert!(t.states.is_empty() && t.info_texts.is_empty(), "{t:?}");
}
