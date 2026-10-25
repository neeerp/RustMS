//! Find the Jewel (report b172): a GM's `/jewel` opens it, Paul takes a
//! player to its lobby, and its gate (`join00`) stays closed until the
//! GM's `/start`. Then in the first hall a box brings out a monster and
//! another sends the player to one of the rooms its `info` names, and when
//! the time is up they go out to 109050001. The harness runs events 60
//! times as fast (`RUSTMS_EVENT_SCALE`), so its 600 s are 10.

use game_data::GameData;
use integration_harness::connection::MapleTestConnection;
use integration_harness::database::set_gm_level;
use integration_harness::hunting::{hit_reactor, walk_to, MapReactor};
use integration_harness::packets::{build_all_chat, build_change_map};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{barrier, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use std::path::Path;
use tokio::time::Duration;

const SOUTHPERRY: i32 = 60000;
const PAUL: i32 = 9000000;
const LOBBY: i32 = 109_010_000;
const HALL_A: i32 = 109_010_100;
const EXIT: i32 = 109_050_001;
/// Where the gate leads: the lobby's `up00`.
const UP: u8 = 6;
const MOB_BOX: i32 = 9001000;
const MOB: i32 = 9100001;
const ROOM_BOX: i32 = 9000000;
const CLOSED: (i16, u8) = (SendOpcode::BlockedMap as i16, 1);

/// Talking to `npc` on `map`.
fn talk_to(map: i32, npc: i32) -> Packet {
    let maps = GameData::load_from_nx_map(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx"),
    )
    .unwrap();
    let index = maps
        .field(map)
        .unwrap()
        .map_npcs
        .iter()
        .position(|n| n.npc_id == npc)
        .unwrap();
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(1_000_000_000 + index as i32).unwrap();
    p
}

/// Through the gate, from the lobby.
async fn gate(c: &mut MapleTestConnection, m: &str) -> Trace {
    let mut t = Trace::default();
    send(c, build_change_map(-1, "join00").unwrap()).await;
    until(c, &mut t, |t| !t.blocked.is_empty() || !t.warps.is_empty()).await;
    barrier(c, m, &mut t).await;
    t
}

/// Break a box of `reactor_id` on the field `t` has seen, standing by it.
async fn break_box(c: &mut MapleTestConnection, t: &mut Trace, reactor_id: i32) {
    let MapReactor {
        object_id, x, y, ..
    } = *t
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == reactor_id)
        .unwrap();
    send(c, walk_to(x, y)).await;
    for state in 1..=3u8 {
        send(c, hit_reactor(object_id)).await;
        until(c, t, |t| t.field.reactor_hits.contains(&(object_id, state))).await;
    }
    send(c, hit_reactor(object_id)).await;
    until(c, t, |t| t.field.reactors_removed.contains(&(object_id, 4))).await;
}

#[tokio::test]
async fn the_halls_boxes_bring_out_monsters_and_lead_to_rooms_until_the_time_is_up() {
    let config = load_harness_config_or_fail().await;
    login_to_world_session(&config).await.expect("GM login");
    let player = load_harness_config_or_fail().await;
    login_to_world_session(&player).await.expect("player login");
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_gm_level(&config.username, 1).await.expect("make a GM");

    let mut gm = relogin(&config).await;
    let g = &mut gm.connection;
    let mut gt = Trace::default();
    send(g, build_all_chat("/jewel", 0).unwrap()).await;
    barrier(g, "opened", &mut gt).await;

    let mut session = relogin(&player).await;
    let (c, m) = (&mut session.connection, player.character_name.as_str());
    let mut t = Trace::default();
    assert_eq!(warp(c, SOUTHPERRY, "", &mut t).await, SOUTHPERRY);
    barrier(c, m, &mut t).await;
    step(c, m, talk_to(SOUTHPERRY, PAUL)).await;
    let t = step(c, m, answer(4, 1, Some(1))).await;
    assert_eq!(t.warps, [LOBBY], "in");
    let t = gate(c, m).await;
    assert_eq!(t.blocked, [CLOSED], "closed");

    assert_eq!(warp(g, LOBBY, "", &mut gt).await, LOBBY);
    barrier(g, "in the lobby", &mut gt).await;
    send(g, build_all_chat("/start", 0).unwrap()).await;
    let mut t = Trace::default();
    until(c, &mut t, |t| !t.countdowns.is_empty()).await;
    assert_eq!(t.countdowns, [10], "the clock");
    let t = gate(c, m).await;
    assert!(t.blocked.is_empty(), "open");
    assert_eq!(t.arrivals.first().map(|a| (a.0, a.1)), Some((LOBBY, UP)));

    let mut t = Trace::default();
    assert_eq!(warp(c, -1, "in00", &mut t).await, HALL_A);
    until(c, &mut t, |t| {
        [MOB_BOX, ROOM_BOX]
            .iter()
            .all(|id| t.field.reactors.iter().any(|r| r.reactor_id == *id))
    })
    .await;
    break_box(c, &mut t, MOB_BOX).await;
    until(c, &mut t, |t| {
        t.field.mobs.iter().any(|&(_, mob)| mob == MOB)
    })
    .await;
    break_box(c, &mut t, ROOM_BOX).await;
    until(c, &mut t, |t| t.warps.len() > 1).await;
    assert!(
        (109_010_104..=109_010_108).contains(&t.warps[1]),
        "{:?}",
        t.warps
    );

    // The time is up, and the player goes out.
    let mut t = Trace::default();
    until(c, &mut t, |t| !t.warps.is_empty()).await;
    assert_eq!(t.warps, [EXIT], "out");

    send(g, build_all_chat("/endevent", 0).unwrap()).await;
    barrier(g, "closed", &mut gt).await;
}
