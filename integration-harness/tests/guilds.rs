//! Guilds (GUILD_OPERATION): Heracle founds one for a level 10 with the
//! fee once the client names it; a bad or taken name asks again. Members
//! get their guild on entering the game, others see the guild name under
//! a member's, and guild chat reaches the members online.

use game_data::GameData;
use integration_harness::database::{join_guild, set_job, set_meso};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, stat, step, until, warp, Trace};
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
const HERACLE: i32 = 2010007;
const FEE: i32 = 1_500_000;

const NAME_PROMPT: u8 = 0x01;
const INFO: u8 = 0x1a;
const NAME_TAKEN: u8 = 0x1c;
const CREATED: u8 = 0x20;

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

    // B, on the map, sees the name under A's.
    let mut bt = Trace::default();
    until(&mut b.connection, &mut bt, |t| !t.guild_names.is_empty()).await;
    assert_eq!(bt.guild_names, [(a_id, name.clone())]);

    // The name is taken, whatever the case.
    let mut c = founder(&c_config, FEE).await;
    ask_heracle(&mut c).await;
    let t = step(&mut c.connection, "c taken", create(&name.to_uppercase())).await;
    assert_eq!(t.guild_ops, [vec![NAME_TAKEN]]);
    assert_eq!(t.stat(stat::MESO), None);
    drop(c);

    // B joins (there are no invitations yet) and gets the guild on entering
    // the game, with A online.
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
