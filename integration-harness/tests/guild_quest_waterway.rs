//! The Guild Quest's Underground Waterway: each of its rooms' doors stays
//! shut until a stack of exactly so many Marks of the guild is set in it,
//! and then lets the guild through to the room past it. The gargoyles'
//! altar takes the Gargoyle's drop and brings out the Jr. Gargoyles whose
//! Marks open the second door; the last room lets only the low levels in,
//! and the grave's gate stays shut till Sharen III's soul opens it.
//! Each room's box gives up a piece of Sharen III's clothes, the end
//! rooms' ways lead to the grave and its way back beside the room last
//! come from, and the clothes set on the grave's altar in turn bring out
//! the soul, whose word to the registrant clears the stage, earns the
//! guild 30 points and opens the gate.

use game_data::server_content::keys;
use integration_harness::events::{cleared, go_to, said, talk_to, throw_down, told};
use integration_harness::guild_quest::{
    arrived, guild_with, into_the_outskirts, touch_script_portal,
};
use integration_harness::hunting::{hit_reactor, pickup, walk_to, MapReactor};
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::WorldSession;
use packet::io::read::PktRead;
use std::io::Cursor;

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
const SHOE_ROOM: i32 = 990_000_620;
const SHOE_BOX: i32 = 9_202_006;
const CLOTHES_ALTAR: i32 = 9_201_001;
const SHAREN_SOUL: i32 = 9_040_003;
const SHOES: i32 = 4_001_032;
const PANTS: i32 = 4_001_031;
const TOP: i32 = 4_001_033;
const CROWN: i32 = 4_001_034;
const POINTS_CHANGED: u8 = 0x48;

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

/// Whether `t` saw `player` arrive on `map` at its portal `portal`.
fn arrived_at(t: &Trace, map: i32, portal: u8) -> bool {
    t.arrivals.iter().any(|a| (a.0, a.1) == (map, portal))
}

#[tokio::test]
async fn sharen_iii_s_soul_opens_the_grave_s_gate() {
    let items = [(SHOES, 1), (PANTS, 1), (TOP, 1), (CROWN, 1)];
    // A line of its own beside the waterway's test.
    let mut team = guild_with(1, 2, 2, &items).await;
    let there = into_the_outskirts(&mut team).await;
    for player in team.iter_mut() {
        told(player, answer(0, 1, None)).await;
    }
    drop(there);

    // The shoes' box gives them up on its fifth hit.
    let member = &mut team[1];
    let (mut room, shoe_box) = go_to(member, SHOE_ROOM, SHOE_BOX).await;
    let c = &mut member.connection;
    send(c, walk_to(shoe_box.x, shoe_box.y)).await;
    for state in 1..=4u8 {
        send(c, hit_reactor(shoe_box.object_id)).await;
        until(c, &mut room, |t| {
            t.field.reactor_hits.contains(&(shoe_box.object_id, state))
        })
        .await;
    }
    send(c, hit_reactor(shoe_box.object_id)).await;
    until(c, &mut room, |t| t.field.item_drop(SHOES).is_some()).await;

    // The room's way leads to the grave, and the grave's way back beside
    // the room; one who never came through a room is taken back to the
    // waterway's start.
    let to_grave = |t: &Trace| arrived_at(t, GRAVE, 1);
    touch_script_portal(member, SHOE_ROOM, "guild1F00").await;
    let mut t = Trace::default();
    until(&mut member.connection, &mut t, to_grave).await;
    touch_script_portal(member, GRAVE, "out00").await;
    until(&mut member.connection, &mut t, |t| {
        arrived_at(t, WATERWAY, 8)
    })
    .await;
    let master = &mut team[0];
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut master.connection, GRAVE, "", &mut seen).await,
        GRAVE
    );
    touch_script_portal(master, GRAVE, "out00").await;
    until(&mut master.connection, &mut seen, |t| {
        arrived_at(t, WATERWAY, 1)
    })
    .await;

    // The altar takes no piece out of turn: the pants first stay put.
    let (mut grave, altar) = go_to(&mut team[1], GRAVE, CLOTHES_ALTAR).await;
    let master = &mut team[0];
    let (mut seen, _) = go_to(master, GRAVE, CLOTHES_ALTAR).await;
    let set = |slot| (walk_to(altar.x, altar.y), throw_down(slot, 1));
    for slot in [2, 1] {
        let (walk, throw) = set(slot);
        send(&mut master.connection, walk).await;
        send(&mut master.connection, throw).await;
    }
    until(&mut master.connection, &mut seen, |t| {
        t.field.reactor_hits.contains(&(altar.object_id, 1))
    })
    .await;
    let pants = seen.field.item_drop(PANTS).unwrap();
    assert!(seen.field.removals(pants.object_id).is_empty(), "{seen:?}");

    // Picked back up (into the first free slot) and set in turn with the
    // rest, the clothes bring out the soul.
    let c = &mut master.connection;
    send(c, pickup(pants.object_id, pants.x, pants.y)).await;
    until(c, &mut seen, |t| {
        !t.field.removals(pants.object_id).is_empty()
    })
    .await;
    for slot in [1, 3, 4] {
        let (walk, throw) = set(slot);
        send(c, walk).await;
        send(c, throw).await;
        let state = if slot == 1 { 2 } else { slot as u8 };
        until(c, &mut seen, |t| {
            t.field.reactor_hits.contains(&(altar.object_id, state))
                || t.field.npc(SHAREN_SOUL).is_some()
        })
        .await;
    }
    let notice = net::content::text().get(keys::GUILD_QUEST_SOUL_APPEARS, &[]);
    until(&mut team[1].connection, &mut grave, |t| {
        t.field.npc(SHAREN_SOUL).is_some() && t.broadcasts.contains(&(PINK, notice.clone()))
    })
    .await;
    let soul = grave.field.npc(SHAREN_SOUL).unwrap();

    // Only the registrant is heard; the gate is shut till then.
    shut(&mut team[1], GRAVE, "next00", keys::GUILD_QUEST_GATE_CLOSED).await;
    let t = told(&mut team[1], talk_to(soul)).await;
    assert!(said(&t, keys::GUILD_QUEST_SOUL_NOT_LEADER), "{t:?}");
    told(&mut team[1], answer(0, 1, None)).await;

    // The registrant's talk clears the stage, earns the guild its points
    // and opens the gate.
    let master = &mut team[0];
    let mut t = told(master, talk_to(soul)).await;
    assert!(said(&t, keys::GUILD_QUEST_SOUL_OPENED), "{t:?}");
    send(&mut master.connection, answer(0, 1, None)).await;
    until(&mut master.connection, &mut t, |t| {
        cleared(t) && (t.guild_ops.iter()).any(|op| op[0] == POINTS_CHANGED)
    })
    .await;
    let op = (t.guild_ops.iter()).find(|op| op[0] == POINTS_CHANGED);
    let mut r = Cursor::new(&op.unwrap()[1..]);
    let (_guild, total) = (r.read_int().unwrap(), r.read_int().unwrap());
    assert_eq!(total, 30, "{t:?}");
    let t = told(master, talk_to(soul)).await;
    assert!(said(&t, keys::GUILD_QUEST_SOUL_CLEARED), "{t:?}");
    told(master, answer(0, 1, None)).await;
    through(&mut team[1], GRAVE, "next00", GRAVE + 100).await;
}
