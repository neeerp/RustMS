//! Guilds (GUILD_OPERATION): Heracle founds one for a level 10 with the
//! fee once the client names it; a bad or taken name asks again. Members
//! get their guild on entering the game, others see the guild name under
//! a member's, and guild chat reaches the members online. The master
//! invites players on the channel, who may decline (DENY_GUILD_REQUEST) or
//! join; members leave, and are expelled. The master and jr. masters move
//! members between ranks and post the notice, and the master renames the
//! ranks.

use game_data::GameData;
use integration_harness::database::{join_guild, set_job, set_meso};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, stat, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, HarnessConfig, WorldSession};
use net::packet::op::RecvOpcode;
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
const FEE: i32 = 1_500_000;

const NAME_PROMPT: u8 = 0x01;
const INFO: u8 = 0x1a;
const NAME_TAKEN: u8 = 0x1c;
const CREATED: u8 = 0x20;
const INVITE: u8 = 0x05;
const JOINED: u8 = 0x27;
const NOT_IN_CHANNEL: u8 = 0x2a;
const LEFT: u8 = 0x2c;
const EXPELLED: u8 = 0x2f;
const DECLINED: u8 = 0x37;
const TITLES_CHANGED: u8 = 0x3e;
const RANK_CHANGED: u8 = 0x40;
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

/// Heracle asks for a name. Barrier lines carry the speaker's name, since
/// everyone on the map hears them.
async fn ask_heracle(session: &mut WorldSession) {
    let m = session.character_name.clone();
    let c = &mut session.connection;
    let t = step(c, &format!("{m} talk"), talk(GUILD_HQ, HERACLE)).await;
    assert_eq!(t.prompts, [(HERACLE, 1)]);
    let t = step(c, &format!("{m} yes"), answer(1, 1, None)).await;
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
    let t = step(&mut b.connection, "b weak", talk(GUILD_HQ, HERACLE)).await;
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
