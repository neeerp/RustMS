//! The Pirate Ship, its first slice: Guon Over the Pirate Ship takes those
//! of a party of three to six at levels 55 to 100 who are there aboard
//! their own copy of the ship, on a 4-minute clock that the first arrival
//! on the second stage starts over at 6; the Guon aboard lets a member
//! off, which leaves too few and ends it for everyone, who land on the way
//! out; and the Guon there sends them back. The harness world runs events
//! 60 times as fast (`RUSTMS_EVENT_SCALE`), so the clock reads seconds.

use game_data::server_content::keys;
use integration_harness::events::{armed_in, enter_pirate, party, said, talk, told};
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, until, warp, Trace};
use net::events::{PIRATE, PIRATE_ENTRY, PIRATE_EXIT, PIRATE_LOBBY, PIRATE_STAGES};
use net::npc_scripts::{GUON, GUON_ABOARD};

/// A map of Herb Town's beside the lobby.
const HERB_TOWN_PATH: i32 = 251_010_401;

#[tokio::test]
async fn guon_takes_three_aboard_and_the_second_stage_gives_more_time() {
    let mut team = Vec::new();
    for level in [55, 70, 100, 54] {
        team.push(armed_in(PIRATE_LOBBY, level).await);
    }
    party(&mut team).await;

    // Only the leader may take the party aboard, and only with three of
    // levels 55 to 100 there: with one away, the one of level 54 makes no
    // third.
    let mut trace = Trace::default();
    let away = &mut team[2].connection;
    assert_eq!(
        warp(away, HERB_TOWN_PATH, "", &mut trace).await,
        HERB_TOWN_PATH
    );
    told(&mut team[1], talk(PIRATE_LOBBY, GUON)).await;
    let trace = told(&mut team[1], answer(4, 1, Some(0))).await;
    assert!(said(&trace, keys::PIRATE_NOT_LEADER), "{trace:?}");
    let menu = told(&mut team[0], talk(PIRATE_LOBBY, GUON)).await;
    assert_eq!(menu.prompts, [(GUON, 4)], "{menu:?}");
    let trace = told(&mut team[0], answer(4, 1, Some(0))).await;
    let (team_size, levels) = (&PIRATE.team, &PIRATE.levels);
    let not_eligible = net::content::text().get(
        keys::PIRATE_NOT_ELIGIBLE,
        &[
            ("least", &team_size.start().to_string()),
            ("most", &team_size.end().to_string()),
            ("low", &levels.start().to_string()),
            ("high", &levels.end().to_string()),
        ],
    );
    assert!(
        trace.prompt_texts.iter().any(|t| t.contains(&not_eligible)),
        "{trace:?}"
    );

    // With them back, the three go aboard, on the clock, and the one of
    // level 54 stays.
    let mut trace = Trace::default();
    let back = &mut team[2].connection;
    assert_eq!(warp(back, PIRATE_LOBBY, "", &mut trace).await, PIRATE_LOBBY);
    let mut left_behind = team.pop().unwrap();
    let mut seen = enter_pirate(&mut team).await;

    // The first on the second stage, at once (the clock reads 4 seconds),
    // starts everyone's clock over at 6.
    let c = &mut team[1].connection;
    assert_eq!(
        warp(c, PIRATE_STAGES[1], "", &mut seen[1]).await,
        PIRATE_STAGES[1]
    );
    for (player, trace) in team.iter_mut().zip(seen.iter_mut()) {
        until(&mut player.connection, trace, |t| {
            t.countdowns.iter().any(|&left| left >= 5)
        })
        .await;
        assert!((2..=4).contains(&trace.countdowns[0]), "{trace:?}");
    }

    // The second there starts nothing over.
    let mut trace = Trace::default();
    let c = &mut team[0].connection;
    assert_eq!(
        warp(c, PIRATE_STAGES[1], "", &mut trace).await,
        PIRATE_STAGES[1]
    );
    let mut trace = Trace::default();
    read_for(&mut team[2].connection, &mut trace, 1000).await;
    assert!(trace.countdowns.is_empty(), "{trace:?}");

    // A member gives up: the two left are too few, and land on the way out.
    let trace = told(&mut team[2], talk(PIRATE_ENTRY, GUON_ABOARD)).await;
    assert_eq!(trace.prompts, [(GUON_ABOARD, 1)], "{trace:?}");
    let trace = told(&mut team[2], answer(1, 1, None)).await;
    assert_eq!(trace.warps, [PIRATE_LOBBY], "{trace:?}");
    for player in team[..2].iter_mut() {
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&PIRATE_EXIT)
        })
        .await;
    }

    // Guon there sends them back at once.
    let trace = told(&mut team[0], talk(PIRATE_EXIT, GUON_ABOARD)).await;
    assert_eq!(trace.warps, [PIRATE_LOBBY], "{trace:?}");

    // The one of level 54 never went aboard.
    let mut trace = Trace::default();
    read_for(&mut left_behind.connection, &mut trace, 200).await;
    assert!(!trace.warps.contains(&PIRATE_ENTRY), "{trace:?}");
}
