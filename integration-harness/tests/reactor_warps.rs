//! Reactors that send whoever breaks them somewhere, and the fourth job's
//! rooms they stand in: the Hitman's hideout, whose boxes lead up its
//! floors or hide him, and the Berserk room, whose way up to the statues
//! waits for its monsters and whose false statues mostly throw the player
//! out. A warp within either room keeps it as it is.

use integration_harness::database::{grant_item, set_dex, set_quest, set_str, set_weapon_attack};
use integration_harness::hunting::{attack, hit_reactor, walk_to, MapReactor};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::collections::BTreeSet;
use tokio::time::Duration;

const ETC: i16 = 4;
const KERNING_HIDEOUT_WAY: i32 = 101030104;
const HITMAN_HIDEOUT: i32 = 910200000;
const LEADS_UP: i32 = 1020000;
const HIDES_HIM: i32 = 1021000;
const HITMAN: i32 = 9300091;
const SLEEPY_DUNGEON: i32 = 105090800;
const SLEEPY_DUNGEON_BELOW: i32 = 105090200;
const BERSERK_ROOM: i32 = 910500200;
const BERSERK_KEY: i32 = 4031475;
const GUARD: i32 = 9300092;
const FALSE_STATUE: i32 = 1050000;
const TRUE_STATUE: i32 = 1052000;
const STATUES_X: [i16; 5] = [-957, -598, -164, 205, 532];

fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// Go to `map` and take its way in `portal` to `room`, arriving at
/// `arrival`.
async fn go_in(
    player: &mut WorldSession,
    (map, portal): (i32, &str),
    room: i32,
    arrival: u8,
) -> Trace {
    let mut trace = Trace::default();
    assert_eq!(warp(&mut player.connection, map, "", &mut trace).await, map);
    let name = player.character_name.clone();
    let mut trace = step(&mut player.connection, &name, enter(portal)).await;
    assert_eq!(trace.arrivals.len(), 1, "{:?}", trace.arrivals);
    assert_eq!(trace.arrivals[0].0, room);
    assert_eq!(trace.arrivals[0].1, arrival);
    trace.arrivals.clear();
    trace
}

fn reactor(trace: &Trace, reactor_id: i32, x: i16) -> MapReactor {
    *(trace.field.reactors.iter())
        .find(|r| r.reactor_id == reactor_id && r.x == x)
        .unwrap_or_else(|| panic!("{reactor_id} at {x}: {:?}", trace.field.reactors))
}

/// Walk up to `reactor` and hit it until it breaks.
async fn break_open(player: &mut WorldSession, trace: &mut Trace, reactor: MapReactor) {
    let c = &mut player.connection;
    send(c, walk_to(reactor.x, reactor.y)).await;
    read_for(c, trace, 300).await;
    for _ in 0..4 {
        send(c, hit_reactor(reactor.object_id)).await;
        read_for(c, trace, 150).await;
    }
    until(c, trace, |t| {
        t.field
            .reactors_removed
            .iter()
            .any(|&(oid, _)| oid == reactor.object_id)
    })
    .await;
}

#[tokio::test]
async fn the_hitmans_boxes_lead_up_his_hideout_or_hide_him() {
    let config = load_harness_config_or_fail().await;
    let player = login_to_world_session(&config).await.unwrap();
    let id = player.character_id;
    drop(player);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_quest(id, 6201, 1, "").await.unwrap();
    let mut player = relogin(&config).await;

    let mut trace = go_in(
        &mut player,
        (KERNING_HIDEOUT_WAY, "pt00"),
        HITMAN_HIDEOUT,
        0,
    )
    .await;
    let c = &mut player.connection;
    until(c, &mut trace, |t| t.field.reactors.len() == 12).await;
    let seen: BTreeSet<i32> = trace.field.reactors.iter().map(|r| r.object_id).collect();

    // The box by the way in leads up to the first floor, pt00.
    let up = reactor(&trace, LEADS_UP, 311);
    break_open(&mut player, &mut trace, up).await;
    let c = &mut player.connection;
    until(c, &mut trace, |t| !t.arrivals.is_empty()).await;
    assert_eq!(trace.arrivals[0].0, HITMAN_HIDEOUT);
    assert_eq!(trace.arrivals[0].1, 1);
    // The room stays as it was: no reactor is put back or made anew.
    read_for(c, &mut trace, 600).await;
    assert!(
        trace
            .field
            .reactors
            .iter()
            .all(|r| seen.contains(&r.object_id)),
        "{:?}",
        trace.field.reactors
    );

    // A box he hides in brings him out.
    let hitmen: BTreeSet<i32> = trace.field.alive(HITMAN).into_iter().collect();
    let hiding = reactor(&trace, HIDES_HIM, 7);
    break_open(&mut player, &mut trace, hiding).await;
    let c = &mut player.connection;
    until(c, &mut trace, |t| {
        t.field
            .alive(HITMAN)
            .iter()
            .any(|oid| !hitmen.contains(oid))
    })
    .await;
}

#[tokio::test]
async fn the_berserk_rooms_statues_wait_for_its_monsters() {
    let config = load_harness_config_or_fail().await;
    let player = login_to_world_session(&config).await.unwrap();
    let id = player.character_id;
    drop(player);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_quest(id, 6153, 1, "").await.unwrap();
    grant_item(id, ETC, 1, BERSERK_KEY).await.unwrap();
    set_str(id, 30000).await.unwrap();
    set_dex(id, 30000).await.unwrap();
    set_weapon_attack(id, 1999).await.unwrap();
    let mut player = relogin(&config).await;

    // In at the bottom, by out01.
    let mut trace = go_in(&mut player, (SLEEPY_DUNGEON, "in00"), BERSERK_ROOM, 2).await;
    let c = &mut player.connection;
    until(c, &mut trace, |t| {
        t.field.reactors.len() == 5 && t.field.alive(GUARD).len() == 7
    })
    .await;
    // The statues stand where the room's statues stand, in some order.
    let mut xs: Vec<i16> = trace.field.reactors.iter().map(|r| r.x).collect();
    xs.sort();
    assert_eq!(xs, STATUES_X);
    let seen: BTreeSet<i32> = trace.field.reactors.iter().map(|r| r.object_id).collect();

    // The way up stays shut while a guard stands.
    let name = player.character_name.clone();
    let shut = step(&mut player.connection, &name, enter("in00")).await;
    assert!(shut.arrivals.is_empty(), "{:?}", shut.arrivals);
    assert_eq!(shut.broadcasts.len(), 1, "{:?}", shut.broadcasts);

    let c = &mut player.connection;
    for guard in trace.field.alive(GUARD) {
        send(c, attack(guard, 50000)).await;
    }
    until(c, &mut trace, |t| t.field.alive(GUARD).is_empty()).await;
    let mut up = step(&mut player.connection, &name, enter("in00")).await;
    assert_eq!(up.arrivals.len(), 1, "{:?}", up.arrivals);
    assert_eq!(up.arrivals[0].0, BERSERK_ROOM);
    assert_eq!(up.arrivals[0].1, 3);
    let c = &mut player.connection;
    read_for(c, &mut up, 600).await;
    assert_eq!(up.field.reactors.len(), 5, "{:?}", up.field.reactors);
    assert!(
        up.field
            .reactors
            .iter()
            .all(|r| seen.contains(&r.object_id)),
        "{:?}",
        up.field.reactors
    );

    // The true statue gives its piece.
    let true_one = *trace
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == TRUE_STATUE)
        .unwrap();
    break_open(&mut player, &mut trace, true_one).await;
    let c = &mut player.connection;
    until(c, &mut trace, |t| t.field.item_drop(4031471).is_some()).await;
    assert!(trace.arrivals.is_empty(), "{:?}", trace.arrivals);

    // A false one throws the player out below or gives nothing.
    let false_one = *trace
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == FALSE_STATUE)
        .unwrap();
    break_open(&mut player, &mut trace, false_one).await;
    let c = &mut player.connection;
    read_for(c, &mut trace, 1000).await;
    match trace.arrivals[..] {
        [] => {}
        [(map, 0, _)] => assert_eq!(map, SLEEPY_DUNGEON_BELOW),
        _ => panic!("{:?}", trace.arrivals),
    }
}
