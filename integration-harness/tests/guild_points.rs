//! Guild points (GP): a GM's `!gp` adds to their guild's points, every
//! member online hears the new total (GUILD_OPERATION 0x48) with the chat
//! line for the points gained (SHOW_STATUS_INFO mode 6), the total is kept,
//! and it never drops below none.

use integration_harness::database::{found_guild, join_guild, set_gm_level};
use integration_harness::packets::build_all_chat;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, until, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::io::read::PktRead;
use std::io::Cursor;
use tokio::time::Duration;

const POINTS_CHANGED: u8 = 0x48;
/// GUILD_OPERATION 0x1A: the guild window, sent on entering the game.
const INFO: u8 = 0x1A;

/// The guild and total of a 0x48.
fn points(op: &[u8]) -> (i32, i32) {
    let mut r = Cursor::new(&op[1..]);
    (r.read_int().unwrap(), r.read_int().unwrap())
}

/// The totals `t` heard, in order.
fn totals(t: &Trace) -> Vec<i32> {
    t.guild_ops
        .iter()
        .filter(|op| op[0] == POINTS_CHANGED)
        .map(|op| points(op).1)
        .collect()
}

#[tokio::test]
async fn guild_points_reach_every_member_online() {
    let (config, other) = (
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    );
    let mut ids = vec![];
    for config in [&config, &other] {
        let session = login_to_world_session(config).await.expect("first login");
        ids.push(session.character_id);
        drop(session);
        drop(relogin(config).await);
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    let guild = &config.character_name;
    found_guild(ids[0], guild).await.expect("found a guild");
    join_guild(ids[1], guild).await.expect("join it");
    set_gm_level(&config.username, 2).await.expect("make a GM");

    let mut gm = relogin(&config).await;
    let mut member = relogin(&other).await;
    let (g, m) = (&mut gm.connection, &mut member.connection);
    let (mut gt, mut mt) = (Trace::default(), Trace::default());
    until(g, &mut gt, |t| t.guild_ops.iter().any(|op| op[0] == INFO)).await;
    until(m, &mut mt, |t| t.guild_ops.iter().any(|op| op[0] == INFO)).await;

    send(g, build_all_chat("!gp 150", 0).unwrap()).await;
    for (c, t) in [(&mut *g, &mut gt), (&mut *m, &mut mt)] {
        until(c, t, |t| {
            !totals(t).is_empty() && !t.guild_point_gains.is_empty()
        })
        .await;
        assert_eq!(totals(t), [150]);
        assert_eq!(t.guild_point_gains, [150]);
        let op = t
            .guild_ops
            .iter()
            .find(|op| op[0] == POINTS_CHANGED)
            .unwrap();
        assert_eq!(op.len(), 9, "guild id and total only");
    }

    // Taking more than the guild has leaves none.
    let (mut gt, mut mt) = (Trace::default(), Trace::default());
    send(g, build_all_chat("!gp -200", 0).unwrap()).await;
    for (c, t) in [(&mut *g, &mut gt), (&mut *m, &mut mt)] {
        until(c, t, |t| {
            !totals(t).is_empty() && !t.guild_point_gains.is_empty()
        })
        .await;
        assert_eq!(totals(t), [0]);
        assert_eq!(t.guild_point_gains, [-200]);
    }

    // The total is kept: the guild window on entering the game has it.
    send(g, build_all_chat("!gp 40", 0).unwrap()).await;
    let mut mt = Trace::default();
    until(m, &mut mt, |t| totals(t) == [40]).await;
    drop(member);
    tokio::time::sleep(Duration::from_millis(300)).await;
    let mut member = relogin(&other).await;
    let mut mt = Trace::default();
    until(&mut member.connection, &mut mt, |t| {
        t.guild_ops.iter().any(|op| op[0] == INFO)
    })
    .await;
    let info = mt.guild_ops.iter().find(|op| op[0] == INFO).unwrap();
    // The window ends with the points and the alliance id.
    let tail = &info[info.len() - 8..info.len() - 4];
    assert_eq!(i32::from_le_bytes(tail.try_into().unwrap()), 40);
}
