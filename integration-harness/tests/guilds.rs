//! Guilds (GUILD_OPERATION): Heracle founds one for a level 10 with the
//! fee once the client names it; a bad or taken name asks again. Members
//! get their guild on entering the game, others see the guild name under
//! a member's, and guild chat reaches the members online. The master
//! invites players on the channel, who may decline (DENY_GUILD_REQUEST) or
//! join; members leave, and are expelled. The master and jr. masters move
//! members between ranks and post the notice, and the master renames the
//! ranks. Members hear who logs in or out (but not a channel change), levels
//! up, or advances a job. At Heracle the master grows the guild five
//! members at a time, for a price, or disbands it; at Lea they make it an
//! emblem, for a price. Members of a guild gone meanwhile stay connected.

use game_data::GameData;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{
    delete_guild, grant_stack, guild_capacity_and_members, join_guild, set_guild_capacity,
    set_guild_rank, set_job, set_meso, set_str,
};
use integration_harness::packets::{
    build_change_channel, build_player_logged_in, decode_channel_change,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{barrier, read, read_for, send, stat, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::io::Cursor;
use std::path::Path;
use std::sync::OnceLock;
use tokio::time::Duration;

const GUILD_HQ: i32 = 200_000_301;
/// Orbis Park, away from the other test's founding.
const PARK: i32 = 200_000_200;
const HERACLE: i32 = 2010007;
const LEA: i32 = 2010008;
const FEE: i32 = 1_500_000;
const EMBLEM_COST: i32 = 5_000_000;
// Heracle's menu.
const CREATE: i32 = 0;
const DISBAND: i32 = 1;
const GROW: i32 = 2;

const NAME_PROMPT: u8 = 0x01;
const EMBLEM_MAKER: u8 = 0x11;
const INFO: u8 = 0x1a;
const NAME_TAKEN: u8 = 0x1c;
const CREATED: u8 = 0x20;
const INVITE: u8 = 0x05;
const JOINED: u8 = 0x27;
const NOT_IN_CHANNEL: u8 = 0x2a;
const LEFT: u8 = 0x2c;
const EXPELLED: u8 = 0x2f;
const DISBANDED: u8 = 0x32;
const DISBAND_FAILED: u8 = 0x34;
const DECLINED: u8 = 0x37;
const CAPACITY_CHANGED: u8 = 0x3a;
const GROW_FAILED: u8 = 0x3b;
const MEMBER_LEVEL_JOB: u8 = 0x3c;
const MEMBER_ONLINE: u8 = 0x3d;
const TITLES_CHANGED: u8 = 0x3e;
const RANK_CHANGED: u8 = 0x40;
const EMBLEM_CHANGED: u8 = 0x42;
const NOTICE_CHANGED: u8 = 0x44;

fn maps() -> &'static GameData {
    static MAPS: OnceLock<GameData> = OnceLock::new();
    MAPS.get_or_init(|| {
        GameData::load_from_nx_map(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx"),
        )
        .unwrap()
    })
}

fn talk(map: i32, npc: i32) -> Packet {
    let index = maps()
        .field(map)
        .unwrap()
        .map_npcs
        .iter()
        .position(|n| n.npc_id == npc)
        .unwrap();
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(1_000_000_000 + index as i32).unwrap();
    p
}

fn create(name: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::GuildOperation as i16).unwrap();
    p.write_byte(2).unwrap();
    p.write_str_with_length(name).unwrap();
    p
}

fn operation(mode: u8, ids: &[i32], name: Option<&str>) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::GuildOperation as i16).unwrap();
    p.write_byte(mode).unwrap();
    for &id in ids {
        p.write_int(id).unwrap();
    }
    if let Some(name) = name {
        p.write_str_with_length(name).unwrap();
    }
    p
}

fn decline(inviter: &str, me: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::DenyGuildRequest as i16).unwrap();
    p.write_byte(DECLINED).unwrap();
    p.write_str_with_length(inviter).unwrap();
    p.write_str_with_length(me).unwrap();
    p
}

/// The guild operation in `mode` `session` gets next.
async fn next_op(session: &mut WorldSession, mode: u8) -> Vec<u8> {
    let mut t = Trace::default();
    until(&mut session.connection, &mut t, |t| {
        t.guild_ops.iter().any(|op| op[0] == mode)
    })
    .await;
    t.guild_ops.into_iter().find(|op| op[0] == mode).unwrap()
}

/// A mode's body: an id, another id, then a name.
fn ids_and_name(op: &[u8]) -> (i32, i32, String) {
    let mut r = Cursor::new(&op[1..]);
    let ids = (r.read_int().unwrap(), r.read_int().unwrap());
    (ids.0, ids.1, r.read_str_with_length().unwrap())
}

fn guild_chat(to: &[i32], text: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::MultiChat as i16).unwrap();
    p.write_byte(2).unwrap();
    p.write_byte(to.len() as u8).unwrap();
    for &id in to {
        p.write_int(id).unwrap();
    }
    p.write_str_with_length(text).unwrap();
    p
}

/// A GUILDDATA block: the guild's name and (member, online) per member.
fn guild_data(body: &[u8]) -> (String, Vec<(i32, bool)>) {
    let mut r = Cursor::new(body);
    r.read_int().unwrap();
    let name = r.read_str_with_length().unwrap();
    for _ in 0..5 {
        r.read_str_with_length().unwrap();
    }
    let ids: Vec<i32> = (0..r.read_byte().unwrap())
        .map(|_| r.read_int().unwrap())
        .collect();
    let members = ids
        .into_iter()
        .map(|id| {
            r.read_bytes(13).unwrap();
            let [_job, _level, _rank, online, _, _] = [(); 6].map(|_| r.read_int().unwrap());
            (id, online == 1)
        })
        .collect();
    (name, members)
}

/// A GUILDDATA block's rank titles and notice.
fn titles_and_notice(body: &[u8]) -> ([String; 5], String) {
    let mut r = Cursor::new(body);
    r.read_int().unwrap();
    r.read_str_with_length().unwrap();
    let titles = [(); 5].map(|_| r.read_str_with_length().unwrap());
    let count = r.read_byte().unwrap() as usize;
    r.read_bytes(count * (4 + 37) + 4 + 6).unwrap();
    (titles, r.read_str_with_length().unwrap())
}

fn set_titles(titles: [&str; 5]) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::GuildOperation as i16).unwrap();
    p.write_byte(0x0d).unwrap();
    for title in titles {
        p.write_str_with_length(title).unwrap();
    }
    p
}

/// A GUILDDATA block's emblem.
fn emblem(body: &[u8]) -> [u16; 4] {
    let mut r = Cursor::new(body);
    r.read_int().unwrap();
    for _ in 0..6 {
        r.read_str_with_length().unwrap();
    }
    let count = r.read_byte().unwrap() as usize;
    r.read_bytes(count * (4 + 37) + 4).unwrap();
    [
        r.read_short().unwrap() as u16,
        u16::from(r.read_byte().unwrap()),
        r.read_short().unwrap() as u16,
        u16::from(r.read_byte().unwrap()),
    ]
}

/// The emblem maker's answer.
fn set_emblem([background, background_color, mark, mark_color]: [u16; 4]) -> Packet {
    let mut p = operation(0x0f, &[], None);
    p.write_short(background as i16).unwrap();
    p.write_byte(background_color as u8).unwrap();
    p.write_short(mark as i16).unwrap();
    p.write_byte(mark_color as u8).unwrap();
    p
}

fn set_rank(member_id: i32, rank: u8) -> Packet {
    let mut p = operation(0x0e, &[member_id], None);
    p.write_byte(rank).unwrap();
    p
}

/// Make the character behind `config` and log it out again; its id.
async fn new_character(config: &HarnessConfig) -> i32 {
    let session = login_to_world_session(config).await.expect("first login");
    let id = session.character_id;
    drop(session);
    drop(relogin(config).await);
    id
}

/// A level 10 with `meso` mesos, logged in at the guild headquarters.
async fn founder(config: &HarnessConfig, meso: i32) -> WorldSession {
    let id = new_character(config).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 10, 50).await.expect("level 10");
    set_meso(id, meso).await.expect("mesos");
    let mut session = relogin(config).await;
    let mut t = Trace::default();
    assert_eq!(
        warp(&mut session.connection, GUILD_HQ, "", &mut t).await,
        GUILD_HQ
    );
    read_for(&mut session.connection, &mut t, 300).await;
    session
}

/// Pick `choice` from Heracle's menu; what follows. Barrier lines carry
/// the speaker's name, since everyone on the map hears them.
async fn heracle_menu(session: &mut WorldSession, choice: i32) -> Trace {
    let m = session.character_name.clone();
    let c = &mut session.connection;
    let t = step(c, &format!("{m} talk"), talk(GUILD_HQ, HERACLE)).await;
    assert_eq!(t.prompts, [(HERACLE, 4)]);
    step(
        c,
        &format!("{m} picks {choice}"),
        answer(4, 1, Some(choice)),
    )
    .await
}

/// Heracle asks for a name.
async fn ask_heracle(session: &mut WorldSession) {
    let t = heracle_menu(session, CREATE).await;
    assert_eq!(t.prompts, [(HERACLE, 1)]);
    let m = session.character_name.clone();
    let t = step(
        &mut session.connection,
        &format!("{m} yes"),
        answer(1, 1, None),
    )
    .await;
    assert_eq!(t.guild_ops, [vec![NAME_PROMPT]]);
}

#[tokio::test]
async fn heracle_founds_a_guild_that_members_see_and_chat_in() {
    let (a_config, b_config, c_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    // A character name is a valid guild name, and unique.
    let name = a_config.character_name.clone();
    let b_id = new_character(&b_config).await;
    let mut b = relogin(&b_config).await;
    let mut bt = Trace::default();
    assert_eq!(
        warp(&mut b.connection, GUILD_HQ, "", &mut bt).await,
        GUILD_HQ
    );
    let mut a = founder(&a_config, FEE + 100_000).await;
    let a_id = a.character_id;

    // A level 1 can't found one, and naming one unasked does nothing.
    let t = heracle_menu(&mut b, CREATE).await;
    assert_eq!(t.prompts, [(HERACLE, 0)]);
    let t = step(&mut b.connection, "b unasked", create("Sneaky")).await;
    assert!(t.guild_ops.is_empty());
    step(&mut b.connection, "b close", answer(0, -1, None)).await;

    // A bad name is asked for again; a good one founds the guild.
    ask_heracle(&mut a).await;
    let t = step(&mut a.connection, "a bad", create("abc")).await;
    assert_eq!(t.guild_ops, [vec![NAME_TAKEN]]);
    let t = step(&mut a.connection, "a good", create(&name)).await;
    assert_eq!(t.stat(stat::MESO), Some(100_000));
    let created = t
        .guild_ops
        .iter()
        .find(|op| op[0] == CREATED)
        .expect("created");
    assert_eq!(
        guild_data(&created[1..]),
        (name.clone(), vec![(a_id, true)])
    );

    // B, on the map, sees the name under A's. The other test may found a
    // guild here meanwhile.
    let mut bt = Trace::default();
    until(&mut b.connection, &mut bt, |t| {
        t.guild_names.iter().any(|(id, _)| *id == a_id)
    })
    .await;
    assert!(bt.guild_names.contains(&(a_id, name.clone())));

    // The name is taken, whatever the case.
    let mut c = founder(&c_config, FEE).await;
    ask_heracle(&mut c).await;
    let t = step(&mut c.connection, "c taken", create(&name.to_uppercase())).await;
    assert_eq!(t.guild_ops, [vec![NAME_TAKEN]]);
    assert_eq!(t.stat(stat::MESO), None);
    drop(c);

    // B, put in the guild while away, gets it on entering the game, with A
    // online.
    drop(b);
    tokio::time::sleep(Duration::from_millis(500)).await;
    join_guild(b_id, &name).await.expect("B joins");
    let mut b = relogin(&b_config).await;
    let mut bt = Trace::default();
    until(&mut b.connection, &mut bt, |t| !t.guild_ops.is_empty()).await;
    assert_eq!(bt.guild_ops[0][..2], [INFO, 1]);
    assert_eq!(
        guild_data(&bt.guild_ops[0][2..]),
        (name.clone(), vec![(a_id, true), (b_id, true)])
    );

    // A sees B arrive with the guild name.
    assert_eq!(
        warp(&mut b.connection, GUILD_HQ, "", &mut bt).await,
        GUILD_HQ
    );
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| {
        t.players.iter().any(|p| p.character_id == b_id)
    })
    .await;
    let seen = at.players.iter().find(|p| p.character_id == b_id).unwrap();
    assert_eq!(seen.guild, name);

    // Guild chat reaches the other members, not the speaker.
    let t = step(
        &mut a.connection,
        "a chat",
        guild_chat(&[b_id], "hello guild"),
    )
    .await;
    assert!(t.guild_chat.is_empty());
    let mut bt = Trace::default();
    until(&mut b.connection, &mut bt, |t| !t.guild_chat.is_empty()).await;
    assert_eq!(
        bt.guild_chat,
        [(a.character_name.clone(), "hello guild".to_string())]
    );
    drop(a);
    drop(b);

    // The guild and the fee were saved.
    let mut a = relogin(&a_config).await;
    assert_eq!(a.meso, 100_000);
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| !t.guild_ops.is_empty()).await;
    assert_eq!(at.guild_ops[0][..2], [INFO, 1]);
    assert_eq!(guild_data(&at.guild_ops[0][2..]).0, name);
}

/// A level 1 in the park, seen by `seer`.
async fn visitor(config: &HarnessConfig, seer: &mut WorldSession) -> WorldSession {
    let id = new_character(config).await;
    let mut session = relogin(config).await;
    let mut t = Trace::default();
    assert_eq!(warp(&mut session.connection, PARK, "", &mut t).await, PARK);
    let mut seen = Trace::default();
    until(&mut seer.connection, &mut seen, |t| {
        t.players.iter().any(|p| p.character_id == id)
    })
    .await;
    session
}

#[tokio::test]
async fn members_are_invited_and_leave_or_are_expelled() {
    let (a_config, b_config, c_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let name = a_config.character_name.clone();
    let mut a = founder(&a_config, FEE).await;
    ask_heracle(&mut a).await;
    send(&mut a.connection, create(&name)).await;
    let created = next_op(&mut a, CREATED).await;
    let mut r = Cursor::new(&created[1..]);
    let guild_id = r.read_int().unwrap();
    let mut t = Trace::default();
    assert_eq!(warp(&mut a.connection, PARK, "", &mut t).await, PARK);
    let mut b = visitor(&b_config, &mut a).await;
    let mut c = visitor(&c_config, &mut a).await;
    let (a_id, b_id, c_id) = (a.character_id, b.character_id, c.character_id);
    let (a_name, b_name, c_name) = (
        a.character_name.clone(),
        b.character_name.clone(),
        c.character_name.clone(),
    );

    // Nobody by that name is on the channel.
    send(&mut a.connection, operation(INVITE, &[], Some("Nobody"))).await;
    assert_eq!(next_op(&mut a, NOT_IN_CHANNEL).await, [NOT_IN_CHANNEL]);

    // B turns the invitation down, and A hears so.
    send(&mut a.connection, operation(INVITE, &[], Some(&b_name))).await;
    let invite = next_op(&mut b, INVITE).await;
    let mut r = Cursor::new(&invite[1..]);
    assert_eq!(r.read_int().unwrap(), guild_id);
    assert_eq!(r.read_str_with_length().unwrap(), a_name);
    send(&mut b.connection, decline(&a_name, &b_name)).await;
    let declined = next_op(&mut a, DECLINED).await;
    assert_eq!(
        Cursor::new(&declined[1..]).read_str_with_length().unwrap(),
        b_name
    );

    // Accepting a declined invitation does nothing; a new one is taken up.
    let accept = |id| operation(0x06, &[guild_id, id], None);
    step(&mut b.connection, "b stale", accept(b_id)).await;
    send(&mut a.connection, operation(INVITE, &[], Some(&b_name))).await;
    next_op(&mut b, INVITE).await;
    send(&mut b.connection, accept(b_id)).await;
    let joined = next_op(&mut b, JOINED).await;
    assert_eq!(
        joined[1..9],
        [guild_id.to_le_bytes(), b_id.to_le_bytes()].concat()
    );
    // A hears it, and sees the name under B's, in either order.
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| {
        t.guild_ops.iter().any(|op| op[0] == JOINED) && !t.guild_names.is_empty()
    })
    .await;
    assert_eq!(at.guild_names, [(b_id, name.clone())]);

    // B, a plain member, may not invite; A invites C, who joins.
    step(
        &mut b.connection,
        "b invites",
        operation(INVITE, &[], Some(&c_name)),
    )
    .await;
    let mut ct = Trace::default();
    read_for(&mut c.connection, &mut ct, 500).await;
    assert!(ct.guild_ops.is_empty());
    send(&mut a.connection, operation(INVITE, &[], Some(&c_name))).await;
    next_op(&mut c, INVITE).await;
    send(&mut c.connection, accept(c_id)).await;
    next_op(&mut c, JOINED).await;

    // B may not expel C; A may, and everyone hears it.
    step(
        &mut b.connection,
        "b expels",
        operation(0x08, &[c_id], Some(&c_name)),
    )
    .await;
    let mut ct = Trace::default();
    read_for(&mut c.connection, &mut ct, 500).await;
    assert!(!ct.guild_ops.iter().any(|op| op[0] == EXPELLED));
    send(&mut a.connection, operation(0x08, &[c_id], Some(&c_name))).await;
    for session in [&mut b, &mut c] {
        let expelled = next_op(session, EXPELLED).await;
        assert_eq!(ids_and_name(&expelled), (guild_id, c_id, c_name.clone()));
    }
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| {
        t.guild_ops.iter().any(|op| op[0] == EXPELLED)
            && t.guild_names.contains(&(c_id, String::new()))
    })
    .await;

    // B leaves; the master can't.
    send(&mut b.connection, operation(0x07, &[b_id], Some(&b_name))).await;
    for session in [&mut a, &mut b] {
        let left = next_op(session, LEFT).await;
        assert_eq!(ids_and_name(&left), (guild_id, b_id, b_name.clone()));
    }
    let t = step(
        &mut a.connection,
        "a leaves",
        operation(0x07, &[a_id], Some(&a_name)),
    )
    .await;
    assert!(t.guild_ops.is_empty());
    drop((a, b, c));
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Only A is left, and B enters the game without a guild.
    let mut a = relogin(&a_config).await;
    let info = next_op(&mut a, INFO).await;
    assert_eq!(guild_data(&info[2..]), (name, vec![(a_id, true)]));
    let mut b = relogin(&b_config).await;
    let mut bt = Trace::default();
    read_for(&mut b.connection, &mut bt, 1000).await;
    assert!(bt.guild_ops.is_empty());
}

#[tokio::test]
async fn ranks_titles_and_the_notice_change() {
    let (a_config, b_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let name = a_config.character_name.clone();
    let mut a = founder(&a_config, FEE).await;
    ask_heracle(&mut a).await;
    send(&mut a.connection, create(&name)).await;
    let created = next_op(&mut a, CREATED).await;
    let guild_id = Cursor::new(&created[1..]).read_int().unwrap();
    // Every rank has a title, so members join at rank 5.
    assert_eq!(
        titles_and_notice(&created[1..]).0,
        ["Master", "Jr. Master", "Member", "Member", "Member"]
    );
    let mut t = Trace::default();
    assert_eq!(warp(&mut a.connection, PARK, "", &mut t).await, PARK);
    let mut b = visitor(&b_config, &mut a).await;
    let (a_id, b_id) = (a.character_id, b.character_id);
    send(
        &mut a.connection,
        operation(INVITE, &[], Some(&b.character_name)),
    )
    .await;
    next_op(&mut b, INVITE).await;
    send(&mut b.connection, operation(0x06, &[guild_id, b_id], None)).await;
    let joined = next_op(&mut b, JOINED).await;
    let mut r = Cursor::new(&joined[1 + 8 + 13 + 8..]);
    assert_eq!(r.read_int().unwrap(), 5);
    next_op(&mut a, JOINED).await;

    // A plain member changes nothing.
    let bm = b.character_name.clone();
    for (what, packet) in [
        ("rank", set_rank(b_id, 4)),
        ("notice", operation(0x10, &[], Some("mine"))),
        ("titles", set_titles(["Boss", "Deputy", "Grunt", "", ""])),
    ] {
        step(&mut b.connection, &format!("{bm} {what}"), packet).await;
    }
    let mut at = Trace::default();
    read_for(&mut a.connection, &mut at, 500).await;
    assert!(at
        .guild_ops
        .iter()
        .all(|op| op[0] != RANK_CHANGED && op[0] != NOTICE_CHANGED && op[0] != TITLES_CHANGED));

    // The master moves B up a rank, and both hear it; not two ranks.
    send(&mut a.connection, set_rank(b_id, 4)).await;
    for session in [&mut a, &mut b] {
        let changed = next_op(session, RANK_CHANGED).await;
        assert_eq!(
            changed[1..],
            [&guild_id.to_le_bytes()[..], &b_id.to_le_bytes(), &[4]].concat()
        );
    }
    let am = a.character_name.clone();
    for (what, packet) in [
        ("skip", set_rank(b_id, 2)),
        ("master", set_rank(b_id, 1)),
        ("self", set_rank(a_id, 2)),
    ] {
        let t = step(&mut a.connection, &format!("{am} {what}"), packet).await;
        assert!(t.guild_ops.is_empty(), "{what}");
    }

    // The notice.
    send(&mut a.connection, operation(0x10, &[], Some("hi all"))).await;
    for session in [&mut a, &mut b] {
        let changed = next_op(session, NOTICE_CHANGED).await;
        let mut r = Cursor::new(&changed[1..]);
        assert_eq!(r.read_int().unwrap(), guild_id);
        assert_eq!(r.read_str_with_length().unwrap(), "hi all");
    }
    // A control character, which the client refuses and Postgres can't
    // store, changes nothing and keeps A connected.
    let t = step(
        &mut a.connection,
        &format!("{am} control"),
        operation(0x10, &[], Some("hi\0all")),
    )
    .await;
    assert!(t.guild_ops.is_empty());

    // Titles: rank 5 may lose its title, but not rank 4 while B holds it.
    // The untouched title goes back as it was, with its space and dot.
    let titles = ["Boss", "Jr. Master", "Grunt", "Newbie", ""];
    send(&mut a.connection, set_titles(titles)).await;
    for session in [&mut a, &mut b] {
        let changed = next_op(session, TITLES_CHANGED).await;
        let mut r = Cursor::new(&changed[1..]);
        assert_eq!(r.read_int().unwrap(), guild_id);
        assert_eq!([(); 5].map(|_| r.read_str_with_length().unwrap()), titles);
    }
    for (what, packet) in [
        ("held", set_titles(["Boss", "Jr. Master", "Grunt", "", ""])),
        (
            "gap",
            set_titles(["Boss", "Jr. Master", "Grunt", "", "Newbie"]),
        ),
        (
            "short",
            set_titles(["Bos", "Deputy", "Grunt", "Newbie", ""]),
        ),
        // "test" is a forbidden word.
        (
            "forbidden",
            set_titles(["Boss", "Jr. Master", "Testy", "Newbie", ""]),
        ),
    ] {
        let t = step(&mut a.connection, &format!("{am} {what}"), packet).await;
        assert!(t.guild_ops.is_empty(), "{what}");
    }
    // B can't go below the last rank with a title.
    let t = step(&mut a.connection, &format!("{am} low"), set_rank(b_id, 5)).await;
    assert!(t.guild_ops.is_empty());
    drop((a, b));
    tokio::time::sleep(Duration::from_millis(500)).await;

    // It all lasts.
    let mut a = relogin(&a_config).await;
    let info = next_op(&mut a, INFO).await;
    assert_eq!(
        titles_and_notice(&info[2..]),
        (titles.map(String::from), "hi all".to_string())
    );
}

/// A Writ of Solomon worth 100 EXP.
const WRIT: i32 = 2_370_012;
const WARRIORS_SANCTUARY: i32 = 102_000_003;
const DANCES_WITH_BALROG: i32 = 1022000;

/// USE_SOLOMON_ITEM of the writ in USE slot 1, then USE_GACHA_EXP to take it.
fn use_writ() -> [Packet; 2] {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseSolomonItem as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_short(1).unwrap();
    p.write_int(WRIT).unwrap();
    let mut take = Packet::new_empty();
    take.write_short(RecvOpcode::UseGachaExp as i16).unwrap();
    take.write_int(0).unwrap();
    [p, take]
}

/// Guild `guild_id`, member `member`, then the rest of a 0x3c or 0x3d.
fn about(guild_id: i32, member: i32, rest: &[u8]) -> Vec<u8> {
    [&guild_id.to_le_bytes()[..], &member.to_le_bytes(), rest].concat()
}

#[tokio::test]
async fn members_hear_who_logs_in_or_out_levels_up_and_advances() {
    let (a_config, b_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let name = a_config.character_name.clone();
    let b_id = new_character(&b_config).await;
    let mut a = founder(&a_config, FEE).await;
    ask_heracle(&mut a).await;
    send(&mut a.connection, create(&name)).await;
    let created = next_op(&mut a, CREATED).await;
    let guild_id = Cursor::new(&created[1..]).read_int().unwrap();
    join_guild(b_id, &name).await.expect("B joins");
    grant_stack(b_id, 2, 1, WRIT, 1).await.expect("a writ");

    // B logs in: A hears it, B hears nothing of themself.
    let mut b = relogin(&b_config).await;
    let online = next_op(&mut a, MEMBER_ONLINE).await;
    assert_eq!(online[1..], about(guild_id, b_id, &[1]));
    let mut bt = Trace::default();
    until(&mut b.connection, &mut bt, |t| !t.guild_ops.is_empty()).await;
    read_for(&mut b.connection, &mut bt, 300).await;
    assert!(bt.guild_ops.iter().all(|op| op[0] != MEMBER_ONLINE));

    // B goes from level 1 to 3: the list changes for both, and A reads a
    // line for each level.
    let bm = b.character_name.clone();
    for packet in use_writ() {
        send(&mut b.connection, packet).await;
    }
    until(&mut b.connection, &mut bt, |t| {
        t.stat(stat::LEVEL) == Some(3)
    })
    .await;
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| t.notices.len() == 2).await;
    let level_up = |level| (SendOpcode::NotifyLevelUp as i16, 2, level, bm.clone());
    assert_eq!(at.notices, [level_up(2), level_up(3)]);
    let level_job = |ops: &[Vec<u8>]| {
        ops.iter()
            .rfind(|op| op[0] == MEMBER_LEVEL_JOB)
            .map(|op| op[1..].to_vec())
    };
    let changed = about(guild_id, b_id, &[3, 0, 0, 0, 0, 0, 0, 0]);
    until(&mut a.connection, &mut at, |t| {
        level_job(&t.guild_ops).as_ref() == Some(&changed)
    })
    .await;
    until(&mut b.connection, &mut bt, |t| {
        level_job(&t.guild_ops).as_ref() == Some(&changed)
    })
    .await;
    barrier(&mut b.connection, &bm, &mut bt).await;
    assert!(bt.notices.is_empty());

    // A channel change says nothing; leaving the game from there does.
    send(&mut b.connection, build_change_channel(1).unwrap()).await;
    let redirect = loop {
        let packet = read(&mut b.connection, &mut bt).await;
        if packet.opcode() == SendOpcode::ChangeChannel as i16 {
            break decode_channel_change(&packet).unwrap();
        }
    };
    assert_eq!(redirect.port, b_config.channel_addr(1).port());
    drop(b);
    let mut moved = MapleTestConnection::connect(b_config.channel_addr(1), "guild channel change")
        .await
        .unwrap();
    send(&mut moved, build_player_logged_in(b_id).unwrap()).await;
    loop {
        let packet = read(&mut moved, &mut bt).await;
        if packet.opcode() == SendOpcode::SetField as i16 {
            break;
        }
    }
    barrier(&mut moved, &bm, &mut bt).await;
    let mut at = Trace::default();
    read_for(&mut a.connection, &mut at, 1000).await;
    assert!(at.guild_ops.iter().all(|op| op[0] != MEMBER_ONLINE));
    drop(moved);
    let offline = next_op(&mut a, MEMBER_ONLINE).await;
    assert_eq!(offline[1..], about(guild_id, b_id, &[0]));

    // A Warrior at last: A reads the job, with the "> " the client's text
    // lacks.
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(b_id, 0, 10, 50).await.expect("level 10");
    set_str(b_id, 35).await.expect("strong");
    let mut b = relogin(&b_config).await;
    next_op(&mut a, MEMBER_ONLINE).await;
    let c = &mut b.connection;
    let mut bt = Trace::default();
    assert_eq!(
        warp(c, WARRIORS_SANCTUARY, "", &mut bt).await,
        WARRIORS_SANCTUARY
    );
    barrier(c, &bm, &mut bt).await;
    step(c, &bm, talk(WARRIORS_SANCTUARY, DANCES_WITH_BALROG)).await;
    step(c, &bm, answer(0, 1, None)).await;
    let t = step(c, &bm, answer(1, 1, None)).await;
    assert_eq!(t.stat_values(0x20), [100]);
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| !t.notices.is_empty()).await;
    assert_eq!(
        at.notices,
        [(
            SendOpcode::NotifyJobChange as i16,
            0,
            100,
            format!("> {bm}")
        )]
    );
    let changed = about(guild_id, b_id, &[10, 0, 0, 0, 100, 0, 0, 0]);
    until(&mut a.connection, &mut at, |t| {
        level_job(&t.guild_ops).as_ref() == Some(&changed)
    })
    .await;
}

#[tokio::test]
async fn the_master_grows_and_disbands_the_guild_at_heracle() {
    let (a_config, b_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let name = a_config.character_name.clone();
    let b_id = new_character(&b_config).await;
    let mut a = founder(&a_config, FEE + 900_000).await;
    let a_id = a.character_id;
    ask_heracle(&mut a).await;
    send(&mut a.connection, create(&name)).await;
    let created = next_op(&mut a, CREATED).await;
    let guild_id = Cursor::new(&created[1..]).read_int().unwrap();
    join_guild(b_id, &name).await.expect("B joins");
    let mut b = relogin(&b_config).await;
    let mut bt = Trace::default();
    assert_eq!(
        warp(&mut b.connection, GUILD_HQ, "", &mut bt).await,
        GUILD_HQ
    );
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| {
        t.players.iter().any(|p| p.character_id == b_id)
    })
    .await;
    let mut members = vec![a_id, b_id];
    members.sort();

    // Only the master may grow or disband it.
    for choice in [GROW, DISBAND] {
        let t = heracle_menu(&mut b, choice).await;
        assert_eq!(t.prompts, [(HERACLE, 0)]);
        step(&mut b.connection, "b close", answer(0, -1, None)).await;
    }

    // Heracle quotes 500,000 mesos to grow it from 10 to 15, but won't once
    // the guild has changed since.
    let t = heracle_menu(&mut a, GROW).await;
    assert_eq!(t.prompts, [(HERACLE, 1)]);
    assert!(t.prompt_texts[0].contains("500000") && t.prompt_texts[0].contains("15"));
    set_guild_capacity(&name, 15)
        .await
        .expect("grown meanwhile");
    let t = step(&mut a.connection, "a stale grow", answer(1, 1, None)).await;
    assert_eq!(t.guild_ops, [vec![GROW_FAILED]]);
    assert_eq!(t.stat(stat::MESO), None);
    set_guild_capacity(&name, 10).await.expect("back to 10");

    // It grows, the whole guild hears so, and A pays.
    let grown = [
        vec![CAPACITY_CHANGED],
        guild_id.to_le_bytes().to_vec(),
        vec![15],
    ]
    .concat();
    heracle_menu(&mut a, GROW).await;
    send(&mut a.connection, answer(1, 1, None)).await;
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| {
        t.guild_ops.contains(&grown) && t.stat(stat::MESO).is_some()
    })
    .await;
    assert_eq!(at.stat(stat::MESO), Some(400_000));
    assert_eq!(next_op(&mut b, CAPACITY_CHANGED).await, grown);
    assert_eq!(
        guild_capacity_and_members(&name).await.unwrap(),
        Some((15, members.clone()))
    );

    // Short of the next step's price, A is told so.
    let t = heracle_menu(&mut a, GROW).await;
    assert_eq!(t.prompts, [(HERACLE, 1)]);
    let t = step(&mut a.connection, "a poor grow", answer(1, 1, None)).await;
    assert_eq!(t.prompts, [(HERACLE, 0)]);
    assert!(t.guild_ops.is_empty());
    step(&mut a.connection, "a close", answer(0, -1, None)).await;

    // A, no longer master by the time they say yes, can't disband it.
    let t = heracle_menu(&mut a, DISBAND).await;
    assert_eq!(t.prompts, [(HERACLE, 1)]);
    set_guild_rank(a_id, 2).await.expect("demoted");
    let t = step(&mut a.connection, "a stale disband", answer(1, 1, None)).await;
    assert_eq!(t.guild_ops, [vec![DISBAND_FAILED]]);
    set_guild_rank(a_id, 1).await.expect("master again");

    // A disbands it. Both hear so and see the name go from under the
    // other's.
    let disbanded = [vec![DISBANDED], guild_id.to_le_bytes().to_vec()].concat();
    heracle_menu(&mut a, DISBAND).await;
    send(&mut a.connection, answer(1, 1, None)).await;
    for (session, other) in [(&mut a, b_id), (&mut b, a_id)] {
        let mut t = Trace::default();
        until(&mut session.connection, &mut t, |t| {
            t.guild_ops.contains(&disbanded) && t.guild_names.contains(&(other, String::new()))
        })
        .await;
    }
    assert_eq!(guild_capacity_and_members(&name).await.unwrap(), None);
    drop((a, b));
    tokio::time::sleep(Duration::from_millis(500)).await;

    // A enters the game again without a guild, and with the mesos left.
    let mut a = relogin(&a_config).await;
    assert_eq!(a.meso, 400_000);
    let mut at = Trace::default();
    read_for(&mut a.connection, &mut at, 1000).await;
    assert!(at.guild_ops.is_empty());
}

/// Ask Lea about the emblem; what follows.
async fn ask_lea(session: &mut WorldSession) -> Trace {
    let m = session.character_name.clone();
    let c = &mut session.connection;
    let t = step(c, &format!("{m} talks to Lea"), talk(GUILD_HQ, LEA)).await;
    assert_eq!(t.prompts, [(LEA, 4)]);
    step(c, &format!("{m} asks Lea"), answer(4, 1, Some(0))).await
}

#[tokio::test]
async fn the_master_changes_the_emblem_at_lea() {
    let (a_config, b_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let name = a_config.character_name.clone();
    let b_id = new_character(&b_config).await;
    let mut a = founder(&a_config, FEE + EMBLEM_COST + 100_000).await;
    let a_id = a.character_id;
    ask_heracle(&mut a).await;
    send(&mut a.connection, create(&name)).await;
    let created = next_op(&mut a, CREATED).await;
    let guild_id = Cursor::new(&created[1..]).read_int().unwrap();
    join_guild(b_id, &name).await.expect("B joins");
    let mut b = relogin(&b_config).await;
    let mut bt = Trace::default();
    assert_eq!(
        warp(&mut b.connection, GUILD_HQ, "", &mut bt).await,
        GUILD_HQ
    );
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| {
        t.players.iter().any(|p| p.character_id == b_id)
    })
    .await;

    // Lea only offers the master the emblem maker, and for a price.
    let t = ask_lea(&mut b).await;
    assert_eq!(t.prompts, [(LEA, 0)]);
    step(&mut b.connection, "b closes", answer(0, -1, None)).await;
    let t = ask_lea(&mut a).await;
    assert_eq!(t.prompts, [(LEA, 1)]);
    assert!(t.prompt_texts[0].contains("5000000"));
    let t = step(&mut a.connection, "a says no", answer(1, 0, None)).await;
    assert_eq!(t.prompts, [(LEA, 0)]);
    assert!(t.guild_ops.is_empty());
    step(&mut a.connection, "a closes", answer(0, -1, None)).await;
    ask_lea(&mut a).await;
    let t = step(&mut a.connection, "a says yes", answer(1, 1, None)).await;
    assert_eq!(t.guild_ops, [vec![EMBLEM_MAKER]]);

    // No emblem the maker can't make, none from another member, and none
    // away from Lea.
    let t = step(
        &mut a.connection,
        "a makes none",
        set_emblem([999, 1, 2000, 1]),
    )
    .await;
    assert!(t.guild_ops.is_empty() && t.stat(stat::MESO).is_none());
    let t = step(
        &mut b.connection,
        "b makes one",
        set_emblem([1001, 1, 2001, 1]),
    )
    .await;
    assert!(t.guild_ops.is_empty() && t.broadcasts.is_empty());
    let mut at = Trace::default();
    assert_eq!(warp(&mut a.connection, PARK, "", &mut at).await, PARK);
    let t = step(
        &mut a.connection,
        "a makes one away",
        set_emblem([1002, 2, 2002, 2]),
    )
    .await;
    assert!(t.guild_ops.is_empty() && t.stat(stat::MESO).is_none());
    let mut at = Trace::default();
    assert_eq!(
        warp(&mut a.connection, GUILD_HQ, "", &mut at).await,
        GUILD_HQ
    );
    until(&mut a.connection, &mut at, |t| {
        t.players.iter().any(|p| p.character_id == b_id)
    })
    .await;

    // A makes one: the whole guild hears so, each sees the other's emblem
    // change, and A pays.
    let made = [1005, 3, 2004, 7];
    let changed = [
        vec![EMBLEM_CHANGED],
        guild_id.to_le_bytes().to_vec(),
        vec![0xed, 0x03, 3, 0xd4, 0x07, 7],
    ]
    .concat();
    send(&mut a.connection, set_emblem(made)).await;
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| {
        t.guild_ops.contains(&changed)
            && t.stat(stat::MESO).is_some()
            && t.guild_marks.contains(&(b_id, made))
    })
    .await;
    assert_eq!(at.stat(stat::MESO), Some(100_000));
    let mut bt = Trace::default();
    until(&mut b.connection, &mut bt, |t| {
        t.guild_ops.iter().any(|op| op[0] == EMBLEM_CHANGED)
            && t.guild_marks.contains(&(a_id, made))
    })
    .await;
    assert_eq!(bt.guild_ops, [changed]);

    // Short of the price, A is told so.
    let t = step(
        &mut a.connection,
        "a can't pay",
        set_emblem([1006, 4, 3000, 8]),
    )
    .await;
    assert!(t.guild_ops.is_empty() && t.stat(stat::MESO).is_none());
    assert_eq!(t.broadcasts.len(), 1);
    assert_eq!(t.broadcasts[0].0, 1);
    assert!(t.broadcasts[0].1.contains(&EMBLEM_COST.to_string()));
    drop((a, b));
    tokio::time::sleep(Duration::from_millis(500)).await;

    // The guild keeps it, and A the mesos left.
    let mut b = relogin(&b_config).await;
    let mut bt = Trace::default();
    until(&mut b.connection, &mut bt, |t| !t.guild_ops.is_empty()).await;
    assert_eq!(bt.guild_ops[0][..2], [INFO, 1]);
    assert_eq!(emblem(&bt.guild_ops[0][2..]), made);
    assert_eq!(relogin(&a_config).await.meso, 100_000);
}

/// A guild can be gone by the time a member acts on it (disbanded on
/// another channel): the client hears of a failure, or nothing, and stays
/// connected.
#[tokio::test]
async fn members_of_a_guild_gone_meanwhile_stay_connected() {
    let (a_config, b_config) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let name = a_config.character_name.clone();
    let b_name = b_config.character_name.clone();
    let b_id = new_character(&b_config).await;
    let mut a = founder(&a_config, 2 * FEE + 500_000).await;
    let a_id = a.character_id;
    ask_heracle(&mut a).await;
    send(&mut a.connection, create(&name)).await;
    next_op(&mut a, CREATED).await;
    join_guild(b_id, &name).await.expect("B joins");
    let mut b = relogin(&b_config).await;
    let mut bt = Trace::default();
    assert_eq!(
        warp(&mut b.connection, GUILD_HQ, "", &mut bt).await,
        GUILD_HQ
    );
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| {
        t.players.iter().any(|p| p.character_id == b_id)
    })
    .await;

    // A's yes to growing it finds it gone.
    let t = heracle_menu(&mut a, GROW).await;
    assert_eq!(t.prompts, [(HERACLE, 1)]);
    delete_guild(&name).await.expect("gone");
    let t = step(&mut a.connection, "a grows it", answer(1, 1, None)).await;
    assert_eq!(t.guild_ops, [vec![GROW_FAILED]]);
    assert_eq!(t.stat(stat::MESO), None);

    // So does an emblem from a maker opened before: not even the price,
    // which A lacks, is mentioned.
    let t = step(
        &mut a.connection,
        "a sets the emblem",
        set_emblem([1001, 1, 2001, 1]),
    )
    .await;
    assert!(t.guild_ops.is_empty() && t.broadcasts.is_empty());
    assert_eq!(t.stat(stat::MESO), None);

    // B asks for the guild window and gets none, and the name goes from
    // under B's.
    let t = step(&mut b.connection, "b reload", operation(0x00, &[], None)).await;
    assert_eq!(t.guild_ops, [vec![INFO, 0]]);
    let mut at = Trace::default();
    until(&mut a.connection, &mut at, |t| {
        t.guild_names.contains(&(b_id, String::new()))
    })
    .await;

    // A's invitation and expulsion go nowhere.
    for (what, op) in [
        ("a invites", operation(INVITE, &[], Some(&b_name))),
        ("a expels", operation(0x08, &[b_id], Some(&b_name))),
    ] {
        let t = step(&mut a.connection, what, op).await;
        assert!(t.guild_ops.is_empty(), "{what}");
    }
    let mut bt = Trace::default();
    read_for(&mut b.connection, &mut bt, 300).await;
    assert!(bt.guild_ops.is_empty());

    // Heracle finds A guildless, so A founds another.
    ask_heracle(&mut a).await;
    let mut bt = Trace::default();
    until(&mut b.connection, &mut bt, |t| {
        t.guild_names.contains(&(a_id, String::new()))
    })
    .await;
    send(&mut a.connection, create(&name)).await;
    let created = next_op(&mut a, CREATED).await;
    let guild_id = Cursor::new(&created[1..]).read_int().unwrap();

    // It goes too, after A invites B and is asked to disband it: A's yes
    // fails, and B's acceptance does nothing.
    send(&mut a.connection, operation(INVITE, &[], Some(&b_name))).await;
    next_op(&mut b, INVITE).await;
    let t = heracle_menu(&mut a, DISBAND).await;
    assert_eq!(t.prompts, [(HERACLE, 1)]);
    delete_guild(&name).await.expect("gone again");
    let t = step(&mut a.connection, "a disbands it", answer(1, 1, None)).await;
    assert_eq!(t.guild_ops, [vec![DISBAND_FAILED]]);
    send(&mut b.connection, operation(0x06, &[guild_id, b_id], None)).await;
    let mut bt = Trace::default();
    read_for(&mut b.connection, &mut bt, 500).await;
    assert!(bt.guild_ops.is_empty());
    barrier(&mut b.connection, "b still here", &mut bt).await;
    assert_eq!(guild_capacity_and_members(&name).await.unwrap(), None);
}
