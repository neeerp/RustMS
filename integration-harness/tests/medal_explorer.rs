//! Exploration medals: entering a region's explorationPoint maps from the
//! medal's level counts each map once toward its title, the count is kept
//! across logins, and the last map earns the title, which Dalair turns into
//! the medal. The highposition portals count toward Touched the Sky.

use integration_harness::database::{explore_map, set_job};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{act, build_quest_action, COMPLETE, FORFEIT};
use integration_harness::trace::{barrier, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const BEGINNER_EXPLORER: i16 = 29005;
const BEGINNER_RECORD: i16 = 27010;
const BEGINNER_MEDAL: i32 = 1142127;
const TOUCHED_THE_SKY: i16 = 29004;
const SKY_RECORD: i16 = 27018;
const DALAIR: i32 = 9000040;
const HENESYS_PARK: i32 = 100000200;
const HIGH_LOOKOUT: i32 = 101010103;
/// An explorationPoint map in no medal's region.
const STRAY: i32 = 910310000;
/// The 20 explorationPoint maps of Victoria Island counted toward Beginner
/// Explorer.
const VICTORIA: [i32; 20] = [
    100000000, 100000006, 100010000, 100040000, 100040100, 101000000, 101010103, 101020000,
    101030104, 101030406, 102000000, 102020300, 102050000, 103000000, 103010001, 103030200,
    104000000, 104010001, 104020000, 110000000,
];

fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// Go to `map` and wait for what its arrival brings.
async fn visit(player: &mut WorldSession, map: i32) -> Trace {
    let mut trace = Trace::default();
    let c = &mut player.connection;
    assert_eq!(warp(c, map, "", &mut trace).await, map);
    barrier(c, &player.character_name, &mut trace).await;
    trace
}

#[tokio::test]
async fn victorias_places_earn_the_beginner_explorer_medal() {
    let config = load_harness_config_or_fail().await;
    let mut player = login_to_world_session(&config).await.unwrap();
    let id = player.character_id;

    // Below the medal's level 15 nothing is counted.
    let low = visit(&mut player, VICTORIA[0]).await;
    assert_eq!(low.states.get(&BEGINNER_EXPLORER), None);
    assert_eq!(low.kills.get(&BEGINNER_RECORD), None);
    drop(player);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 20, 100).await.unwrap();
    let mut player = relogin(&config).await;

    // The first map starts the medal's quest and counts one.
    let first = visit(&mut player, VICTORIA[0]).await;
    assert_eq!(first.states.get(&BEGINNER_EXPLORER), Some(&1));
    assert_eq!(
        first.kills.get(&BEGINNER_RECORD).map(String::as_str),
        Some("1")
    );
    assert_eq!(
        first.script_messages.len(),
        1,
        "{:?}",
        first.script_messages
    );
    assert_eq!(first.info_texts.len(), 1, "{:?}", first.info_texts);
    assert!(first.quest_clears.is_empty());

    let second = visit(&mut player, VICTORIA[1]).await;
    assert_eq!(
        second.kills.get(&BEGINNER_RECORD).map(String::as_str),
        Some("2")
    );
    // A map seen before counts nothing.
    let again = visit(&mut player, VICTORIA[0]).await;
    assert_eq!(again.kills.get(&BEGINNER_RECORD), None);
    assert!(again.script_messages.is_empty());

    // Dalair has no medal for two maps.
    let mut early = visit(&mut player, HENESYS_PARK).await;
    let name = player.character_name.clone();
    let c = &mut player.connection;
    act(c, &mut early, &name, COMPLETE, BEGINNER_EXPLORER, DALAIR).await;
    assert_eq!(early.states.get(&BEGINNER_EXPLORER), None);
    assert!(early.items.is_empty(), "{:?}", early.items);

    // Forfeited, it keeps its maps and starts again on one already seen.
    let gone = step(c, &name, build_quest_action(FORFEIT, BEGINNER_EXPLORER, 0)).await;
    assert_eq!(gone.states.get(&BEGINNER_EXPLORER), Some(&0));
    let back = visit(&mut player, VICTORIA[0]).await;
    assert_eq!(back.states.get(&BEGINNER_EXPLORER), Some(&1));
    assert_eq!(
        back.kills.get(&BEGINNER_RECORD).map(String::as_str),
        Some("2")
    );
    assert!(back.script_messages.is_empty());
    drop(player);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Seen all but two (and one place in no region, which counts as
    // well), the count comes back at login.
    for map in &VICTORIA[2..18] {
        explore_map(id, BEGINNER_RECORD, *map).await.unwrap();
    }
    explore_map(id, BEGINNER_RECORD, STRAY).await.unwrap();
    let mut player = relogin(&config).await;
    assert_eq!(player.quests.get(&BEGINNER_EXPLORER), Some(&1));
    assert_eq!(
        player
            .quest_progress
            .get(&BEGINNER_RECORD)
            .map(String::as_str),
        Some("19")
    );

    // The last earns the title.
    let last = visit(&mut player, VICTORIA[19]).await;
    assert_eq!(
        last.kills.get(&BEGINNER_RECORD).map(String::as_str),
        Some("20")
    );
    assert_eq!(last.quest_clears, [BEGINNER_EXPLORER]);
    assert_eq!(last.script_messages.len(), 1, "{:?}", last.script_messages);
    assert_eq!(last.info_texts.len(), 1, "{:?}", last.info_texts);

    // Dalair gives the medal.
    let mut done = visit(&mut player, HENESYS_PARK).await;
    let c = &mut player.connection;
    act(c, &mut done, &name, COMPLETE, BEGINNER_EXPLORER, DALAIR).await;
    assert_eq!(done.states.get(&BEGINNER_EXPLORER), Some(&2));
    assert!(
        done.items.contains(&(BEGINNER_MEDAL, 1)),
        "{:?}",
        done.items
    );

    // Once it is done its maps count nothing more.
    let unseen = visit(&mut player, VICTORIA[18]).await;
    assert_eq!(unseen.kills.get(&BEGINNER_RECORD), None);
    assert!(unseen.script_messages.is_empty());

    // The lookout's highposition counts toward Touched the Sky without
    // taking the player anywhere.
    visit(&mut player, HIGH_LOOKOUT).await;
    let up = step(&mut player.connection, &name, enter("highposition")).await;
    assert!(up.arrivals.is_empty(), "{:?}", up.arrivals);
    assert_eq!(up.states.get(&TOUCHED_THE_SKY), Some(&1));
    assert_eq!(up.kills.get(&SKY_RECORD).map(String::as_str), Some("1"));
    let twice = step(&mut player.connection, &name, enter("highposition")).await;
    assert_eq!(twice.kills.get(&SKY_RECORD), None);
}
