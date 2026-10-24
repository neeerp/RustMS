//! The Mu Lung Dojo's clock runs by sections of six floors. A rest floor
//! stops it, so a challenger may stay there as long as they like, and the
//! floor after it starts the next section's clock over with more time:
//! 6 minutes for the second. The entrance taunts whoever arrives. The
//! harness world runs events 60 times as fast, so minutes read as seconds.

use game_data::server_content::keys;
use integration_harness::events::{dojo_challenger, talk, told};
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use net::events::{DOJO, DOJO_ENTRANCE, DOJO_EXIT, DOJO_FIRST, DOJO_TAUNT_WEATHER};
use net::npc_scripts::SO_GONG;

#[tokio::test]
async fn a_rest_floor_stops_the_clock_and_the_next_section_starts_it_over() {
    let mut player = dojo_challenger(30).await;
    told(&mut player, talk(DOJO.lobby, SO_GONG)).await;
    let mut seen = Trace::default();
    send(&mut player.connection, answer(4, 1, Some(0))).await;
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&DOJO_FIRST) && !t.countdowns.is_empty()
    })
    .await;
    assert!((4..=5).contains(&seen.countdowns[0]), "{seen:?}");

    // The sixth floor stops it and shows none, and the first section's 5
    // seconds go by with the challenger still there.
    let c = &mut player.connection;
    let rest = DOJO.stage_map(6);
    let mut seen = Trace::default();
    assert_eq!(warp(c, rest, "", &mut seen).await, rest);
    until(c, &mut seen, |t| t.clocks_stopped > 0).await;
    read_for(c, &mut seen, 6000).await;
    assert!(seen.countdowns.is_empty(), "{seen:?}");
    assert_eq!(seen.warps, [rest], "{seen:?}");

    // The seventh starts the second section's 6, then the time runs out
    // and the challenger is sent to the exit.
    let next = DOJO.stage_map(7);
    let mut seen = Trace::default();
    assert_eq!(warp(c, next, "", &mut seen).await, next);
    until(c, &mut seen, |t| t.countdowns.iter().any(|&left| left >= 5)).await;
    assert!(seen.countdowns.iter().all(|&left| left <= 6), "{seen:?}");
    read_for(c, &mut seen, 4000).await;
    assert!(!seen.warps.contains(&DOJO_EXIT), "{seen:?}");
    until(c, &mut seen, |t| t.warps.contains(&DOJO_EXIT)).await;
}

#[tokio::test]
async fn the_entrance_taunts_whoever_arrives() {
    let mut player = dojo_challenger(30).await;
    let mut seen = Trace::default();
    let c = &mut player.connection;
    assert_eq!(warp(c, DOJO_ENTRANCE, "", &mut seen).await, DOJO_ENTRANCE);
    until(c, &mut seen, |t| !t.weathers.is_empty()).await;
    let taunts = keys::DOJO_TAUNT.map(|key| net::content::text().get(key, &[]));
    let (item_id, taunt) = &seen.weathers[0];
    assert_eq!(*item_id, DOJO_TAUNT_WEATHER);
    assert!(taunts.contains(taunt), "{seen:?}");
}
