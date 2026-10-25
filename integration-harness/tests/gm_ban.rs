//! Blocking an account: a GM's `!ban` or the client's `/block` disconnects
//! the player, and the login refuses them (result 2) with the reason and
//! the end the client shows, until `!unban`. `!dc` only disconnects.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::set_gm_level;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{barrier, read, send, Trace};
use integration_harness::{login_refusal, login_to_world_session, relogin};
use net::codec::time::{expiration, filetime};
use net::packet::build::world::messaging::Broadcast;
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::SystemTime;
use tokio::time::{timeout, Duration};

/// FILETIME's ticks in a day.
const DAY: i64 = 864_000_000_000;

fn chat(line: &str) -> Packet {
    integration_harness::packets::build_all_chat(line, 0).unwrap()
}

/// `/block NAME`, as the client sends it once its dialog is filled in.
fn slash_block(name: &str, reason: u8, days: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::AdminCommand as i16).unwrap();
    p.write_byte(0x04).unwrap();
    p.write_str_with_length(name).unwrap();
    p.write_byte(reason).unwrap();
    p.write_int(days).unwrap();
    p.write_str_with_length("from the dialog").unwrap();
    p
}

/// `/ban NAME`.
fn slash_ban(name: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::AdminCommand as i16).unwrap();
    p.write_byte(0x03).unwrap();
    p.write_str_with_length(name).unwrap();
    p
}

/// Read until the server closes `c`.
async fn disconnected(c: &mut MapleTestConnection) {
    loop {
        match timeout(Duration::from_secs(5), c.read_packet("disconnect")).await {
            Ok(Ok(_)) => {}
            Ok(Err(_)) => return,
            Err(_) => panic!("still connected"),
        }
    }
}

/// The next ADMIN_RESULT's bytes after its opcode.
async fn admin_result(c: &mut MapleTestConnection) -> Vec<u8> {
    let mut trace = Trace::default();
    loop {
        let p = read(c, &mut trace).await;
        if p.opcode() == SendOpcode::AdminResult as i16 {
            return p.bytes[2..].to_vec();
        }
    }
}

/// A refused login's reason and end.
fn refusal(bytes: &[u8]) -> (u8, i64) {
    assert_eq!(bytes[..6], [2, 0, 0, 0, 0, 0], "{bytes:02x?}");
    let end = i64::from_le_bytes(bytes[7..15].try_into().unwrap());
    (bytes[6], end)
}

fn pink(line: String) -> (u8, String) {
    (Broadcast::Pink as u8, line)
}

#[tokio::test]
async fn a_blocked_player_is_disconnected_and_refused_until_unblocked() {
    let config = load_harness_config_or_fail().await;
    login_to_world_session(&config)
        .await
        .expect("first login failed");
    drop(relogin(&config).await);
    let other_config = load_harness_config_or_fail().await;
    login_to_world_session(&other_config)
        .await
        .expect("second login failed");
    drop(relogin(&other_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_gm_level(&config.username, 3).await.expect("make a GM");
    let (me, them) = (&config.character_name, &other_config.character_name);

    // `!ban` for a week, as a bot.
    let mut gm = relogin(&config).await;
    let mut player = relogin(&other_config).await;
    let g = &mut gm.connection;
    let mut gt = Trace::default();
    send(
        g,
        chat(&format!("!ban {} 7 bot farming", them.to_lowercase())),
    )
    .await;
    disconnected(&mut player.connection).await;
    barrier(g, "gm done", &mut gt).await;
    assert_eq!(
        gt.broadcasts,
        [pink(format!("Blocked {them}'s account for 7 days."))]
    );
    let week = filetime(SystemTime::now()) + 7 * DAY;
    let (reason, end) = refusal(&login_refusal(&other_config).await);
    assert_eq!(reason, 2, "BOT");
    assert!((end - week).abs() < DAY, "{end} {week}");

    // Lifted, they log in.
    let mut gt = Trace::default();
    send(g, chat(&format!("!unban {them}"))).await;
    send(g, chat(&format!("!unban {them}"))).await;
    send(g, chat(&format!("!ban {me}"))).await;
    send(g, chat("!dc nobody_here")).await;
    send(g, chat(&format!("!dc {them}"))).await;
    barrier(g, "gm done", &mut gt).await;
    assert_eq!(
        gt.broadcasts,
        [
            pink(format!("Unblocked {them}'s account.")),
            pink(format!("{them}'s account is not blocked.")),
            pink(format!("{me} is your rank or higher.")),
            pink("There is no character named nobody_here.".to_string()),
            pink(format!("{them} is not online.")),
        ]
    );

    // `!dc` only disconnects.
    let mut player = relogin(&other_config).await;
    send(g, chat(&format!("!dc {them}"))).await;
    disconnected(&mut player.connection).await;

    // `/ban` for good, with no reason; `/block` replaces it.
    let mut player = relogin(&other_config).await;
    send(g, slash_ban("nobody_here")).await;
    assert_eq!(admin_result(g).await, [6, 1]);
    send(g, slash_ban(them)).await;
    assert_eq!(admin_result(g).await, [4, 0]);
    disconnected(&mut player.connection).await;
    assert_eq!(
        refusal(&login_refusal(&other_config).await),
        (0, expiration(None))
    );
    // The client never sends no days; the server ignores it.
    send(g, slash_block(them, 1, 0)).await;
    send(g, slash_block(them, 1, 2)).await;
    assert_eq!(admin_result(g).await, [4, 0]);
    let (reason, end) = refusal(&login_refusal(&other_config).await);
    assert_eq!(reason, 1, "HACK");
    let two_days = filetime(SystemTime::now()) + 2 * DAY;
    assert!((end - two_days).abs() < DAY, "{end} {two_days}");

    // A level 2 GM may not.
    set_gm_level(&config.username, 2)
        .await
        .expect("a lesser GM");
    drop(gm);
    let mut gm = relogin(&config).await;
    let g = &mut gm.connection;
    let mut gt = Trace::default();
    send(g, chat(&format!("!unban {them}"))).await;
    barrier(g, "gm done", &mut gt).await;
    assert_eq!(
        gt.broadcasts,
        [pink("Unknown command. Type !help for a list.".to_string())]
    );
    refusal(&login_refusal(&other_config).await);
}
