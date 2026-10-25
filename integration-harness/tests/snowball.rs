//! Snowball: players entering the field are put on a team and told its
//! state, a GM's `/start` and `/reset` start and stop the game while a
//! player's do nothing, a team's hits roll its own ball and not the
//! other's, a player walking into a ball is thrown back, and at the end
//! the team whose ball is further on wins and everyone is warped out. A
//! player who walks into a snowman is hurt as by a trap.
//! The harness world's event scale makes the 600-second game 10 seconds.

use integration_harness::database::{set_gm_level, set_hp};
use integration_harness::packets::build_all_chat;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{barrier, read_for, send, stat, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const FIELD: i32 = 109_060_000;
const WINNERS: i32 = 109_050_000;
const EXIT: i32 = 109_050_001;

fn chat(line: &str) -> Packet {
    build_all_chat(line, 0).unwrap()
}

/// SNOWBALL: a hit on `what` (ball 0, ball 1, snowman 0, snowman 1).
fn hit(what: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::Snowball as i16).unwrap();
    p.write_byte(what).unwrap();
    p.write_short(10).unwrap();
    p.write_short(300).unwrap();
    p
}

fn knock() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::LeftKnockback as i16).unwrap();
    p
}

/// TAKE_DAMAGE as a v83 client sends it from a trap, or a snowman (A 574df1).
fn trap_damage(damage: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TakeDamage as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(-3i8 as u8).unwrap();
    p.write_byte(0).unwrap();
    p.write_int(damage).unwrap();
    p.write_short(0).unwrap();
    p.write_byte(0).unwrap();
    p
}

fn bodies(trace: &Trace, op: SendOpcode) -> Vec<Vec<u8>> {
    let op = op as i16;
    trace
        .snowball
        .iter()
        .filter(|(o, _)| *o == op)
        .map(|(_, body)| body.clone())
        .collect()
}

/// SNOWBALL_STATE bodies: (state, ball 0's step, ball 1's step).
fn states(trace: &Trace) -> Vec<(u8, i16, i16)> {
    bodies(trace, SendOpcode::SnowballState)
        .iter()
        .map(|b| {
            (
                b[0],
                i16::from_le_bytes([b[9], b[10]]),
                i16::from_le_bytes([b[12], b[13]]),
            )
        })
        .collect()
}

/// HIT_SNOWBALL bodies: (what, damage).
fn hits(trace: &Trace) -> Vec<(u8, u16)> {
    bodies(trace, SendOpcode::HitSnowball)
        .iter()
        .map(|b| (b[0], u16::from_le_bytes([b[1], b[2]])))
        .collect()
}

#[tokio::test]
async fn a_gm_runs_a_snowball_game_and_the_ball_further_on_wins() {
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

    // The first in is on Team Story, the second on Team Maple; each is
    // told the field's state with the damages its hits show.
    assert_eq!(warp(g, FIELD, "", &mut gt).await, FIELD);
    barrier(g, "gm in", &mut gt).await;
    let entry = bodies(&gt, SendOpcode::SnowballState);
    assert_eq!(entry.len(), 1);
    assert_eq!(
        entry[0][..15],
        [0, 100, 0, 0, 0, 100, 0, 0, 0, 0, 0, 8, 0, 0, 8]
    );
    assert_eq!(entry[0][15..], [10, 0, 15, 0, 45, 0]);
    gt.players.clear();
    assert_eq!(warp(p, FIELD, "", &mut pt).await, FIELD);
    barrier(p, "player in", &mut pt).await;
    let gm_seen = pt.players.iter().find(|s| s.character_id == gm_id);
    assert_eq!(gm_seen.expect("the GM in the field").team, 0);
    until(g, &mut gt, |t| {
        t.players.iter().any(|s| s.character_id == player_id)
    })
    .await;
    let player_seen = gt.players.iter().find(|s| s.character_id == player_id);
    assert_eq!(player_seen.unwrap().team, 1);

    // Before a game, and on a player's /start, hits do nothing.
    let mut quiet = Trace::default();
    send(p, chat("/start")).await;
    send(p, hit(1)).await;
    barrier(p, "after start", &mut quiet).await;
    assert!(quiet.snowball.is_empty() && quiet.countdowns.is_empty());

    // A GM's starts the game and their /reset stops it.
    send(g, chat("/start")).await;
    until(p, &mut pt, |t| !t.countdowns.is_empty()).await;
    assert_eq!(states(&pt).last(), Some(&(1, 0, 0)));
    assert_eq!(pt.countdowns, [10]);
    send(g, chat("/reset")).await;
    until(p, &mut pt, |t| t.countdowns.len() == 2).await;
    assert_eq!(states(&pt).last(), Some(&(0, 0, 0)));
    assert_eq!(pt.countdowns[1], 0);

    // In a new game the player's three hits roll Maple's ball one step;
    // a hit on Story's ball is refused.
    send(g, chat("/start")).await;
    until(p, &mut pt, |t| t.countdowns.len() == 3).await;
    send(p, hit(0)).await;
    for _ in 0..3 {
        send(p, hit(1)).await;
        read_for(p, &mut pt, 550).await;
    }
    assert_eq!(hits(&pt), [(1, 10), (1, 10), (1, 10)]);
    assert_eq!(states(&pt).last(), Some(&(1, 0, 1)));
    // The GM sees it roll too.
    until(g, &mut gt, |t| states(t).last() == Some(&(1, 0, 1))).await;

    // Walking into a ball throws the player back.
    send(p, knock()).await;
    until(p, &mut pt, |t| {
        !bodies(t, SendOpcode::LeftKnockBack).is_empty()
    })
    .await;

    // Maple wins: Story's ball fades, the player goes to the winners' map
    // and the GM the way out.
    until(p, &mut pt, |t| t.warps.contains(&WINNERS)).await;
    assert!(states(&pt).iter().any(|&(state, _, _)| state == 2));
    until(g, &mut gt, |t| t.warps.contains(&EXIT)).await;
}

#[tokio::test]
async fn a_snowman_hurts_a_player_who_walks_into_it() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_hp(id, 1000, 1000).await.expect("set HP");
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, FIELD, "", &mut t).await, FIELD);
    barrier(c, "in", &mut t).await;

    // A snowman's 30 to 40 is taken: the field's ice traps do up to 55.
    send(c, trap_damage(35)).await;
    until(c, &mut t, |t| t.stat(stat::HP) == Some(965)).await;

    // More than the traps do is refused.
    let updates = t.stat_updates.len();
    send(c, trap_damage(56)).await;
    barrier(c, "refused", &mut t).await;
    assert_eq!(t.stat_updates.len(), updates);
}
