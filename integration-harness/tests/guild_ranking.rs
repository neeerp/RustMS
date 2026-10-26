//! The guild rank boards (row 198 R4): clicking the Honorable Rock in
//! Perion or the Guild Rank Board at the Excavation Site's camp opens the
//! client's guild ranking (GUILD_OPERATION 0x49) with the world's guilds,
//! the most points first, each with its emblem.

use integration_harness::database::{delete_guild, found_guild, rank_guild};
use integration_harness::events::talk;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::io::read::PktRead;
use std::io::Cursor;
use tokio::time::Duration;

const PERION: i32 = 102000000;
const HONORABLE_ROCK: i32 = 9040004;
const EXCAVATION_CAMP: i32 = 101030104;
const GUILD_RANK_BOARD: i32 = 9040008;
const RANKING: u8 = 0x49;

/// A guild as the ranking lists it: name, points, and the mark, its
/// colour, the frame and its colour.
type Standing = (String, i32, [i32; 4]);

fn ranking(op: &[u8]) -> (i32, Vec<Standing>) {
    let mut r = Cursor::new(&op[1..]);
    let npc = r.read_int().unwrap();
    let count = r.read_int().unwrap();
    let guilds = (0..count)
        .map(|_| {
            let name = r.read_str_with_length().unwrap();
            let points = r.read_int().unwrap();
            let emblem = [(); 4].map(|_| r.read_int().unwrap());
            (name, points, emblem)
        })
        .collect();
    assert_eq!(
        r.position() as usize,
        op.len() - 1,
        "nothing after the list"
    );
    (npc, guilds)
}

async fn read_ranking(
    c: &mut integration_harness::connection::MapleTestConnection,
) -> (i32, Vec<Standing>) {
    let mut t = Trace::default();
    until(c, &mut t, |t| t.guild_ops.iter().any(|op| op[0] == RANKING)).await;
    ranking(t.guild_ops.iter().find(|op| op[0] == RANKING).unwrap())
}

#[tokio::test]
async fn the_rank_boards_list_the_guilds_with_the_most_points() {
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
    // A character name is a valid guild name, and unique.
    let (high, low) = (&config.character_name, &other.character_name);
    found_guild(ids[0], high).await.expect("found a guild");
    found_guild(ids[1], low).await.expect("found another");
    rank_guild(high, 2_000_000_000, (1003, 5, 4010, 12))
        .await
        .expect("rank it");
    rank_guild(low, 1_999_999_999, (1000, 1, 2000, 2))
        .await
        .expect("rank the other");

    let mut player = relogin(&config).await;
    let c = &mut player.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, PERION, "", &mut t).await, PERION);
    send(c, talk(PERION, HONORABLE_ROCK)).await;
    let (npc, guilds) = read_ranking(c).await;
    assert_eq!(npc, HONORABLE_ROCK);
    assert!(guilds.len() <= 50, "{}", guilds.len());
    assert!(guilds.windows(2).all(|w| w[0].1 >= w[1].1), "{guilds:?}");
    let ours: Vec<_> = guilds
        .iter()
        .filter(|g| g.0 == *high || g.0 == *low)
        .cloned()
        .collect();
    assert_eq!(
        ours,
        [
            (high.clone(), 2_000_000_000, [4010, 12, 1003, 5]),
            (low.clone(), 1_999_999_999, [2000, 2, 1000, 1]),
        ]
    );

    // The camp's board shows the same, by its own id.
    let mut t = Trace::default();
    assert_eq!(warp(c, EXCAVATION_CAMP, "", &mut t).await, EXCAVATION_CAMP);
    send(c, talk(EXCAVATION_CAMP, GUILD_RANK_BOARD)).await;
    let (npc, again) = read_ranking(c).await;
    assert_eq!(npc, GUILD_RANK_BOARD);
    assert!(again.contains(&ours[0]) && again.contains(&ours[1]));

    drop(player);
    delete_guild(high).await.expect("clean up");
    delete_guild(low).await.expect("clean up");
}
