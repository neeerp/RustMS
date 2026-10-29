//! Title Challenge - Celebrity (29002): Dalair's start keeps the fame it
//! began from and the goal (`popS`, `popG`), and the end gives the
//! Celebrity Medal once the fame has risen by 1,000 before the end date.

use integration_harness::database::{set_fame, set_job, set_quest};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END, SCRIPT_START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use tokio::time::Duration;

const PERION: i32 = 102000000;
const DALAIR: i32 = 9000040;
const CELEBRITY: i16 = 29002;
const CELEBRITY_MEDAL: i32 = 1142003;

/// Log in, then go to Dalair in Perion.
async fn visit(config: &HarnessConfig) -> WorldSession {
    let mut session = relogin(config).await;
    let c = &mut session.connection;
    let m = &config.character_name;
    let mut trace = Trace::default();
    assert_eq!(warp(c, PERION, "", &mut trace).await, PERION);
    let npcs = &maps().field(PERION).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == DALAIR).unwrap();
    send(c, walk_to(at.x, at.y)).await;
    barrier(c, m, &mut trace).await;
    session
}

/// Dalair's end script: the quest's state afterwards, if it changed, and
/// the items gained.
async fn end(session: &mut WorldSession, m: &str) -> (Option<u8>, Vec<(i32, i32)>) {
    let c = &mut session.connection;
    let t = step(c, m, build_quest_action(SCRIPT_END, CELEBRITY, DALAIR)).await;
    assert_eq!(t.prompts, [(DALAIR, 0)], "{t:?}");
    let t = step(c, m, answer(0, 1, None)).await;
    (t.states.get(&CELEBRITY).copied(), t.items)
}

/// Read until the title is said to have expired (UPDATE_QUEST_INFO 15),
/// and given up, within 10 seconds.
async fn expires(session: &mut WorldSession) {
    let mut trace = Trace::default();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while !trace
        .quest_clocks
        .contains(&(15, CELEBRITY, 0, trace.progress.len()))
    {
        let envelope = tokio::time::timeout(
            deadline.saturating_duration_since(tokio::time::Instant::now()),
            session.connection.read_packet("trace"),
        )
        .await
        .unwrap_or_else(|_| panic!("the title never expired: {trace:?}"))
        .unwrap();
        trace.observe(&envelope.packet);
    }
    assert!(
        trace.quest_clocks.iter().all(|clock| clock.0 == 15),
        "{trace:?}"
    );
}

#[tokio::test]
async fn celebrity_counts_fame_from_the_start_and_ends_with_the_medal() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 100, 30, 50).await.unwrap();
    set_fame(id, 40).await.unwrap();

    let mut session = visit(&config).await;
    let c = &mut session.connection;
    let t = step(c, &m, build_quest_action(SCRIPT_START, CELEBRITY, DALAIR)).await;
    assert_eq!(t.prompts, [(DALAIR, 0)], "{t:?}");
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&CELEBRITY), Some(&1), "{t:?}");
    assert_eq!(
        t.quest_records.last(),
        Some(&(CELEBRITY, "popS=40;popG=1000".to_string())),
        "{t:?}"
    );
    // 960 more is not enough.
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_fame(id, 1000).await.unwrap();
    let mut session = visit(&config).await;
    assert_eq!(end(&mut session, &m).await, (None, vec![]));

    // 1,000 more is, but not once the 30 days are over: logging in after
    // the end date gives the title up, and the chat log says so.
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_fame(id, 1040).await.unwrap();
    let progress = integration_harness::database::quest_progress(id, CELEBRITY)
        .await
        .unwrap()
        .unwrap();
    set_quest(id, CELEBRITY, 1, "20000101000000").await.unwrap();
    let mut session = relogin(&config).await;
    assert_eq!(session.quests.get(&CELEBRITY), None);
    expires(&mut session).await;
    assert_eq!(
        integration_harness::database::quest_progress(id, CELEBRITY)
            .await
            .unwrap(),
        None
    );

    // Nor in the game when the date passes.
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let soon = std::time::SystemTime::now() + Duration::from_secs(3);
    set_quest(id, CELEBRITY, 1, &game::quest::end_date(soon))
        .await
        .unwrap();
    let mut session = relogin(&config).await;
    assert_eq!(session.quests.get(&CELEBRITY), Some(&1));
    expires(&mut session).await;

    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_quest(id, CELEBRITY, 1, &progress).await.unwrap();
    let mut session = visit(&config).await;
    assert_eq!(
        end(&mut session, &m).await,
        (Some(2), vec![(CELEBRITY_MEDAL, 1)])
    );
}
