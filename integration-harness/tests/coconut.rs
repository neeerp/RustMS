//! Coconut Season: players entering the field are put on a team, a GM's
//! `/start` and `/reset` start and stop the game while a player's do
//! nothing, the nuts a player brings down score for their team, and at the
//! end the winners and the losers get their effects and are warped out.
//! The harness world's event scale makes the 300-second game 5 seconds.

use integration_harness::database::set_gm_level;
use integration_harness::packets::build_all_chat;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{barrier, read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const FIELD: i32 = 109_080_000;
const WINNERS: i32 = 109_050_000;
const EXIT: i32 = 109_050_001;

fn chat(line: &str) -> Packet {
    build_all_chat(line, 0).unwrap()
}

/// COCONUT: a hit on nut `index`.
fn hit(index: u16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::Coconut as i16).unwrap();
    p.write_short(index as i16).unwrap();
    p.write_short(0).unwrap();
    p
}

fn bodies(trace: &Trace, op: SendOpcode) -> Vec<Vec<u8>> {
    let op = op as i16;
    trace
        .coconut
        .iter()
        .filter(|(o, _)| *o == op)
        .map(|(_, body)| body.clone())
        .collect()
}

/// COCONUT_HIT bodies: (index, delay, state).
fn hits(trace: &Trace) -> Vec<(i16, u16, u8)> {
    bodies(trace, SendOpcode::CoconutHit)
        .iter()
        .map(|b| {
            (
                i16::from_le_bytes([b[0], b[1]]),
                u16::from_le_bytes([b[2], b[3]]),
                b[4],
            )
        })
        .collect()
}

/// COCONUT_SCORE bodies: (Maple, Story).
fn scores(trace: &Trace) -> Vec<(u16, u16)> {
    bodies(trace, SendOpcode::CoconutScore)
        .iter()
        .map(|b| {
            (
                u16::from_le_bytes([b[0], b[1]]),
                u16::from_le_bytes([b[2], b[3]]),
            )
        })
        .collect()
}

fn teams(trace: &Trace) -> Vec<Vec<u8>> {
    bodies(trace, SendOpcode::FieldSpecificData)
}

#[tokio::test]
async fn a_gm_runs_a_coconut_game_and_the_team_that_brings_down_more_wins() {
    let config = load_harness_config_or_fail().await;
    let gm_id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    let other_config = load_harness_config_or_fail().await;
    let player_id = login_to_world_session(&other_config)
        .await
        .expect("second login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_gm_level(&config.username, 1).await.expect("make a GM");

    let mut gm = relogin(&config).await;
    let mut player = relogin(&other_config).await;
    let (g, p) = (&mut gm.connection, &mut player.connection);
    let (mut gt, mut pt) = (Trace::default(), Trace::default());

    // The first in is on Team Maple, the second on Team Story; each is
    // told the score, and sees the other's team.
    assert_eq!(warp(g, FIELD, "", &mut gt).await, FIELD);
    barrier(g, "gm in", &mut gt).await;
    assert_eq!(teams(&gt), [vec![0]]);
    assert_eq!(scores(&gt), [(0, 0)]);
    // Forget seeing the player in the login map they both started in.
    gt.players.clear();
    assert_eq!(warp(p, FIELD, "", &mut pt).await, FIELD);
    barrier(p, "player in", &mut pt).await;
    assert_eq!(teams(&pt), [vec![1]]);
    assert_eq!(scores(&pt), [(0, 0)]);
    let gm_seen = pt.players.iter().find(|s| s.character_id == gm_id);
    assert_eq!(gm_seen.expect("the GM in the field").team, 0);
    until(g, &mut gt, |t| {
        t.players.iter().any(|s| s.character_id == player_id)
    })
    .await;
    let player_seen = gt.players.iter().find(|s| s.character_id == player_id);
    assert_eq!(player_seen.unwrap().team, 1);

    // A player's /start does nothing, and isn't said.
    let mut quiet = Trace::default();
    send(p, chat("/start")).await;
    barrier(p, "after start", &mut quiet).await;
    assert!(quiet.coconut.is_empty() && quiet.countdowns.is_empty());

    // A GM's starts the game and their /reset stops it.
    send(g, chat("/start")).await;
    until(p, &mut pt, |t| !t.countdowns.is_empty()).await;
    assert_eq!(hits(&pt).last(), Some(&(-1, 5000, 0)));
    assert_eq!(scores(&pt).last(), Some(&(0, 0)));
    assert_eq!(pt.countdowns, [5]);
    send(g, chat("/reset")).await;
    until(p, &mut pt, |t| t.countdowns.len() == 2).await;
    assert_eq!(hits(&pt).last(), Some(&(-1, 0, 0)));
    assert_eq!(pt.countdowns[1], 0);

    // In a new game the player hits ten nuts in turn until one falls,
    // a point for Team Story.
    send(g, chat("/start")).await;
    until(p, &mut pt, |t| t.countdowns.len() == 3).await;
    'hunt: for _ in 0..12 {
        for nut in 0..10 {
            send(p, hit(nut)).await;
        }
        read_for(p, &mut pt, 800).await;
        if scores(&pt).iter().any(|&(_, story)| story > 0) {
            break 'hunt;
        }
    }
    let (maple, story) = *scores(&pt).last().unwrap();
    assert!(maple == 0 && story > 0, "no nut fell: {:?}", hits(&pt));
    assert!(hits(&pt)
        .iter()
        .any(|&(index, _, state)| index >= 0 && state == 3));
    assert!(hits(&pt)
        .iter()
        .any(|&(index, _, state)| index >= 0 && state == 1));

    // Story wins: the player gets the win effect and goes to the winners'
    // map, the GM the lose effect and the way out.
    until(p, &mut pt, |t| t.warps.contains(&WINNERS)).await;
    assert!(pt
        .field_effects
        .iter()
        .any(|(_, path)| path == "event/coconut/victory"));
    until(g, &mut gt, |t| t.warps.contains(&EXIT)).await;
    assert!(gt
        .field_effects
        .iter()
        .any(|(_, path)| path == "event/coconut/lose"));
}
