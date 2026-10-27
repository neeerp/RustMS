//! The Guild Quest's line: a guild's master registers it with Shuang at the
//! Excavation Site's camp, and as no one else plays on the channel its turn
//! comes at once: its members hear of it (GUILD_OPERATION 0x4C), the master
//! goes into the waiting room, and others of the guild follow while it
//! gathers, the way on shut until then. Another guild registering hears it
//! is next. Once under way, too few staying ends it a few seconds after the
//! client's notice (0x4A), everyone goes to the Returning Path, where Nuris
//! takes them back to the camp, and the next guild's turn comes; alone, its
//! quest ends when the gathering is over. The harness lets two go
//! (`RUSTMS_EXPEDITION_MIN`) and runs its 3 minutes of gathering in 3
//! seconds (`RUSTMS_EVENT_SCALE`).

use game_data::server_content::keys;
use integration_harness::database::{found_guild, join_guild};
use integration_harness::events::{maps, said, talk, told};
use integration_harness::hunting::walk_to;
use integration_harness::packets::build_change_map;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use packet::io::read::PktRead;
use std::io::Cursor;
use tokio::time::Duration;

const CAMP: i32 = 101_030_104;
const WAITING_ROOM: i32 = 990_000_000;
const FIRST_STAGE: i32 = 990_000_100;
const RETURNING_PATH: i32 = 990_001_100;
const SHUANG: i32 = 9_040_000;
const NURIS: i32 = 9_040_001;
const TOO_FEW: u8 = 0x4A;
const WAITLIST: u8 = 0x4C;

/// New characters, logged in at the camp: the first founds a guild named
/// after them, the rest join it.
async fn guild(size: usize) -> Vec<WorldSession> {
    let mut configs = vec![];
    let mut ids = vec![];
    for _ in 0..size {
        let config = load_harness_config_or_fail().await;
        ids.push(login_to_world_session(&config).await.unwrap().character_id);
        drop(relogin(&config).await);
        configs.push(config);
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    let name = &configs[0].character_name;
    found_guild(ids[0], name).await.unwrap();
    for &id in &ids[1..] {
        join_guild(id, name).await.unwrap();
    }
    let mut sessions = vec![];
    for config in &configs {
        let mut session = relogin(config).await;
        let mut seen = Trace::default();
        assert_eq!(
            warp(&mut session.connection, CAMP, "", &mut seen).await,
            CAMP
        );
        sessions.push(session);
    }
    sessions
}

/// The waitlist numbers `t` heard (0x4C: channel, number).
fn waitlist(t: &Trace) -> Vec<(u8, i32)> {
    t.guild_ops
        .iter()
        .filter(|op| op[0] == WAITLIST)
        .map(|op| {
            let mut r = Cursor::new(&op[1..]);
            (r.read_byte().unwrap(), r.read_int().unwrap())
        })
        .collect()
}

fn arrived(t: &Trace, map: i32) -> bool {
    t.arrivals.iter().any(|a| a.0 == map)
}

/// Walk to the waiting room's way on and take it.
async fn take_the_way_on(player: &mut WorldSession) {
    let room = maps().field(WAITING_ROOM).unwrap();
    let portal = room.portal_by_name("join00").unwrap();
    let c = &mut player.connection;
    send(c, walk_to(portal.x as i16, portal.y as i16)).await;
    send(c, build_change_map(-1, "join00").unwrap()).await;
}

#[tokio::test]
async fn guilds_take_their_turns_at_the_guild_quest() {
    let mut a = guild(2).await;
    let mut b = guild(1).await;
    let shuang = || talk(CAMP, SHUANG);

    // Only a master or Jr. Master registers the guild.
    let t = told(&mut a[1], shuang()).await;
    assert_eq!(t.prompts, [(SHUANG, 4)]);
    let t = told(&mut a[1], answer(4, 1, Some(0))).await;
    assert!(said(&t, keys::GUILD_QUEST_NOT_MASTER), "{t:?}");
    told(&mut a[1], answer(0, 1, None)).await;

    // The master does; no one plays, so the guild's turn comes at once and
    // the master goes in. Its members hear they are called.
    told(&mut a[0], shuang()).await;
    let t = told(&mut a[0], answer(4, 1, Some(0))).await;
    assert_eq!(t.prompts, [(SHUANG, 1)]);
    let mut master = told(&mut a[0], answer(1, 1, None)).await;
    assert!(said(&master, keys::GUILD_QUEST_REGISTERED), "{master:?}");
    until(&mut a[0].connection, &mut master, |t| {
        arrived(t, WAITING_ROOM) && waitlist(t) == [(0, 1)]
    })
    .await;

    // The member heard it too, and goes in while the guild gathers.
    let t = told(&mut a[1], shuang()).await;
    assert_eq!(waitlist(&t), [(0, 1)], "{t:?}");
    let mut member = told(&mut a[1], answer(4, 1, Some(1))).await;
    until(&mut a[1].connection, &mut member, |t| {
        arrived(t, WAITING_ROOM)
    })
    .await;
    // The way on is shut until the gathering is over.
    take_the_way_on(&mut a[1]).await;
    until(&mut a[1].connection, &mut member, |t| !t.blocked.is_empty()).await;
    assert!(!arrived(&member, FIRST_STAGE), "{member:?}");

    // Another guild registers, and is next.
    told(&mut b[0], shuang()).await;
    told(&mut b[0], answer(4, 1, Some(0))).await;
    let mut other = told(&mut b[0], answer(1, 1, None)).await;
    until(&mut b[0].connection, &mut other, |t| {
        waitlist(t) == [(0, 2)]
    })
    .await;
    told(&mut b[0], answer(0, 1, None)).await;
    // It may not go in yet.
    told(&mut b[0], shuang()).await;
    let t = told(&mut b[0], answer(4, 1, Some(1))).await;
    assert!(said(&t, keys::GUILD_QUEST_NOT_CALLED), "{t:?}");
    told(&mut b[0], answer(0, 1, None)).await;

    // The gathering is over: the quest's clock starts and the way opens.
    until(&mut a[1].connection, &mut member, |t| {
        t.countdowns.iter().any(|&seconds| seconds > 60)
    })
    .await;
    let mut member = Trace::default();
    take_the_way_on(&mut a[1]).await;
    until(&mut a[1].connection, &mut member, |t| {
        arrived(t, FIRST_STAGE)
    })
    .await;

    // The member leaves, which leaves too few: the master hears it, and is
    // sent to the Returning Path.
    warp(&mut a[1].connection, CAMP, "", &mut member).await;
    let mut master = Trace::default();
    until(&mut a[0].connection, &mut master, |t| {
        t.guild_ops.iter().any(|op| op == &[TOO_FEW])
    })
    .await;
    until(&mut a[0].connection, &mut master, |t| {
        arrived(t, RETURNING_PATH) && waitlist(t) == [(0, 0)]
    })
    .await;

    // The next guild is called, and its registrant, at the camp, goes in.
    let mut other = Trace::default();
    until(&mut b[0].connection, &mut other, |t| {
        arrived(t, WAITING_ROOM) && waitlist(t) == [(0, 1)]
    })
    .await;
    // Nuris takes the master back to the camp.
    let t = told(&mut a[0], talk(RETURNING_PATH, NURIS)).await;
    assert_eq!(t.prompts, [(NURIS, 1)]);
    let mut master = told(&mut a[0], answer(1, 1, None)).await;
    until(&mut a[0].connection, &mut master, |t| arrived(t, CAMP)).await;

    // Alone, the next guild's quest ends once its gathering is over.
    until(&mut b[0].connection, &mut other, |t| {
        t.guild_ops.iter().any(|op| op == &[TOO_FEW])
    })
    .await;
    until(&mut b[0].connection, &mut other, |t| {
        arrived(t, RETURNING_PATH) && waitlist(t) == [(0, 1), (0, 0)]
    })
    .await;
}
