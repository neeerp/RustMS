//! Captain Al's lessons in leading a family (2232-2234): each ends through
//! his end script, at Captain Al, once the Family window shows a junior
//! taken (2232, for 3,000 EXP), 1,000 Rep earned (2233), then 2,000 earned
//! with under 500 left to spend (2234).

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{join_family, set_family_reputations, set_job, set_quest};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END};
use integration_harness::trace::{step, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use std::time::Duration;

const CAPTAIN_AL: i32 = 1_002_103;
const LITH_HARBOR: i32 = 104_000_000;

async fn new_character(config: &HarnessConfig, level: i16) -> i32 {
    let session = login_to_world_session(config).await.expect("first login");
    let id = session.character_id;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, level, 50).await.expect("level");
    id
}

/// Log in, then go to Captain Al in Lith Harbor.
async fn visit(config: &HarnessConfig) -> WorldSession {
    let mut session = relogin(config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(c, LITH_HARBOR, "", &mut trace).await, LITH_HARBOR);
    step(c, &config.character_name, walk_to(1309, -146)).await;
    session
}

/// Report `quest` to Captain Al; what his script did.
async fn report(c: &mut MapleTestConnection, m: &str, quest: i16) -> Trace {
    let t = step(c, m, build_quest_action(SCRIPT_END, quest, CAPTAIN_AL)).await;
    assert_eq!(t.prompts, [(CAPTAIN_AL, 0)], "{quest}: Captain Al speaks");
    step(c, m, answer(0, 1, None)).await
}

#[tokio::test]
async fn captain_al_ends_each_lesson_by_the_family_window() {
    let (config, junior_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let m = config.character_name.clone();
    let id = new_character(&config, 30).await;
    let junior = new_character(&junior_config, 15).await;
    for (quest, status) in [(2231, 2), (2232, 1), (2233, 1), (2234, 1)] {
        set_quest(id, quest, status, "").await.expect("the lessons");
    }

    // No junior and no Rep yet: nothing ends.
    let mut session = visit(&config).await;
    let c = &mut session.connection;
    for quest in [2232, 2233, 2234] {
        let t = report(c, &m, quest).await;
        assert!(t.progress.is_empty() && t.gains.is_empty(), "{quest}");
    }
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // A junior, and 1,500 Rep earned with all of it left.
    join_family(id, junior).await.expect("a junior");
    set_family_reputations(id, 1500, 1500).await.expect("rep");
    let mut session = visit(&config).await;
    let c = &mut session.connection;
    let t = report(c, &m, 2232).await;
    assert_eq!(t.progress, [(2232, 2)]);
    assert_eq!(t.gains, [3000]);
    let t = report(c, &m, 2233).await;
    assert_eq!(t.progress, [(2233, 2)]);
    assert!(t.gains.is_empty());
    let t = report(c, &m, 2234).await;
    assert!(t.progress.is_empty(), "not 2,000 earned");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 2,000 earned, but 500 left: not yet; 499 left: done.
    set_family_reputations(id, 500, 2000).await.expect("rep");
    let mut session = visit(&config).await;
    let c = &mut session.connection;
    assert!(report(c, &m, 2234).await.progress.is_empty(), "500 left");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_family_reputations(id, 499, 2000).await.expect("rep");
    let mut session = visit(&config).await;
    let c = &mut session.connection;
    assert_eq!(report(c, &m, 2234).await.progress, [(2234, 2)]);
    drop(session);

    let restored = relogin(&config).await;
    for quest in [2232, 2233, 2234] {
        assert_eq!(restored.quests.get(&quest), Some(&2), "quest {quest}");
    }
}
