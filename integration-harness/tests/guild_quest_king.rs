//! The Guild Quest's King's Corridor: its door and the crack beside it stay
//! shut till the door takes Sharen III's earring, and then lead on to
//! Ergoth's room. Till then, a member who dies in the quest revives on its
//! way out and leaves it, the rest playing on; from then on, the dead
//! revive on the quest's own maps. Ergoth's altar brings Ergoth and its
//! guards out, and with everyone in the room dead the quest ends; the
//! registrant's Rubian wins it, earning the guild its points, and takes
//! the guild to the treasure room for its half a minute (30 seconds less 60
//! times here, `RUSTMS_EVENT_SCALE`), then out.

use game_data::server_content::keys;
use integration_harness::events::{cleared, go_to, said, talk, told};
use integration_harness::guild_quest::{
    arrived, guild_with, into_the_outskirts, touch_script_portal,
};
use integration_harness::hunting::{hit_reactor, walk_to};
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::WorldSession;
use net::packet::op::RecvOpcode;
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::io::Cursor;

const GRAVE: i32 = 990_000_700;
const KING_CORRIDOR: i32 = 990_000_800;
const KING_ROOM: i32 = 990_000_900;
const RETURN_MAP: i32 = 990_000_200;
const RETURNING_PATH: i32 = 990_001_100;
const KING_GATE: i32 = 9_208_009;
const GHOST_GATE: i32 = 9_208_008;
const JR_GARGOYLE: i32 = 9_300_033;
const EARRINGS_SLOT: i16 = -4;
const EQUIP: u8 = 1;
const PINK: u8 = 5;
const HP: i32 = 0x400;
const ERGOTH_ALTAR: i32 = 9_201_002;
const ERGOTH: i32 = 9_300_028;
const ERGOTH_GUARDS: [i32; 4] = [9_300_031, 9_300_032, 9_300_029, 9_300_030];
const RUBIAN: i32 = 4_001_024;
const RUBIAN_KEEPER: i32 = 9_040_010;
const TREASURE_ROOM: i32 = 990_001_000;
const POINTS_CHANGED: u8 = 0x48;

/// A touch of `mob_id`'s for `damage` (TAKE_DAMAGE from its body).
fn touched(damage: i32, mob_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TakeDamage as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(-1i8 as u8).unwrap();
    p.write_byte(0).unwrap();
    p.write_int(damage).unwrap();
    p.write_int(mob_id).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(0).unwrap();
    p
}

/// Move `quantity` of the item in `kind`'s slot `from` to `to`; to 0 drops
/// it.
fn item_move(kind: u8, from: i16, to: i16, quantity: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemMove as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(kind).unwrap();
    p.write_short(from).unwrap();
    p.write_short(to).unwrap();
    p.write_short(quantity).unwrap();
    p
}

/// `player` is touched till they die.
async fn fall(player: &mut WorldSession) -> Trace {
    let mut seen = Trace::default();
    for _ in 0..40 {
        send(&mut player.connection, touched(20, JR_GARGOYLE)).await;
        read_for(&mut player.connection, &mut seen, 150).await;
        if seen.stat(HP) == Some(0) {
            break;
        }
    }
    assert_eq!(seen.stat(HP), Some(0), "{seen:?}");
    seen
}

/// `player` is touched till they die, and answers the death dialog: where
/// they revive.
async fn die(player: &mut WorldSession, map: i32) -> i32 {
    let mut seen = fall(player).await;
    warp(&mut player.connection, map, "", &mut seen).await
}

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

#[tokio::test]
async fn the_dead_leave_the_quest_till_the_king_s_door_takes_the_earring() {
    let mut team = guild_with(0, 3, 3, &[]).await;
    let there = into_the_outskirts(&mut team).await;
    for player in team.iter_mut() {
        told(player, answer(0, 1, None)).await;
    }
    drop(there);

    // Dead on the grave before the door opens, a member revives on the
    // way out, out of the quest; the rest play on.
    let fallen = &mut team[2];
    go_to(fallen, GRAVE, GHOST_GATE).await;
    assert_eq!(die(fallen, GRAVE).await, RETURNING_PATH);

    // The door and the crack are shut till then.
    let master = &mut team[0];
    let (mut corridor, gate) = go_to(master, KING_CORRIDOR, KING_GATE).await;
    assert_eq!(gate.state, 0);
    shut(
        master,
        KING_CORRIDOR,
        "boss00",
        keys::GUILD_QUEST_GATE_CLOSED,
    )
    .await;
    shut(
        master,
        KING_CORRIDOR,
        "boss01",
        keys::GUILD_QUEST_GATE_CRACK,
    )
    .await;

    // Taken off and thrown down on the door, the earring opens it.
    let c = &mut master.connection;
    send(c, item_move(EQUIP, EARRINGS_SLOT, 1, 1)).await;
    send(c, walk_to(gate.x, gate.y)).await;
    send(c, item_move(EQUIP, 1, 0, 1)).await;
    until(c, &mut corridor, |t| {
        t.field.reactor_hits.contains(&(gate.object_id, 1))
    })
    .await;

    // From then on the dead revive on the quest's maps: back in the
    // corridor, the door stands open.
    let member = &mut team[1];
    go_to(member, KING_CORRIDOR, KING_GATE).await;
    assert_eq!(die(member, KING_CORRIDOR).await, RETURN_MAP);
    let (_, open) = go_to(member, KING_CORRIDOR, KING_GATE).await;
    assert_eq!(open.state, 1);

    // The door and the crack lead to Ergoth's room.
    for (n, name, portal) in [(0, "boss00", 1), (1, "boss01", 2)] {
        let player = &mut team[n];
        touch_script_portal(player, KING_CORRIDOR, name).await;
        let mut t = Trace::default();
        until(&mut player.connection, &mut t, |t| {
            arrived(t, KING_ROOM) && t.arrivals.iter().any(|a| a.1 == portal)
        })
        .await;
    }
}

#[tokio::test]
async fn ergoth_comes_out_of_the_altar_and_the_quest_ends_with_its_room_fallen() {
    // A line of its own beside the door's test.
    let mut team = guild_with(1, 3, 0, &[]).await;
    let there = into_the_outskirts(&mut team).await;
    for player in team.iter_mut() {
        told(player, answer(0, 1, None)).await;
    }
    drop(there);
    let (mut corridor, _) = go_to(&mut team[2], KING_CORRIDOR, KING_GATE).await;
    go_to(&mut team[1], KING_ROOM, ERGOTH_ALTAR).await;

    // The altar's fifth hit plays Ergoth's music and brings it out with
    // its four guards.
    let master = &mut team[0];
    let (mut room, altar) = go_to(master, KING_ROOM, ERGOTH_ALTAR).await;
    let c = &mut master.connection;
    send(c, walk_to(altar.x, altar.y)).await;
    for state in 1..=4u8 {
        send(c, hit_reactor(altar.object_id)).await;
        until(c, &mut room, |t| {
            t.field.reactor_hits.contains(&(altar.object_id, state))
        })
        .await;
    }
    send(c, hit_reactor(altar.object_id)).await;
    let out = |t: &Trace| {
        std::iter::once(ERGOTH)
            .chain(ERGOTH_GUARDS)
            .all(|mob| t.field.alive(mob).len() == 1)
    };
    until(c, &mut room, out).await;
    let music = (6, "Bgm10/Eregos".to_string());
    assert!(room.field_effects.contains(&music), "{room:?}");

    // With one of the two in the room dead the quest goes on; with both,
    // and Ergoth alive, it ends, and even the one outside goes out.
    fall(&mut team[1]).await;
    fall(&mut team[0]).await;
    until(&mut team[2].connection, &mut corridor, |t| {
        arrived(t, RETURNING_PATH)
    })
    .await;
}

#[tokio::test]
async fn the_registrant_s_rubian_wins_the_quest_and_the_treasure_room() {
    let mut team = guild_with(2, 2, 0, &[(RUBIAN, 1)]).await;
    let there = into_the_outskirts(&mut team).await;
    for player in team.iter_mut() {
        told(player, answer(0, 1, None)).await;
    }
    drop(there);

    // Only the registrant hands it in.
    let (mut room, _) = go_to(&mut team[1], KING_ROOM, ERGOTH_ALTAR).await;
    let t = told(&mut team[1], talk(KING_ROOM, RUBIAN_KEEPER)).await;
    assert!(said(&t, keys::GUILD_QUEST_RUBIAN_NOT_LEADER), "{t:?}");
    told(&mut team[1], answer(0, 1, None)).await;

    // The registrant's wins the quest, with no word said, earning the
    // guild a point less than 1000 for every six seconds since it was set
    // up, and the guild goes to the treasure room.
    let master = &mut team[0];
    let mut t = Trace::default();
    go_to(master, KING_ROOM, ERGOTH_ALTAR).await;
    send(&mut master.connection, talk(KING_ROOM, RUBIAN_KEEPER)).await;
    until(&mut master.connection, &mut t, |t| {
        cleared(t)
            && (t.guild_ops.iter()).any(|op| op[0] == POINTS_CHANGED)
            && arrived(t, TREASURE_ROOM)
    })
    .await;
    assert!(t.prompts.is_empty(), "{t:?}");
    let op = (t.guild_ops.iter()).find(|op| op[0] == POINTS_CHANGED);
    let mut r = Cursor::new(&op.unwrap()[1..]);
    let (_guild, total) = (r.read_int().unwrap(), r.read_int().unwrap());
    assert!((950..=1000).contains(&total), "{total} {t:?}");

    // Its time up, they go out.
    until(&mut master.connection, &mut t, |t| {
        arrived(t, RETURNING_PATH)
    })
    .await;
    until(&mut team[1].connection, &mut room, |t| {
        arrived(t, TREASURE_ROOM) && arrived(t, RETURNING_PATH)
    })
    .await;
}
