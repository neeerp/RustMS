//! Rock, Paper, Scissors (RPS_ACTION, RPS_GAME): the admin in Henesys opens
//! the window, a game costs its fee, and a won streak is kept as a
//! certificate. The admin's hands are random, so each game is played until
//! it is won, checking every round on the way.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::set_meso;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, stat, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const HENESYS: i32 = 100_000_000;
const ADMIN: i32 = 9000019;
const FEE: i64 = 1000;
const CONSOLATION: i64 = 500;
const FIRST_CERTIFICATE: i32 = 4031332;
const ETC: u8 = 4;

const ROCK: u8 = 0;
const PAPER: u8 = 1;
const SCISSORS: u8 = 2;

const NO_MESOS: u8 = 0x06;
const OPEN: u8 = 0x08;
const START: u8 = 0x09;
const TIME_OVER: u8 = 0x0a;
const ROUND: u8 = 0x0b;
const NEXT_ROUND: u8 = 0x0c;
const CLOSE: u8 = 0x0d;

fn talk(object_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(object_id).unwrap();
    p
}

fn action(mode: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::RpsAction as i16).unwrap();
    p.write_byte(mode).unwrap();
    p
}

fn choose(hand: u8) -> Packet {
    let mut p = action(1);
    p.write_byte(hand).unwrap();
    p
}

/// Talk to the admin; the window opens with his notice.
async fn open(c: &mut MapleTestConnection, seen: &mut Trace) {
    until(c, seen, |t| t.field.npc(ADMIN).is_some()).await;
    let admin = seen.field.npc(ADMIN).unwrap();
    let t = step(c, "rps-open", talk(admin)).await;
    let mut body = vec![OPEN];
    body.extend(ADMIN.to_le_bytes());
    assert_eq!(t.rps, [body]);
}

/// In a window just opened, pay for a game and play rock until a round is
/// won, retrying lost games. `meso` follows the fee and the consolation.
async fn win_a_round(c: &mut MapleTestConnection, meso: &mut i64) {
    let mut start = action(0);
    // Only a window's first loss is consoled: after that the client holds
    // the lost streak.
    let mut consoled = false;
    for _ in 0..40 {
        let t = step(c, "rps-start", start).await;
        assert_eq!(t.rps, [[START]]);
        *meso -= FEE;
        assert_eq!(t.stat(stat::MESO), Some(*meso));
        loop {
            let t = step(c, "rps-rock", choose(ROCK)).await;
            let [body] = &t.rps[..] else {
                panic!("one round: {:?}", t.rps);
            };
            assert_eq!(body[0], ROUND);
            match (body[1], body[2] as i8) {
                // A draw keeps the streak, and the client plays again.
                (ROCK, 0) => assert!(t.stat(stat::MESO).is_none()),
                (SCISSORS, 1) => {
                    assert!(t.added.is_empty(), "nothing until leaving");
                    return;
                }
                (PAPER, -1) if !consoled => {
                    *meso += CONSOLATION;
                    assert_eq!(t.stat(stat::MESO), Some(*meso));
                    consoled = true;
                    break;
                }
                (PAPER, -1) => {
                    assert!(t.stat(stat::MESO).is_none());
                    break;
                }
                round => panic!("rock against {round:?}"),
            }
        }
        start = action(5);
    }
    panic!("no round won in 40 games");
}

#[tokio::test]
async fn a_won_streak_is_kept_as_a_certificate() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(id, 999).await.expect("set mesos");

    // Too poor for the fee: the window shows Start again.
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut seen = Trace::default();
    assert_eq!(warp(c, HENESYS, "", &mut seen).await, HENESYS);
    open(c, &mut seen).await;
    let t = step(c, "rps-broke", action(0)).await;
    assert_eq!(t.rps, [[NO_MESOS]]);
    assert!(t.stat(stat::MESO).is_none());
    let t = step(c, "rps-unstarted", choose(ROCK)).await;
    assert!(t.rps.is_empty());
    let t = step(c, "rps-leave-broke", action(4)).await;
    assert_eq!(t.rps, [[CLOSE]]);
    drop(session);

    tokio::time::sleep(Duration::from_millis(500)).await;
    set_meso(id, 100_000).await.expect("set mesos");
    let mut meso = 100_000;
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut seen = Trace::default();
    assert_eq!(warp(c, HENESYS, "", &mut seen).await, HENESYS);
    open(c, &mut seen).await;

    // A win risked on the next round and lost to the clock leaves nothing.
    win_a_round(c, &mut meso).await;
    let t = step(c, "rps-continue", action(3)).await;
    assert_eq!(t.rps, [[NEXT_ROUND]]);
    let t = step(c, "rps-time-up", action(2)).await;
    assert_eq!(t.rps, [[TIME_OVER]]);
    assert!(t.stat(stat::MESO).is_none());
    let t = step(c, "rps-leave-empty", action(4)).await;
    assert_eq!(t.rps, [[CLOSE]]);
    assert!(t.added.is_empty());

    // The window is gone until the admin is spoken to again.
    let t = step(c, "rps-closed", action(0)).await;
    assert!(t.rps.is_empty());

    // A win kept: leaving takes the certificate for one win.
    open(c, &mut seen).await;
    win_a_round(c, &mut meso).await;
    let t = step(c, "rps-keep", action(4)).await;
    assert_eq!(t.rps, [[CLOSE]]);
    assert!(
        matches!(t.added[..], [(ETC, _, FIRST_CERTIFICATE)]),
        "{:?}",
        t.added
    );
    assert!(t.stat(stat::MESO).is_none());
    send(c, action(4)).await;
}
