//! Guild alliances (ALLIANCE_OPERATION): at Lenario the leader of a party
//! of two guild masters on the map founds one, for a price, once named; a
//! bad name is asked for again. The alliance's master grows it a guild at a
//! time, for a price, and invites the masters of other guilds, online
//! anywhere, who decline (DENY_ALLIANCE_REQUEST) or join. Alliance chat and
//! the notice and titles reach the other guilds, who hear who logs in or
//! out; a guild leaves, and the master disbands the alliance at Lenario.

use integration_harness::database::{found_guild, set_meso};
use integration_harness::events::{party, talk};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, stat, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::io::Cursor;
use tokio::time::Duration;

const GUILD_HQ: i32 = 200_000_301;
const LENARIO: i32 = 2010009;
const CREATE_COST: i32 = 2_000_000;
const GROW_COST: i32 = 1_000_000;
// Lenario's menu.
const CREATE: i32 = 2;
const GROW: i32 = 3;
const DISBAND: i32 = 4;

const INVITE: u8 = 0x03;
const MEMBER_ONLINE: u8 = 0x0e;
const INFO: u8 = 0x0f;
const GUILD_LEFT: u8 = 0x10;
const GUILD_JOINED: u8 = 0x12;
const CHANGED: u8 = 0x17;
const TITLES_CHANGED: u8 = 0x1a;
const NOTICE_CHANGED: u8 = 0x1c;
const DISBANDED: u8 = 0x1d;
/// BROADCAST_MSG's pink text.
const PINK: u8 = 5;

fn operation(op: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::AllianceOperation as i16).unwrap();
    p.write_byte(op).unwrap();
    p
}

fn invite(guild: &str) -> Packet {
    let mut p = operation(3);
    p.write_str_with_length(guild).unwrap();
    p
}

fn accept(alliance_id: i32, guild: &str) -> Packet {
    let mut p = operation(4);
    p.write_int(alliance_id).unwrap();
    p.write_str_with_length(guild).unwrap();
    p
}

fn decline(inviter: &str, guild: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::DenyAllianceRequest as i16)
        .unwrap();
    p.write_byte(0x16).unwrap();
    p.write_str_with_length(inviter).unwrap();
    p.write_str_with_length(guild).unwrap();
    p
}

fn alliance_chat(to: &[i32], text: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::MultiChat as i16).unwrap();
    p.write_byte(3).unwrap();
    p.write_byte(to.len() as u8).unwrap();
    for &id in to {
        p.write_int(id).unwrap();
    }
    p.write_str_with_length(text).unwrap();
    p
}

fn type_text(text: &str) -> Packet {
    let mut p = answer(2, 1, None);
    p.write_str_with_length(text).unwrap();
    p
}

/// The alliance an op carries: (id, name, guild ids, capacity, notice).
fn alliance(op: &[u8]) -> (i32, String, Vec<i32>, i32, String) {
    let mut r = Cursor::new(&op[1..]);
    let id = r.read_int().unwrap();
    let name = r.read_str_with_length().unwrap();
    for _ in 0..5 {
        r.read_str_with_length().unwrap();
    }
    let count = r.read_byte().unwrap();
    let guilds = (0..count).map(|_| r.read_int().unwrap()).collect();
    let capacity = r.read_int().unwrap();
    (
        id,
        name,
        guilds,
        capacity,
        r.read_str_with_length().unwrap(),
    )
}

/// The first alliance op in `mode` in `t`, reading on until it comes:
/// news from the world directory may follow a step's barrier.
async fn op_in(session: &mut WorldSession, mut t: Trace, mode: u8) -> Vec<u8> {
    until(&mut session.connection, &mut t, |t| {
        t.alliance_ops.iter().any(|op| op[0] == mode)
    })
    .await;
    t.alliance_ops.into_iter().find(|op| op[0] == mode).unwrap()
}

/// The alliance op in `mode` `session` gets next.
async fn next_op(session: &mut WorldSession, mode: u8) -> Vec<u8> {
    op_in(session, Trace::default(), mode).await
}

/// The next pink line `session` gets.
async fn next_pink(session: &mut WorldSession) {
    let mut t = Trace::default();
    until(&mut session.connection, &mut t, |t| {
        t.broadcasts.iter().any(|(kind, _)| *kind == PINK)
    })
    .await;
}

/// A new character, logged out, master of a guild named after them.
async fn master(config: &HarnessConfig) -> i32 {
    let session = login_to_world_session(config).await.expect("first login");
    let id = session.character_id;
    drop(session);
    drop(relogin(config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    found_guild(id, &config.character_name)
        .await
        .expect("guild");
    id
}

/// Logged in again, at the guild headquarters.
async fn at_hq(config: &HarnessConfig) -> WorldSession {
    let mut session = relogin(config).await;
    let mut t = Trace::default();
    assert_eq!(
        warp(&mut session.connection, GUILD_HQ, "", &mut t).await,
        GUILD_HQ
    );
    read_for(&mut session.connection, &mut t, 300).await;
    session
}

/// Pick `choice` from Lenario's menu; what follows.
async fn lenario_menu(session: &mut WorldSession, choice: i32) -> Trace {
    let m = session.character_name.clone();
    let c = &mut session.connection;
    let t = step(c, &format!("{m} talk"), talk(GUILD_HQ, LENARIO)).await;
    assert_eq!(t.prompts, [(LENARIO, 4)]);
    step(
        c,
        &format!("{m} picks {choice}"),
        answer(4, 1, Some(choice)),
    )
    .await
}

async fn yes(session: &mut WorldSession) -> Trace {
    let m = session.character_name.clone();
    step(
        &mut session.connection,
        &format!("{m} yes"),
        answer(1, 1, None),
    )
    .await
}

async fn close(session: &mut WorldSession) {
    let m = session.character_name.clone();
    step(
        &mut session.connection,
        &format!("{m} close"),
        answer(0, -1, None),
    )
    .await;
}

#[tokio::test]
async fn masters_found_grow_join_and_disband_an_alliance() {
    let (a_config, b_config, c_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let (a_name, c_name) = (
        a_config.character_name.clone(),
        c_config.character_name.clone(),
    );
    let a_id = master(&a_config).await;
    let b_id = master(&b_config).await;
    let c_id = master(&c_config).await;
    set_meso(a_id, CREATE_COST + GROW_COST + 100)
        .await
        .expect("mesos");
    let mut a = at_hq(&a_config).await;
    let b = at_hq(&b_config).await;

    // Out of a party, Lenario won't found one.
    let t = lenario_menu(&mut a, CREATE).await;
    assert_eq!(t.prompts, [(LENARIO, 0)]);
    close(&mut a).await;

    // With B's guild, A founds one, after a name too short.
    let mut ab = [a, b];
    party(&mut ab).await;
    let [mut a, mut b] = ab;
    let t = lenario_menu(&mut a, CREATE).await;
    assert_eq!(t.prompts, [(LENARIO, 1)]);
    let t = yes(&mut a).await;
    assert_eq!(t.prompts, [(LENARIO, 2)]);
    let t = step(&mut a.connection, "a short", type_text("abc")).await;
    assert_eq!(t.prompts, [(LENARIO, 2)]);
    let t = step(&mut a.connection, "a name", type_text(&a_name)).await;
    assert_eq!(t.prompts, [(LENARIO, 1)]);
    let t = yes(&mut a).await;
    assert_eq!(t.stat(stat::MESO), Some(i64::from(GROW_COST + 100)));
    let (alliance_id, name, guilds, capacity, _) = alliance(&op_in(&mut a, t, INFO).await);
    assert_eq!(name, a_name);
    assert_eq!(guilds.len(), 2);
    assert_eq!(capacity, 2);
    let (_, _, b_guilds, _, _) = alliance(&next_op(&mut b, INFO).await);
    assert_eq!(b_guilds, guilds);
    close(&mut a).await;

    // Full, it grows by one for A's last million.
    let t = step(&mut a.connection, "a invite full", invite(&c_name)).await;
    assert!(t.broadcasts.iter().any(|(kind, _)| *kind == PINK));
    let t = lenario_menu(&mut a, GROW).await;
    assert_eq!(t.prompts, [(LENARIO, 1)]);
    let t = yes(&mut a).await;
    assert_eq!(t.stat(stat::MESO), Some(100));
    assert_eq!(alliance(&op_in(&mut a, t, CHANGED).await).3, 3);
    close(&mut a).await;

    // C, the master, is offline, then declines, then joins.
    send(&mut a.connection, invite(&c_name)).await;
    next_pink(&mut a).await;
    let mut c = at_hq(&c_config).await;
    send(&mut a.connection, invite(&c_name.to_uppercase())).await;
    let card = next_op(&mut c, INVITE).await;
    let mut r = Cursor::new(&card[1..]);
    assert_eq!(r.read_int().unwrap(), alliance_id);
    assert_eq!(r.read_str_with_length().unwrap(), a_name);
    send(&mut c.connection, decline(&a_name, &c_name)).await;
    next_pink(&mut a).await;
    send(&mut a.connection, invite(&c_name)).await;
    next_op(&mut c, INVITE).await;
    send(&mut c.connection, accept(alliance_id, &c_name)).await;
    for session in [&mut a, &mut b, &mut c] {
        let joined = next_op(session, GUILD_JOINED).await;
        assert_eq!(alliance(&joined).2.len(), 3);
    }
    send(&mut c.connection, operation(1)).await;
    assert_eq!(alliance(&next_op(&mut c, INFO).await).2.len(), 3);

    // Chat, the notice, and the titles reach the other guilds.
    let t = step(
        &mut a.connection,
        "a chat",
        alliance_chat(&[b_id, c_id], "hello alliance"),
    )
    .await;
    assert!(t.alliance_chat.is_empty());
    for session in [&mut b, &mut c] {
        let mut t = Trace::default();
        until(&mut session.connection, &mut t, |t| {
            !t.alliance_chat.is_empty()
        })
        .await;
        assert_eq!(
            t.alliance_chat,
            [(a_name.clone(), "hello alliance".to_string())]
        );
    }
    let mut notice = operation(0x0a);
    notice.write_str_with_length("Meet at Orbis").unwrap();
    send(&mut a.connection, notice).await;
    for session in [&mut a, &mut b, &mut c] {
        let op = next_op(session, NOTICE_CHANGED).await;
        let mut r = Cursor::new(&op[1..]);
        assert_eq!(r.read_int().unwrap(), alliance_id);
        assert_eq!(r.read_str_with_length().unwrap(), "Meet at Orbis");
    }
    let mut titles = operation(8);
    for title in ["Chief", "Lords", "Elder", "Folks", "Kith"] {
        titles.write_str_with_length(title).unwrap();
    }
    send(&mut a.connection, titles).await;
    let op = next_op(&mut c, TITLES_CHANGED).await;
    let mut r = Cursor::new(&op[1..]);
    assert_eq!(r.read_int().unwrap(), alliance_id);
    assert_eq!(r.read_str_with_length().unwrap(), "Chief");

    // The other guilds hear B leave the game and come back.
    drop(b);
    let card = |op: &[u8]| {
        let mut r = Cursor::new(&op[1..]);
        let ids = [
            r.read_int().unwrap(),
            r.read_int().unwrap(),
            r.read_int().unwrap(),
        ];
        (ids, r.read_byte().unwrap())
    };
    // Guilds are listed by id, and A's was founded first.
    let offline = card(&next_op(&mut a, MEMBER_ONLINE).await);
    assert_eq!(offline, ([alliance_id, guilds[1], b_id], 0));
    assert_eq!(card(&next_op(&mut c, MEMBER_ONLINE).await), offline);
    let mut b = at_hq(&b_config).await;
    let online = card(&next_op(&mut c, MEMBER_ONLINE).await);
    assert_eq!((online.0[2], online.1), (b_id, 1));

    // C's guild leaves; the others hear so.
    send(&mut c.connection, operation(2)).await;
    for session in [&mut a, &mut b] {
        let left = next_op(session, GUILD_LEFT).await;
        assert_eq!(alliance(&left).2.len(), 2);
        next_pink(session).await;
    }
    next_op(&mut c, GUILD_LEFT).await;

    // Only the alliance's master disbands it.
    let t = lenario_menu(&mut b, DISBAND).await;
    assert_eq!(t.prompts, [(LENARIO, 0)]);
    close(&mut b).await;
    let t = lenario_menu(&mut a, DISBAND).await;
    assert_eq!(t.prompts, [(LENARIO, 1)]);
    yes(&mut a).await;
    let op = next_op(&mut b, DISBANDED).await;
    assert_eq!(Cursor::new(&op[1..]).read_int().unwrap(), alliance_id);
    close(&mut a).await;

    // Gone: B's guild is in none.
    drop(b);
    let mut b = relogin(&b_config).await;
    let t = step(&mut b.connection, "b info", operation(1)).await;
    assert!(t.alliance_ops.is_empty());
}
