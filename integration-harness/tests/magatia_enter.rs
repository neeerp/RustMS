//! Romeo and Juliet's party quest, its first slice: Romeo in his room on
//! Magatia's Zenumist side takes exactly four of a party at levels 71 to 85
//! who are there into their own copy of the quest's maps; inside, he lets a
//! member retreat to the room it lets out on, and with only three left the
//! run ends for everyone. Juliet does the same from the Alcadno side, and
//! when the time runs out everyone lands on its own exit, whose portal
//! leads back to her room. The harness world runs events 60 times as fast
//! (`RUSTMS_EVENT_SCALE`), so a run lasts 45 seconds.

use game_data::server_content::keys;
use integration_harness::events::{armed_in, party, talk, told};
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{EventKind, MagatiaSide, ALCADNO_LOBBY, ZENUMIST_LOBBY};
use net::npc_scripts::{JULIET, ROMEO};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

/// Somewhere outside Magatia's rooms to step away to.
const ELSEWHERE: i32 = 261_000_000;

/// `key`, with who may play.
fn terms(key: &str, side: MagatiaSide) -> String {
    let event = EventKind::Magatia(side).event();
    let (team, levels) = (&event.team, &event.levels);
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

/// Four at level 75 in `lobby`, partied up.
async fn four_in(lobby: i32) -> Vec<WorldSession> {
    let mut team = Vec::new();
    for _ in 0..4 {
        team.push(armed_in(lobby, 75).await);
    }
    party(&mut team).await;
    team
}

/// The leader asks `host` to go in until no party before them is in the
/// way; everyone then arrives on the entry, on the clock.
async fn go_in(team: &mut [WorldSession], side: MagatiaSide, host: i32) {
    let event = EventKind::Magatia(side).event();
    let mut seen = None;
    for _ in 0..60 {
        told(&mut team[0], talk(event.lobby, host)).await;
        let trace = told(&mut team[0], answer(4, 1, Some(0))).await;
        if !trace.prompts.iter().any(|&(from, _)| from == host) {
            seen = Some(trace);
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    assert!(seen.is_some(), "{host} never let the party in");
    for player in team.iter_mut() {
        let mut trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&event.entry) && !t.countdowns.is_empty()
        })
        .await;
    }
}

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// `player` takes the exit's way out and lands in `lobby`.
async fn home(player: &mut WorldSession, lobby: i32) {
    send(&mut player.connection, enter("out00")).await;
    let mut trace = Trace::default();
    until(&mut player.connection, &mut trace, |t| !t.warps.is_empty()).await;
    assert_eq!(trace.warps, [lobby], "{trace:?}");
}

#[tokio::test]
async fn romeo_takes_four_in_and_a_retreat_ends_the_run_for_all() {
    let side = MagatiaSide::Zenumist;
    let event = EventKind::Magatia(side).event();
    let mut team = four_in(ZENUMIST_LOBBY).await;

    // Only the leader may take the party in, and only with all four there.
    told(&mut team[1], talk(ZENUMIST_LOBBY, ROMEO)).await;
    let trace = told(&mut team[1], answer(4, 1, Some(0))).await;
    let not_leader = net::content::text().get(keys::MAGATIA_NOT_LEADER, &[]);
    assert!(trace.prompt_texts[0].contains(&not_leader), "{trace:?}");
    let mut trace = Trace::default();
    let away = &mut team[3].connection;
    assert_eq!(warp(away, ELSEWHERE, "", &mut trace).await, ELSEWHERE);
    told(&mut team[0], talk(ZENUMIST_LOBBY, ROMEO)).await;
    let trace = told(&mut team[0], answer(4, 1, Some(0))).await;
    let not_eligible = terms(keys::MAGATIA_NOT_ELIGIBLE, side);
    assert!(trace.prompt_texts[0].contains(&not_eligible), "{trace:?}");
    let mut trace = Trace::default();
    let back = &mut team[3].connection;
    assert_eq!(
        warp(back, ZENUMIST_LOBBY, "", &mut trace).await,
        ZENUMIST_LOBBY
    );

    go_in(&mut team, side, ROMEO).await;

    // Romeo inside asks whether a member would rather retreat; one does,
    // and with three left the others follow to the exit.
    let trace = told(&mut team[3], talk(event.entry, ROMEO)).await;
    assert_eq!(trace.prompts, [(ROMEO, 1)], "{trace:?}");
    let retreat = net::content::text().get(keys::MAGATIA_ROMEO_RETREAT, &[]);
    assert_eq!(trace.prompt_texts, [retreat]);
    assert_eq!(
        told(&mut team[3], answer(1, 1, None)).await.warps,
        [event.exit]
    );
    for player in team[..3].iter_mut() {
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&event.exit)
        })
        .await;
    }
    home(&mut team[3], ZENUMIST_LOBBY).await;
}

#[tokio::test]
async fn juliet_takes_four_in_and_the_time_running_out_sends_them_to_her_exit() {
    let side = MagatiaSide::Alcadno;
    let event = EventKind::Magatia(side).event();
    let mut team = four_in(ALCADNO_LOBBY).await;
    go_in(&mut team, side, JULIET).await;

    for player in team.iter_mut() {
        let mut trace = Trace::default();
        for _ in 0..60 {
            if trace.warps.contains(&event.exit) {
                break;
            }
            read_for(&mut player.connection, &mut trace, 1_000).await;
        }
        assert!(trace.warps.contains(&event.exit), "{trace:?}");
    }
    // Her exit's portal has Romeo's script, but leads back to her room.
    home(&mut team[0], ALCADNO_LOBBY).await;
}
