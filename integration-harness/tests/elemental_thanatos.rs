//! Elemental Thanatos, on the Fire/Poison and Ice/Lightning mages' way to
//! the third job's Mystic Trial: Flo on El Nath's Tower 1st floor takes a
//! party of two magicians of level 100 or more, its leader on the quest,
//! into the room of the two elementals, on the clock; when the time runs
//! out, both land back by Flo. The harness world runs events 60 times as
//! fast (`RUSTMS_EVENT_SCALE`), so the fight lasts 20 seconds.

use game_data::server_content::keys;
use integration_harness::database::{set_job, set_quest, start_quest};
use integration_harness::events::{party, talk, told, waiting_as};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, until_within, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{ELEMENTAL, ELEMENTALS, ELEMENTAL_LOBBY, ELEMENTAL_ROOM};
use net::npc_scripts::FLO;
use tokio::time::Duration;

/// A new level-120 `job` mage, on the first of Flo's quests if `quest`,
/// past its second, logged in by Flo.
async fn mage(job: i16, quest: Option<(i16, i16)>) -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, job, 120, 5_000).await.unwrap();
    if let Some((on, past)) = quest {
        start_quest(id, on).await.unwrap();
        set_quest(id, past, 2, "").await.unwrap();
    }
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(
        warp(c, ELEMENTAL_LOBBY, "", &mut seen).await,
        ELEMENTAL_LOBBY
    );
    session
}

/// `key`, with who may come.
fn terms(key: &str) -> String {
    let (team, levels) = (&ELEMENTAL.team, &ELEMENTAL.levels);
    net::content::text().get(
        key,
        &[
            ("least", &team.start().to_string()),
            ("most", &team.end().to_string()),
            ("low", &levels.start().to_string()),
            ("high", &levels.end().to_string()),
        ],
    )
}

#[tokio::test]
async fn flo_has_nothing_for_one_off_the_quest_and_takes_no_warrior() {
    // Off the quest, Flo has no reason to say more.
    let mut loner = mage(212, None).await;
    let trace = told(&mut loner, talk(ELEMENTAL_LOBBY, FLO)).await;
    let no_reason = net::content::text().get(keys::ELEMENTAL_NO_REASON, &[]);
    assert!(trace.prompt_texts[0].contains(&no_reason), "{trace:?}");

    // A warrior doesn't make the second of the two.
    let mut team = vec![
        mage(212, Some((6225, 6226))).await,
        waiting_as(ELEMENTAL_LOBBY, 120, 112).await,
    ];
    party(&mut team).await;
    assert_eq!(
        told(&mut team[0], talk(ELEMENTAL_LOBBY, FLO)).await.prompts,
        [(FLO, 4)]
    );
    let trace = told(&mut team[0], answer(4, 1, Some(0))).await;
    let not_eligible = terms(keys::ELEMENTAL_NOT_ELIGIBLE);
    assert!(trace.prompt_texts[0].contains(&not_eligible), "{trace:?}");
}

#[tokio::test]
async fn flo_takes_two_mages_to_the_elementals_and_back_when_the_time_runs_out() {
    let mut team = vec![mage(222, Some((6315, 6316))).await, mage(212, None).await];
    party(&mut team).await;

    // The leader takes the party in once any party before them is out.
    let mut seen = None;
    for _ in 0..30 {
        told(&mut team[0], talk(ELEMENTAL_LOBBY, FLO)).await;
        let trace = told(&mut team[0], answer(4, 1, Some(0))).await;
        if !trace.prompts.iter().any(|&(from, _)| from == FLO) {
            seen = Some(trace);
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    assert!(seen.is_some(), "Flo never let the party in");

    // Both land in the room on the clock, with the two elementals there.
    for player in team.iter_mut() {
        let mut trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&ELEMENTAL_ROOM) && !t.countdowns.is_empty()
        })
        .await;
        until(&mut player.connection, &mut trace, |t| {
            ELEMENTALS
                .iter()
                .all(|&elemental| !t.field.alive(elemental).is_empty())
        })
        .await;
    }

    // The time runs out, and both land back by Flo.
    for player in team.iter_mut() {
        let mut trace = Trace::default();
        until_within(
            &mut player.connection,
            &mut trace,
            Duration::from_secs(40),
            |t| t.warps.contains(&ELEMENTAL_LOBBY),
        )
        .await;
    }
}
