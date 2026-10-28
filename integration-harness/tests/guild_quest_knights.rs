//! The Guild Quest's knights' hall: its gates stay shut until the guild
//! sets what each needs. A spear thrown on each of the four spears in the
//! room of the spears moves the hall's gate a state; the fourth opens it,
//! clears the stage, shown on the hall and the maps off it, and earns the
//! guild 20 points. A stone opens the hall's door to the metal room, and a
//! piece of metal the metal room's door on. Everyone wears the earring
//! against the fortress's air.

use game_data::server_content::keys;
use integration_harness::events::{cleared, go_to, throw_down, told};
use integration_harness::guild_quest::{
    arrived, guild_with, into_the_outskirts, touch_script_portal,
};
use integration_harness::hunting::{walk_to, MapReactor};
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, Trace};
use integration_harness::WorldSession;
use packet::io::read::PktRead;
use std::io::Cursor;

const KNIGHT_HALL: i32 = 990_000_400;
const SPEAR_ROOM: i32 = 990_000_440;
const METAL_ROOM: i32 = 990_000_430;
const SPEAR_GATE: i32 = 9_208_004;
const METAL_GATE: i32 = 9_208_005;
const STONE_GATE: i32 = 9_208_006;
const SPEAR: i32 = 9_208_007;
const SPEARS: i32 = 4_001_025;
const STONE: i32 = 4_001_026;
const METAL: i32 = 4_001_037;
const PINK: u8 = 5;
const POINTS_CHANGED: u8 = 0x48;

/// Whether `t` saw `key` in pink.
fn pink(t: &Trace, key: &str) -> bool {
    let text = net::content::text().get(key, &[]);
    t.broadcasts.contains(&(PINK, text))
}

/// `player` touches `map`'s script portal `name` and hears `key` instead
/// of going through.
async fn shut(player: &mut WorldSession, map: i32, name: &str, key: &str) {
    touch_script_portal(player, map, name).await;
    let mut t = Trace::default();
    until(&mut player.connection, &mut t, |t| pink(t, key)).await;
    assert!(t.warps.is_empty(), "{t:?}");
}

/// The states `t` saw `reactor` move to, in turn.
fn states(t: &Trace, reactor: &MapReactor) -> Vec<u8> {
    (t.field.reactor_hits.iter())
        .filter(|&&(oid, _)| oid == reactor.object_id)
        .map(|&(_, state)| state)
        .collect()
}

/// `player` throws one of the Etc stack in `slot` on `reactor`, which
/// moves to state 1 and stays.
async fn set(player: &mut WorldSession, seen: &mut Trace, slot: i16, reactor: &MapReactor) {
    let c = &mut player.connection;
    send(c, walk_to(reactor.x, reactor.y)).await;
    send(c, throw_down(slot, 1)).await;
    until(c, seen, |t| {
        t.field.reactor_hits.contains(&(reactor.object_id, 1))
    })
    .await;
}

#[tokio::test]
async fn the_knights_hall_opens_on_the_spears_the_stone_and_the_metal() {
    let items = [(SPEARS, 4), (STONE, 1), (METAL, 1)];
    let mut team = guild_with(0, 2, 2, &items).await;
    let there = into_the_outskirts(&mut team).await;
    for player in team.iter_mut() {
        told(player, answer(0, 1, None)).await;
    }
    drop(there);

    // The hall's gate and door are shut.
    let (mut hall, gate) = go_to(&mut team[1], KNIGHT_HALL, SPEAR_GATE).await;
    shut(
        &mut team[1],
        KNIGHT_HALL,
        "next00",
        keys::GUILD_QUEST_GATE_CLOSED,
    )
    .await;
    shut(
        &mut team[1],
        KNIGHT_HALL,
        "in02",
        keys::GUILD_QUEST_GATE_BLOCKED,
    )
    .await;

    // Each spear set moves the gate; the last clears the stage, here and
    // in the hall, and earns the guild its points.
    let (mut room, _) = go_to(&mut team[0], SPEAR_ROOM, SPEAR).await;
    until(&mut team[0].connection, &mut room, |t| {
        (t.field.reactors.iter())
            .filter(|r| r.reactor_id == SPEAR)
            .count()
            >= 4
    })
    .await;
    let spears: Vec<MapReactor> = (room.field.reactors.iter())
        .filter(|r| r.reactor_id == SPEAR)
        .copied()
        .collect();
    assert_eq!(spears.len(), 4, "{room:?}");
    for (n, spear) in spears.iter().enumerate() {
        set(&mut team[0], &mut room, 1, spear).await;
        let moved = n as u8 + 1;
        until(&mut team[1].connection, &mut hall, |t| {
            states(t, &gate).len() >= usize::from(moved)
        })
        .await;
    }
    let c = &mut team[0].connection;
    until(c, &mut room, |t| {
        cleared(t) && (t.guild_ops.iter()).any(|op| op[0] == POINTS_CHANGED)
    })
    .await;
    let op = (room.guild_ops.iter()).find(|op| op[0] == POINTS_CHANGED);
    let mut r = Cursor::new(&op.unwrap()[1..]);
    let (_guild, total) = (r.read_int().unwrap(), r.read_int().unwrap());
    assert_eq!(total, 20, "{room:?}");
    until(&mut team[1].connection, &mut hall, cleared).await;
    assert_eq!(states(&hall, &gate), [1, 2, 3, 4], "{hall:?}");

    // The gate lets the guild on.
    touch_script_portal(&mut team[1], KNIGHT_HALL, "next00").await;
    let mut on = Trace::default();
    until(&mut team[1].connection, &mut on, |t| {
        arrived(t, KNIGHT_HALL + 1)
    })
    .await;

    // A stone opens the door to the metal room, and a piece of metal the
    // door past it.
    let master = &mut team[0];
    let (mut hall, stone_gate) = go_to(master, KNIGHT_HALL, STONE_GATE).await;
    set(master, &mut hall, 2, &stone_gate).await;
    touch_script_portal(master, KNIGHT_HALL, "in02").await;
    let mut metal_room = Trace::default();
    until(&mut master.connection, &mut metal_room, |t| {
        arrived(t, METAL_ROOM) && t.field.reactors.iter().any(|r| r.reactor_id == METAL_GATE)
    })
    .await;
    shut(master, METAL_ROOM, "in00", keys::GUILD_QUEST_GATE_CLOSED).await;
    let metal_gate = *(metal_room.field.reactors.iter())
        .find(|r| r.reactor_id == METAL_GATE)
        .unwrap();
    set(master, &mut metal_room, 3, &metal_gate).await;
    touch_script_portal(master, METAL_ROOM, "in00").await;
    let mut past = Trace::default();
    until(&mut master.connection, &mut past, |t| {
        arrived(t, METAL_ROOM + 1)
    })
    .await;
}
