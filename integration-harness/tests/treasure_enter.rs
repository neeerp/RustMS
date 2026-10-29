//! MV's Lair (Cosmic `TreasurePQ`): Charles in its lobby off New Leaf City
//! takes a party of four to six, all of level 140 or more, into the
//! dungeon. A party with anyone under 140 is turned away.
//! Milla lets a member back out to the lobby.

use integration_harness::database::set_job;
use integration_harness::events::{party, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{TREASURE_DUNGEON, TREASURE_LOBBY};
use net::npc_scripts::CHARLES;
use tokio::time::Duration;

const MILLA: i32 = 9220019;

/// A new character of `level`, logged in at the lobby.
async fn waiting(level: i16) -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 112, level, 50).await.unwrap();
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut session.connection, TREASURE_LOBBY, "", &mut seen).await,
        TREASURE_LOBBY
    );
    session
}

/// The leader asks Charles to go in; returns what came back.
async fn enter(leader: &mut WorldSession) -> Trace {
    told(leader, talk(TREASURE_LOBBY, CHARLES)).await;
    let mut trace = Trace::default();
    send(&mut leader.connection, answer(4, 1, Some(0))).await;
    until(&mut leader.connection, &mut trace, |t| {
        t.warps.contains(&TREASURE_DUNGEON) || t.prompts.iter().any(|p| p.0 == CHARLES)
    })
    .await;
    trace
}

#[tokio::test]
async fn charles_takes_four_of_140_in_and_milla_lets_one_out() {
    // One of 139 is too low for the rest.
    let mut team = Vec::new();
    for level in [140, 140, 140, 139] {
        team.push(waiting(level).await);
    }
    party(&mut team).await;
    let refused = enter(&mut team[0]).await;
    assert!(!refused.warps.contains(&TREASURE_DUNGEON), "{refused:?}");
    drop(team);

    let mut team = Vec::new();
    for _ in 0..4 {
        team.push(waiting(140).await);
    }
    party(&mut team).await;
    let mut seen = None;
    for _ in 0..20 {
        let trace = enter(&mut team[0]).await;
        if trace.warps.contains(&TREASURE_DUNGEON) {
            seen = Some(trace);
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    assert!(seen.is_some(), "Charles never let the party in");
    for player in team.iter_mut() {
        let mut trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&TREASURE_DUNGEON)
        })
        .await;
    }

    // Milla sends a member back to the lobby.
    let member = &mut team[3];
    let asked = told(member, talk(TREASURE_DUNGEON, MILLA)).await;
    assert!(asked.prompts.iter().any(|p| p.0 == MILLA), "{asked:?}");
    let mut trace = told(member, answer(1, 1, None)).await;
    until(&mut member.connection, &mut trace, |t| {
        t.warps.contains(&TREASURE_LOBBY)
    })
    .await;
}
