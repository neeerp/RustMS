//! The Guild Quest's Underground Waterway: each of its rooms' doors stays
//! shut until a stack of exactly so many Marks of the guild is set in it,
//! and then lets the guild through to the room past it. The gargoyles'
//! altar takes the Gargoyle's drop and brings out the Jr. Gargoyles whose
//! Marks open the second door; the last room lets only the low levels in,
//! and the grave's gate stays shut till Sharen III's soul opens it.

use game_data::server_content::keys;
use integration_harness::events::{go_to, throw_down, told};
use integration_harness::guild_quest::{
    arrived, guild_with, into_the_outskirts, touch_script_portal,
};
use integration_harness::hunting::{walk_to, MapReactor};
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::WorldSession;

const WATERWAY: i32 = 990_000_600;
const FIRST_ROOM: i32 = 990_000_610;
const GARGOYLES_ROOM: i32 = 990_000_630;
const LAST_ROOM: i32 = 990_000_640;
const GRAVE: i32 = 990_000_700;
const FIRST_DOOR: i32 = 9_208_010;
const GARGOYLE_ALTAR: i32 = 9_201_000;
const GHOST_GATE: i32 = 9_208_008;
const MARK: i32 = 4_001_035;
const GARGOYLE_DROP: i32 = 4_001_036;
const JR_GARGOYLE: i32 = 9_300_033;
const PINK: u8 = 5;

/// `player` touches `map`'s script portal `name` and hears `key` in pink
/// instead of going through.
async fn shut(player: &mut WorldSession, map: i32, name: &str, key: &str) {
    touch_script_portal(player, map, name).await;
    let text = net::content::text().get(key, &[]);
    let mut t = Trace::default();
    until(&mut player.connection, &mut t, |t| {
        t.broadcasts.contains(&(PINK, text.clone()))
    })
    .await;
    assert!(t.warps.is_empty(), "{t:?}");
}

/// `player` throws `quantity` of the Etc stack in `slot` down on `reactor`.
async fn set_on(player: &mut WorldSession, slot: i16, quantity: i16, reactor: &MapReactor) {
    let c = &mut player.connection;
    send(c, walk_to(reactor.x, reactor.y)).await;
    send(c, throw_down(slot, quantity)).await;
}

/// `player` goes through `map`'s script portal `name` to `to`.
async fn through(player: &mut WorldSession, map: i32, name: &str, to: i32) {
    touch_script_portal(player, map, name).await;
    let mut t = Trace::default();
    until(&mut player.connection, &mut t, |t| arrived(t, to)).await;
}

#[tokio::test]
async fn the_waterway_s_doors_open_on_their_marks() {
    let items = [(MARK, 8), (MARK, 9), (GARGOYLE_DROP, 1)];
    let mut team = guild_with(0, 2, 2, &items).await;
    let there = into_the_outskirts(&mut team).await;
    for player in team.iter_mut() {
        told(player, answer(0, 1, None)).await;
    }
    drop(there);

    // The first room's door is shut, and a stack one short doesn't open
    // it; the stack of nine does, and it lets the guild through.
    let master = &mut team[0];
    let (mut room, door) = go_to(master, FIRST_ROOM, FIRST_DOOR).await;
    shut(master, FIRST_ROOM, "in00", keys::GUILD_QUEST_GATE_BLOCKED).await;
    set_on(master, 1, 8, &door).await;
    until(&mut master.connection, &mut room, |t| {
        t.field.item_drop(MARK).is_some()
    })
    .await;
    let short = room.field.item_drop(MARK).unwrap();
    set_on(master, 2, 9, &door).await;
    until(&mut master.connection, &mut room, |t| {
        t.field.reactor_hits.contains(&(door.object_id, 1))
    })
    .await;
    assert!(room.field.removals(short.object_id).is_empty(), "{room:?}");
    through(master, FIRST_ROOM, "in00", FIRST_ROOM + 1).await;

    // The gargoyles' altar takes the Gargoyle's drop, brings out the
    // eight Jr. Gargoyles and goes; their door is shut till their Marks
    // are set.
    let (mut room, altar) = go_to(master, GARGOYLES_ROOM, GARGOYLE_ALTAR).await;
    shut(
        master,
        GARGOYLES_ROOM,
        "in00",
        keys::GUILD_QUEST_GATE_BLOCKED,
    )
    .await;
    set_on(master, 3, 1, &altar).await;
    until(&mut master.connection, &mut room, |t| {
        t.field.alive(JR_GARGOYLE).len() >= 8
    })
    .await;
    assert_eq!(room.field.alive(JR_GARGOYLE).len(), 8, "{room:?}");
    assert!(
        room.field.reactors_removed.contains(&(altar.object_id, 1)),
        "{room:?}"
    );

    // A new character may take the way to the last room; the grave's gate
    // is shut.
    let low = &mut team[1];
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut low.connection, WATERWAY, "", &mut seen).await,
        WATERWAY
    );
    through(low, WATERWAY, "in03", LAST_ROOM).await;
    go_to(low, GRAVE, GHOST_GATE).await;
    shut(low, GRAVE, "next00", keys::GUILD_QUEST_GATE_CLOSED).await;
}
