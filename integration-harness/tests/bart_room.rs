//! "Jonathan's Court" (6400): Lord Jonathan sends a Corsair on the quest
//! into his test room, one at a time; hitting the real Bart there sets the
//! record the quest's completion reads, and the room's way out, whose
//! target spawn the data lacks, takes the Corsair back to him. The record
//! outlasts a relogin, goes with a forfeit, and completes the quest.

use game_data::server_content::keys;
use integration_harness::database::{set_job, start_quest};
use integration_harness::events::{maps, said, talk, told};
use integration_harness::hunting::{hit_reactor, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, FORFEIT, START,
};
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::reactors::{BART_FOUND, BART_RECORD, BART_ROOM, JONATHANS_COURT, REAL_BART};
use tokio::time::Duration;

const NAVIGATION_ROOM: i32 = 120000102;
const LORD_JONATHAN: i32 = 1092019;
const BART: i32 = 1094000;
const NAUTILUS_HARBOR: i32 = 120000000;

/// A new level-120 Corsair on "Jonathan's Court", logged in by Lord
/// Jonathan, and the account to log in again with.
async fn corsair() -> (WorldSession, HarnessConfig) {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 522, 120, 50).await.unwrap();
    start_quest(id, JONATHANS_COURT).await.unwrap();
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut session.connection, NAVIGATION_ROOM, "", &mut seen).await,
        NAVIGATION_ROOM
    );
    (session, config)
}

/// `player` asks Lord Jonathan for his test and says yes.
async fn ask_for_the_test(player: &mut WorldSession) -> Trace {
    let trace = told(player, talk(NAVIGATION_ROOM, LORD_JONATHAN)).await;
    assert!(said(&trace, keys::JONATHAN_TEST_ASK), "{trace:?}");
    told(player, answer(1, 1, None)).await
}

/// `player`, sent into the room, finds the real Bart with one hit.
async fn find_bart(player: &mut WorldSession, mut seen: Trace) {
    let c = &mut player.connection;
    until(c, &mut seen, |t| t.warps.contains(&BART_ROOM)).await;
    let on_map = |t: &Trace| {
        t.field
            .reactors
            .iter()
            .find(|r| r.reactor_id == REAL_BART)
            .copied()
    };
    until(c, &mut seen, |t| on_map(t).is_some()).await;
    let bart = on_map(&seen).unwrap();
    send(c, walk_to(bart.x, bart.y)).await;
    send(c, hit_reactor(bart.object_id)).await;
    let line = net::content::text().get(keys::JONATHAN_BART_FOUND, &[]);
    until(c, &mut seen, |t| {
        t.kills.get(&BART_RECORD).map(String::as_str) == Some(BART_FOUND)
            && t.broadcasts.contains(&(5, line.clone()))
    })
    .await;
}

/// Walk `player`, on `map`, up to `npc`, near enough for its quests.
async fn beside(player: &mut WorldSession, map: i32, npc: i32) {
    let field = maps().field(map).unwrap();
    let at = field.map_npcs.iter().find(|n| n.npc_id == npc).unwrap();
    told(player, walk_to(at.x, at.y)).await;
}

/// `player` takes the room's way out, back to Lord Jonathan.
async fn walk_out(player: &mut WorldSession) {
    let mut out = Trace::default();
    assert_eq!(
        warp(&mut player.connection, -1, "ntq00", &mut out).await,
        NAVIGATION_ROOM
    );
}

#[tokio::test]
async fn the_real_bart_sets_the_record_and_the_way_out_leads_back() {
    let (mut a, config) = corsair().await;
    let (mut b, _) = corsair().await;

    // One goes in; the next is turned away while the room is taken.
    let seen = ask_for_the_test(&mut a).await;
    let trace = ask_for_the_test(&mut b).await;
    assert!(said(&trace, keys::JONATHAN_TEST_BUSY), "{trace:?}");
    assert!(trace.warps.is_empty(), "{trace:?}");

    // One hit on the real Bart sets the record, with a pink line, and the
    // way out leads back to Lord Jonathan.
    find_bart(&mut a, seen).await;
    walk_out(&mut a).await;

    // The record outlasts a relogin: he has no test left to give.
    drop(a);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut a = relogin(&config).await;
    let trace = told(&mut a, talk(NAVIGATION_ROOM, LORD_JONATHAN)).await;
    assert!(said(&trace, keys::JONATHAN_HELLO), "{trace:?}");

    // A forfeit takes the record with it, so the test is to take again.
    let trace = told(&mut a, build_quest_action(FORFEIT, JONATHANS_COURT, 0)).await;
    assert!(trace.progress.contains(&(BART_RECORD, 0)), "{trace:?}");
    let mut moved = Trace::default();
    warp(&mut a.connection, NAUTILUS_HARBOR, "", &mut moved).await;
    beside(&mut a, NAUTILUS_HARBOR, BART).await;
    let trace = told(&mut a, build_quest_action(START, JONATHANS_COURT, BART)).await;
    assert_eq!(trace.states.get(&JONATHANS_COURT), Some(&1), "{trace:?}");
    warp(&mut a.connection, NAVIGATION_ROOM, "", &mut moved).await;
    let seen = ask_for_the_test(&mut a).await;
    find_bart(&mut a, seen).await;
    walk_out(&mut a).await;

    // Lord Jonathan completes the quest on the record.
    beside(&mut a, NAVIGATION_ROOM, LORD_JONATHAN).await;
    let trace = told(
        &mut a,
        build_quest_completion(JONATHANS_COURT, LORD_JONATHAN, -1),
    )
    .await;
    assert_eq!(trace.states.get(&JONATHANS_COURT), Some(&2), "{trace:?}");
}
