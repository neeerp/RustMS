//! The original beginner visits (1031 Heena → Sera, 1032/1033 Nina ⇄ Sen)
//! through the v83 quest flow, along the asset-defined route.

use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{act, COMPLETE, START};
use integration_harness::trace::{warp, Trace};
use integration_harness::{login_to_world_session, relogin};

const HEENA: i32 = 2101;
const SERA: i32 = 2100;
const NINA: i32 = 2102;
const SEN: i32 = 2001;

#[tokio::test]
async fn beginner_visits_follow_portals_and_persist_rewards_once() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut trace = Trace::default();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    assert_eq!(session.map_id, 0);

    act(c, &mut trace, &m, START, 1031, HEENA).await;
    assert_eq!(trace.states.get(&1031), Some(&1));
    assert_eq!(trace.results, [(1031, HEENA, 0)]);
    // Heena gives directions; only Sera completes the visit.
    act(c, &mut trace, &m, COMPLETE, 1031, HEENA).await;
    assert!(trace.gains.is_empty());
    act(c, &mut trace, &m, COMPLETE, 1031, SERA).await;
    assert_eq!(trace.states.get(&1031), Some(&2));
    assert_eq!(trace.gains, [5]);
    act(c, &mut trace, &m, COMPLETE, 1031, SERA).await;
    assert_eq!(trace.gains, [5]);

    assert_eq!(warp(c, -1, "out00", &mut trace).await, 10_000);
    assert_eq!(warp(c, -1, "out00", &mut trace).await, 20_000);
    act(c, &mut trace, &m, START, 1032, NINA).await;
    assert_eq!(trace.states.get(&1032), Some(&1));
    // Sen is inside his house, not on Nina's map.
    act(c, &mut trace, &m, COMPLETE, 1032, SEN).await;
    assert_eq!(trace.gains, [5]);
    drop(session);

    let mut session = relogin(&config).await;
    assert_eq!((session.map_id, session.exp), (20_000, 5));
    assert_eq!(session.quests.get(&1032), Some(&1));
    let c = &mut session.connection;
    assert_eq!(warp(c, -1, "in01", &mut trace).await, 20_001);
    act(c, &mut trace, &m, COMPLETE, 1032, SEN).await;
    // Act/1032/1/nextQuest makes the client continue into Sen's own offer.
    assert_eq!(trace.results.last(), Some(&(1032, SEN, 1033)));
    assert_eq!(trace.gains, [5, 7]);
    // Sen's follow-up needs 1032 completed, which it now is.
    act(c, &mut trace, &m, START, 1033, SEN).await;
    assert_eq!(trace.states.get(&1033), Some(&1));
    assert_eq!(warp(c, -1, "out00", &mut trace).await, 20_000);
    act(c, &mut trace, &m, COMPLETE, 1033, NINA).await;
    assert_eq!(trace.gains, [5, 7, 7]);
    // 1033's nextQuest continues into Nina's candy errand (1034), which
    // quest_objectives.rs covers.
    assert_eq!(trace.results.last(), Some(&(1033, NINA, 1034)));
    drop(session);

    let restored = relogin(&config).await;
    assert_eq!((restored.level, restored.exp, restored.ap), (2, 4, 5));
    for id in [1031, 1032, 1033] {
        assert_eq!(restored.quests.get(&id), Some(&2));
    }
    assert_eq!(restored.quests.len(), 3);
}
