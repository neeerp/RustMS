//! The Guild Quest's way in, shared by its tests: a guild of new
//! characters, its registration with Shuang, and the walk past the waiting
//! room to the outskirts.

use crate::database::{found_guild, grant_stack, join_guild, wear_equip};
use crate::events::{maps, talk, told};
use crate::hunting::walk_to;
use crate::packets::build_change_map;
use crate::preconditions::load_harness_config_or_fail;
use crate::quests::answer;
use crate::trace::{send, until, warp, Trace};
use crate::{login_to_world_session, relogin, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

pub const CAMP: i32 = 101_030_104;
pub const WAITING_ROOM: i32 = 990_000_000;
pub const OUTSKIRTS: i32 = 990_000_100;
pub const SHUANG: i32 = 9_040_000;
/// The rocks on the outskirts, which drop the earring.
pub const ROCK: i32 = 9_202_000;
pub const EARRING: i32 = 1_032_033;
pub const EARRINGS_SLOT: i16 = -4;
/// The Etc inventory.
const ETC: i16 = 4;

/// New characters, logged in at the camp on channel `channel` (a line of
/// its own, so the tests don't wait on each other's runs): the first
/// founds a guild named after them, the rest join it, and the last `worn`
/// wear the earring.
pub async fn guild_on(channel: u8, size: usize, worn: usize) -> Vec<WorldSession> {
    guild_with(channel, size, worn, &[]).await
}

/// As `guild_on`, the first holding `items` too, in their Etc slots from 1.
pub async fn guild_with(
    channel: u8,
    size: usize,
    worn: usize,
    items: &[(i32, i16)],
) -> Vec<WorldSession> {
    let mut configs = vec![];
    let mut ids = vec![];
    for _ in 0..size {
        let config = load_harness_config_or_fail().await.with_channel(0, channel);
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
    for &id in &ids[size - worn..] {
        wear_equip(id, EARRINGS_SLOT, EARRING).await.unwrap();
    }
    for (slot, &(item_id, quantity)) in (1..).zip(items) {
        grant_stack(ids[0], ETC, slot, item_id, quantity)
            .await
            .unwrap();
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

pub fn arrived(t: &Trace, map: i32) -> bool {
    t.arrivals.iter().any(|a| a.0 == map)
}

/// Walk to the waiting room's way on and take it.
pub async fn take_the_way_on(player: &mut WorldSession) {
    let room = maps().field(WAITING_ROOM).unwrap();
    let portal = room.portal_by_name("join00").unwrap();
    let c = &mut player.connection;
    send(c, walk_to(portal.x as i16, portal.y as i16)).await;
    send(c, build_change_map(-1, "join00").unwrap()).await;
}

/// Walks `player` to `map`'s script portal `name` and enters it.
pub async fn touch_script_portal(player: &mut WorldSession, map: i32, name: &str) {
    let portal = maps().field(map).unwrap().portal_by_name(name).unwrap();
    let c = &mut player.connection;
    send(c, walk_to(portal.x as i16, portal.y as i16)).await;
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(name).unwrap();
    p.write_short(0).unwrap();
    send(c, p).await;
}

/// The first registers the guild with Shuang, the rest join, and once the
/// gathering is over all go on to the outskirts, where Shuang's brief
/// waits for each, still open.
pub async fn into_the_outskirts(team: &mut [WorldSession]) -> Vec<Trace> {
    let shuang = || talk(CAMP, SHUANG);
    told(&mut team[0], shuang()).await;
    told(&mut team[0], answer(4, 1, Some(0))).await;
    let mut master = told(&mut team[0], answer(1, 1, None)).await;
    until(&mut team[0].connection, &mut master, |t| {
        arrived(t, WAITING_ROOM)
    })
    .await;
    let mut seen = vec![master];
    for player in &mut team[1..] {
        told(player, shuang()).await;
        let mut t = told(player, answer(4, 1, Some(1))).await;
        until(&mut player.connection, &mut t, |t| arrived(t, WAITING_ROOM)).await;
        seen.push(t);
    }
    for (player, t) in team.iter_mut().zip(&mut seen) {
        until(&mut player.connection, t, |t| {
            t.countdowns.iter().any(|&seconds| seconds > 60)
        })
        .await;
    }
    let mut there = vec![];
    for player in team.iter_mut() {
        let mut t = Trace::default();
        take_the_way_on(player).await;
        // The field's spawns may come after Shuang's brief.
        until(&mut player.connection, &mut t, |t| {
            arrived(t, OUTSKIRTS)
                && !t.prompts.is_empty()
                && t.field.reactors.iter().any(|r| r.reactor_id == ROCK)
        })
        .await;
        there.push(t);
    }
    there
}
