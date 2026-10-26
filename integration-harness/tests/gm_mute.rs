//! A GM's `!mute`: the account can't chat until the time is up or
//! `!unmute`. The world drops what the player says at once, and the next
//! login tells the client the reason and the end (report b180).

use integration_harness::database::set_gm_level;
use integration_harness::packets::{build_all_chat, build_whisper};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::protocol::login_refusal;
use integration_harness::trace::{barrier, read, send, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::build::world::messaging::Broadcast;
use packet::Packet;
use tokio::time::Duration;

const BLOCKED: &str = "You are currently blocked from chatting.";

fn chat(line: &str) -> Packet {
    build_all_chat(line, 0).unwrap()
}

fn pink(line: impl Into<String>) -> (u8, String) {
    (Broadcast::Pink as u8, line.into())
}

/// The chat block in a login result: the reason, and whether an end
/// follows. After the status, two zero bytes, the result's int and the
/// account id: gender, grade, subgrade, country, the name, a flag, then
/// the reason and the end.
fn login_block(bytes: &[u8]) -> (u8, u64) {
    assert_eq!(bytes[0], 0, "logged in");
    let name_at = 6 + 4 + 4;
    let len = u16::from_le_bytes([bytes[name_at], bytes[name_at + 1]]) as usize;
    let at = name_at + 2 + len + 1;
    let until = u64::from_le_bytes(bytes[at + 1..at + 9].try_into().unwrap());
    (bytes[at], until)
}

#[tokio::test]
async fn a_muted_account_cant_chat_until_let_be() {
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
    set_gm_level(&config.username, 2).await.expect("make a GM");
    let (me, them) = (&config.character_name, &other_config.character_name);

    let mut gm = relogin(&config).await;
    let mut player = relogin(&other_config).await;
    let them_id = player.character_id;
    let g = &mut gm.connection;
    let p = &mut player.connection;

    // Muted, and told.
    let (mut gt, mut pt) = (Trace::default(), Trace::default());
    send(g, chat(&format!("!mute {them} 10 advertising"))).await;
    while pt.broadcasts.is_empty() {
        read(p, &mut pt).await;
    }
    send(g, chat(&format!("!mute {me}"))).await;
    send(g, chat(&format!("!mute {them} 10 spam"))).await;
    barrier(g, "gm done", &mut gt).await;
    assert_eq!(
        gt.broadcasts,
        [
            pink(format!(
                "Blocked {them}'s account from chatting for 10 minutes."
            )),
            pink(format!("{me} is your rank or higher.")),
            pink("Usage: !mute <name> [minutes] [reason]"),
        ]
    );
    assert_eq!(
        pt.broadcasts,
        [pink("A GM blocked you from chatting for 10 minutes.")]
    );

    // What they say goes nowhere.
    let (mut gt, mut pt) = (Trace::default(), Trace::default());
    send(p, chat("hello")).await;
    send(p, build_whisper(me, "psst").unwrap()).await;
    // The player's own barrier would be dropped too.
    while pt.broadcasts.len() < 2 {
        read(p, &mut pt).await;
    }
    barrier(g, "gm done", &mut gt).await;
    assert_eq!(pt.broadcasts, [pink(BLOCKED), pink(BLOCKED)]);
    let said = |t: &Trace| t.chats.iter().any(|&(id, _)| id == them_id);
    assert!(!said(&pt) && !said(&gt), "{:?}", gt.chats);

    // The next login carries the reason (advertising) and the end.
    drop(player);
    let (reason, until) = login_block(&login_refusal(&other_config).await);
    assert_eq!(reason, 2);
    assert!(until > 0);
    let mut player = relogin(&other_config).await;
    let p = &mut player.connection;

    // Let be.
    let (mut gt, mut pt) = (Trace::default(), Trace::default());
    send(g, chat(&format!("!unmute {them}"))).await;
    while pt.broadcasts.is_empty() {
        read(p, &mut pt).await;
    }
    send(g, chat(&format!("!unmute {them}"))).await;
    barrier(g, "gm done", &mut gt).await;
    assert_eq!(
        gt.broadcasts,
        [
            pink(format!("{them}'s account may chat again.")),
            pink(format!("{them}'s account is not blocked from chatting.")),
        ]
    );
    assert_eq!(
        pt.broadcasts,
        [pink(
            "A GM let you chat again. If you still can't, log in again."
        )]
    );
    let mut gt = Trace::default();
    send(p, chat("hello again")).await;
    while !said(&gt) {
        read(g, &mut gt).await;
    }
    assert_eq!(gt.chats, [(them_id, "hello again".to_string())]);

    // Muted while away: the login says so.
    drop(player);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut gt = Trace::default();
    send(g, chat(&format!("!mute {them} 5"))).await;
    barrier(g, "gm done", &mut gt).await;
    assert_eq!(
        gt.broadcasts,
        [pink(format!(
            "Blocked {them}'s account from chatting for 5 minutes."
        ))]
    );
    let (reason, until) = login_block(&login_refusal(&other_config).await);
    assert_eq!((reason, until > 0), (1, true));
    send(g, chat(&format!("!unmute {them}"))).await;
    barrier(g, "gm done", &mut Trace::default()).await;
    assert_eq!(login_block(&login_refusal(&other_config).await), (0, 0));
}
