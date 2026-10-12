//! Quests with item and mob objectives and item rewards along the Maple
//! Island route: 1034 (Nina's candy for Sen), 1035 (Todd's hunt), and
//! 1037/1038 (Sam's snails, Maria, and Lucas).

use integration_harness::connection::MapleTestConnection;
use integration_harness::hunting::{attack, pickup, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{act, COMPLETE, START};
use integration_harness::trace::{barrier, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};

const NINA: i32 = 2102;
const SEN: i32 = 2001;
const TODD: i32 = 2004;
const PETER: i32 = 2002;
const SAM: i32 = 2005;
const MARIA: i32 = 2103;
const LUCAS: i32 = 12000;
const SENTINEL: i32 = 9300018;
const SNAIL: i32 = 100100;
const CANDY: i32 = 4031792;
const SHELLPIECE: i32 = 4031802;
const LETTER: i32 = 4031800;
const BEGINNER_HATS: [i32; 5] = [1002053, 1002008, 1002014, 1002068, 1002069];

/// Warp somewhere and wait for `count` of `mob_id` to show up.
async fn enter_hunting_ground(
    c: &mut MapleTestConnection,
    trace: &mut Trace,
    map: i32,
    mob_id: i32,
    count: usize,
) {
    assert_eq!(warp(c, map, "", trace).await, map);
    until(c, trace, |t| t.field.alive(mob_id).len() >= count).await;
}

/// Kill one `mob_id` and wait for it to die.
async fn kill_one(c: &mut MapleTestConnection, trace: &mut Trace, mob_id: i32) {
    let target = trace.field.alive(mob_id)[0];
    send(c, attack(target, 8)).await;
    until(c, trace, |t| t.field.killed.contains(&target)).await;
}

#[tokio::test]
async fn nina_gives_candy_that_sen_shares_with_beginners() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut trace = Trace::default();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;

    // 1034 needs Sen's own errand (1033) done first.
    assert_eq!(warp(c, 30_000, "", &mut trace).await, 30_000);
    act(c, &mut trace, &m, START, 1032, NINA).await;
    assert_eq!(warp(c, 30_001, "", &mut trace).await, 30_001);
    act(c, &mut trace, &m, COMPLETE, 1032, SEN).await;
    act(c, &mut trace, &m, START, 1033, SEN).await;
    assert_eq!(warp(c, 30_000, "", &mut trace).await, 30_000);
    act(c, &mut trace, &m, COMPLETE, 1033, NINA).await;
    // Act/1033/1/nextQuest continues straight into Nina's candy errand.
    assert_eq!(trace.results.last(), Some(&(1033, NINA, 1034)));

    // Accepting hands over the ten candies (Act/1034/0/item).
    act(c, &mut trace, &m, START, 1034, NINA).await;
    assert_eq!(trace.states.get(&1034), Some(&1));
    assert_eq!(trace.items, [(CANDY, 10)]);

    assert_eq!(warp(c, 30_001, "", &mut trace).await, 30_001);
    act(c, &mut trace, &m, COMPLETE, 1034, SEN).await;
    assert_eq!(trace.states.get(&1034), Some(&2));
    // Sen's candies go; two edible ones (job-filtered: beginners) come.
    assert_eq!(&trace.items[1..], [(CANDY, -10), (2022253, 2)]);
    assert_eq!(trace.gains.last(), Some(&20));
    // 1032, 1033, and 1034 each played the "Quest Clear" effect.
    assert_eq!(trace.cleared, 3);
    drop(session);

    let restored = relogin(&config).await;
    assert_eq!(restored.quests.get(&1034), Some(&2));
    assert_eq!(restored.items.get(&CANDY), None);
    assert_eq!(restored.items.get(&2022253), Some(&2));
}

#[tokio::test]
async fn todds_hunt_needs_the_kill_and_the_shellpiece() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut trace = Trace::default();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    enter_hunting_ground(c, &mut trace, 40_000, SENTINEL, 1).await;

    act(c, &mut trace, &m, START, 1035, TODD).await;
    assert_eq!(trace.states.get(&1035), Some(&1));
    // One Jr. Sentinel to hunt, none yet.
    assert_eq!(trace.kills.get(&1035).map(String::as_str), Some("000"));

    // Peter won't finish it before the hunt.
    act(c, &mut trace, &m, COMPLETE, 1035, PETER).await;
    assert_eq!(trace.states.get(&1035), Some(&1));
    assert!(trace.results.iter().all(|r| r.0 != 1035 || r.1 != PETER));

    kill_one(c, &mut trace, SENTINEL).await;
    until(c, &mut trace, |t| {
        t.kills.get(&1035).map(String::as_str) == Some("001")
            && t.field.item_drop(SHELLPIECE).is_some()
    })
    .await;
    // The kill alone isn't enough: the shellpiece must be picked up.
    act(c, &mut trace, &m, COMPLETE, 1035, PETER).await;
    assert_eq!(trace.states.get(&1035), Some(&1));

    let shellpiece = trace.field.item_drop(SHELLPIECE).unwrap();
    send(c, pickup(shellpiece.object_id, shellpiece.x, shellpiece.y)).await;
    until(c, &mut trace, |t| {
        !t.field.removals(shellpiece.object_id).is_empty()
    })
    .await;
    act(c, &mut trace, &m, COMPLETE, 1035, PETER).await;
    assert_eq!(trace.states.get(&1035), Some(&2));
    assert_eq!(trace.gains.last(), Some(&30));
    // One random beginner hat (prop), and the shellpiece is taken.
    let hats: Vec<_> = trace
        .items
        .iter()
        .filter(|(id, count)| BEGINNER_HATS.contains(id) && *count == 1)
        .collect();
    assert_eq!(hats.len(), 1, "{:?}", trace.items);
    assert!(trace.items.contains(&(SHELLPIECE, -1)));
    assert_eq!(trace.cleared, 1);
    let hat = hats[0].0;
    drop(session);

    let restored = relogin(&config).await;
    assert_eq!(restored.quests.get(&1035), Some(&2));
    assert_eq!(restored.items.get(&hat), Some(&1));
    assert_eq!(restored.items.get(&SHELLPIECE), None);
}

#[tokio::test]
async fn snail_kills_persist_and_lead_on_to_lucas() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut trace = Trace::default();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    enter_hunting_ground(c, &mut trace, 50_000, SNAIL, 17).await;
    act(c, &mut trace, &m, START, 1037, SAM).await;
    assert_eq!(trace.kills.get(&1037).map(String::as_str), Some("000"));
    for count in 1..=4 {
        kill_one(c, &mut trace, SNAIL).await;
        let expected = format!("{count:03}");
        until(c, &mut trace, |t| t.kills.get(&1037) == Some(&expected)).await;
    }
    drop(session);

    // The kill count survives a relog.
    let mut session = relogin(&config).await;
    assert_eq!(session.quests.get(&1037), Some(&1));
    assert_eq!(
        session.quest_progress.get(&1037).map(String::as_str),
        Some("004")
    );
    let c = &mut session.connection;
    assert_eq!(warp(c, 40_000, "", &mut trace).await, 40_000);
    enter_hunting_ground(c, &mut trace, 50_000, SNAIL, 6).await;
    for _ in 5..=11 {
        if trace.field.alive(SNAIL).is_empty() {
            until(c, &mut trace, |t| !t.field.alive(SNAIL).is_empty()).await;
        }
        kill_one(c, &mut trace, SNAIL).await;
    }
    barrier(c, &m, &mut trace).await;
    // The eleventh kill isn't counted past the ten needed.
    assert_eq!(trace.kills.get(&1037).map(String::as_str), Some("010"));

    assert_eq!(warp(c, 1_000_000, "", &mut trace).await, 1_000_000);
    act(c, &mut trace, &m, COMPLETE, 1037, MARIA).await;
    assert_eq!(trace.states.get(&1037), Some(&2));
    assert_eq!(trace.gains.last(), Some(&60));
    assert_eq!(trace.results.last(), Some(&(1037, MARIA, 1038)));

    // Maria's letter for Lucas, then potions for delivering it.
    act(c, &mut trace, &m, START, 1038, MARIA).await;
    assert_eq!(trace.items.last(), Some(&(LETTER, 1)));
    // Lucas stands across town, out of reach from the spawn point.
    act(c, &mut trace, &m, COMPLETE, 1038, LUCAS).await;
    assert_eq!(trace.states.get(&1038), Some(&1));
    send(c, walk_to(1500, 154)).await;
    act(c, &mut trace, &m, COMPLETE, 1038, LUCAS).await;
    assert_eq!(trace.states.get(&1038), Some(&2));
    assert!(trace
        .items
        .ends_with(&[(2000000, 5), (2000003, 5), (LETTER, -1)]));
    assert_eq!(trace.gains.last(), Some(&40));
}
