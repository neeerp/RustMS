//! Reactors that take an item put on them, a few seconds after it lands:
//! the fourth job's resurrection mound takes a Cold Flame, then the Black
//! Book, and gives Caron's Note; the Fire Hawk's egg takes its egg, then
//! three hits, and hatches the Fire Hawk. Both rooms are reached by their
//! quest's way in.

use integration_harness::database::{grant_item, set_quest};
use integration_harness::hunting::{hit_reactor, walk_to, MapReactor};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const ETC: i16 = 4;
const AQUA_ROAD: i32 = 230040300;
const RESURRECTION_ROOM: i32 = 923000100;
const MOUND: i32 = 2302003;
const COLD_FLAME: i32 = 4001108;
const BLACK_BOOK: i32 = 4001107;
const CARONS_NOTE: i32 = 4161017;
const LEAFRE_FOREST: i32 = 240010700;
const EGG_NEST: i32 = 924000100;
const FIRE_HAWK_EGG: i32 = 2401001;
const FIRE_HAWKS_EGG: i32 = 4001113;
const FIRE_HAWK: i32 = 9300089;

fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// Drop one of what lies in the Etc tab's `slot`.
fn throw(slot: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemMove as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(ETC as u8).unwrap();
    p.write_short(slot).unwrap();
    p.write_short(0).unwrap();
    p.write_short(1).unwrap();
    p
}

/// Go to `map`, take its way in `portal` to `room`, and find the reactor
/// `reactor_id` there.
async fn go_in(
    player: &mut WorldSession,
    (map, portal): (i32, &str),
    room: i32,
    reactor_id: i32,
) -> (Trace, MapReactor) {
    let mut trace = Trace::default();
    assert_eq!(warp(&mut player.connection, map, "", &mut trace).await, map);
    let name = player.character_name.clone();
    let mut trace = step(&mut player.connection, &name, enter(portal)).await;
    let [(arrived, 0, _)] = trace.arrivals[..] else {
        panic!("{:?}", trace.arrivals);
    };
    assert_eq!(arrived, room);
    let c = &mut player.connection;
    until(c, &mut trace, |t| {
        t.field.reactors.iter().any(|r| r.reactor_id == reactor_id)
    })
    .await;
    let reactor = trace
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == reactor_id);
    let reactor = *reactor.unwrap();
    (trace, reactor)
}

#[tokio::test]
async fn the_mound_and_the_egg_take_what_is_put_on_them() {
    let config = load_harness_config_or_fail().await;
    let player = login_to_world_session(&config).await.unwrap();
    let id = player.character_id;
    drop(player);
    tokio::time::sleep(Duration::from_millis(500)).await;
    for (slot, item) in [(1, COLD_FLAME), (2, BLACK_BOOK), (3, FIRE_HAWKS_EGG)] {
        grant_item(id, ETC, slot, item).await.unwrap();
    }
    set_quest(id, 6241, 1, "").await.unwrap();
    let mut player = relogin(&config).await;

    // The Cold Flame opens Aqua Road's way to the mound.
    let (mut trace, mound) =
        go_in(&mut player, (AQUA_ROAD, "in01"), RESURRECTION_ROOM, MOUND).await;
    let c = &mut player.connection;
    send(c, walk_to(105, 78)).await;
    read_for(c, &mut trace, 300).await;
    // The book first, which it doesn't want yet, then the flame.
    send(c, throw(2)).await;
    send(c, throw(1)).await;
    until(c, &mut trace, |t| {
        t.field.item_drop(BLACK_BOOK).is_some() && t.field.item_drop(COLD_FLAME).is_some()
    })
    .await;
    let book = trace.field.item_drop(BLACK_BOOK).unwrap().object_id;
    let flame = trace.field.item_drop(COLD_FLAME).unwrap().object_id;
    read_for(c, &mut trace, 4000).await;
    assert!(trace.field.reactor_hits.is_empty(), "{trace:?}");
    until(c, &mut trace, |t| {
        t.field.reactor_hits.contains(&(mound.object_id, 1))
    })
    .await;
    assert_eq!(trace.field.removals(flame), [None]);
    assert!(trace.field.removals(book).is_empty());
    // Then the book lying there already, and it gives Caron's Note.
    until(c, &mut trace, |t| {
        t.field.reactors_removed.contains(&(mound.object_id, 2))
    })
    .await;
    assert_eq!(trace.field.removals(book), [None]);
    until(c, &mut trace, |t| t.field.item_drop(CARONS_NOTE).is_some()).await;

    // On the Fire Hawk's quest, Leafre's way leads to the nest.
    let (mut trace, egg) = go_in(
        &mut player,
        (LEAFRE_FOREST, "pt00"),
        EGG_NEST,
        FIRE_HAWK_EGG,
    )
    .await;
    let c = &mut player.connection;
    send(c, walk_to(-145, -418)).await;
    // It takes no hit before its egg.
    send(c, hit_reactor(egg.object_id)).await;
    read_for(c, &mut trace, 400).await;
    assert!(trace.field.reactor_hits.is_empty(), "{trace:?}");
    send(c, throw(3)).await;
    until(c, &mut trace, |t| {
        t.field.reactor_hits.contains(&(egg.object_id, 1))
    })
    .await;
    for state in 2..=3u8 {
        send(c, hit_reactor(egg.object_id)).await;
        until(c, &mut trace, |t| {
            t.field.reactor_hits.contains(&(egg.object_id, state))
        })
        .await;
    }
    send(c, hit_reactor(egg.object_id)).await;
    until(c, &mut trace, |t| {
        t.field.reactors_removed.contains(&(egg.object_id, 4))
    })
    .await;
    until(c, &mut trace, |t| {
        t.field.mobs.iter().any(|&(_, mob)| mob == FIRE_HAWK)
    })
    .await;
}
