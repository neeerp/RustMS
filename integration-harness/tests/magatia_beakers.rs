//! Romeo and Juliet's party quest, its third stage: the beaker room's way
//! on stays shut until each of its three beakers has taken six Magic
//! Stones; the third to fill clears the stage, for 20000 EXP each, kills
//! the room's mobs, and opens its door.

use integration_harness::events::{armed_in, armed_with};
use integration_harness::events::{cleared, maps, party, talk, throw_down, told};
use integration_harness::hunting::{walk_to, MapReactor};
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, until_within, warp, Trace};
use integration_harness::WorldSession;
use net::events::{EventKind, MagatiaSide, ZENUMIST_LOBBY};
use net::npc_scripts::ROMEO;
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::cell::Cell;
use tokio::time::Duration;

const MAGIC_STONE: i32 = 4001132;
const BEAKER: i32 = 2618000;
const DOOR: i32 = 2618006;
/// The room's mobs.
const MOB: i32 = 9300147;
const PINK: u8 = 5;

/// Four at level 75 in Romeo's room, the leader holding 18 Magic Stones,
/// partied up and taken in.
async fn four_in() -> Vec<WorldSession> {
    let event = EventKind::Magatia(MagatiaSide::Zenumist).event();
    let mut team = vec![armed_with(ZENUMIST_LOBBY, 75, &[(MAGIC_STONE, 18)]).await];
    for _ in 0..3 {
        team.push(armed_in(ZENUMIST_LOBBY, 75).await);
    }
    party(&mut team).await;
    let mut seen = None;
    for _ in 0..60 {
        told(&mut team[0], talk(ZENUMIST_LOBBY, ROMEO)).await;
        let trace = told(&mut team[0], answer(4, 1, Some(0))).await;
        if !trace.prompts.iter().any(|&(from, _)| from == ROMEO) {
            seen = Some(trace);
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    assert!(seen.is_some(), "Romeo never let the party in");
    for player in team.iter_mut() {
        let mut trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&event.entry)
        })
        .await;
    }
    team
}

/// CHANGE_MAP_SPECIAL: walk to `map`'s portal `name` and press Up; what
/// came of it.
async fn take(player: &mut WorldSession, map: i32, name: &str) -> Trace {
    let portal = maps().field(map).unwrap().portal_by_name(name).unwrap();
    send(
        &mut player.connection,
        walk_to(portal.x as i16, portal.y as i16),
    )
    .await;
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(name).unwrap();
    p.write_short(0).unwrap();
    let mut trace = Trace::default();
    let c = &mut player.connection;
    send(c, p).await;
    until(c, &mut trace, |t| {
        !t.warps.is_empty() || !t.broadcasts.is_empty()
    })
    .await;
    read_for(c, &mut trace, 300).await;
    trace
}

#[tokio::test]
async fn three_beakers_of_six_magic_stones_clear_the_room_and_open_its_door() {
    let stages = MagatiaSide::Zenumist.stages();
    let mut team = four_in().await;

    // The leader goes on to the party's beaker room, whose way on is shut.
    let room = stages[2];
    let mut seen = Trace::default();
    let c = &mut team[0].connection;
    assert_eq!(warp(c, room, "", &mut seen).await, room);
    until(c, &mut seen, |t| {
        (t.field.reactors.iter())
            .filter(|r| r.reactor_id == BEAKER)
            .count()
            == 3
            && t.field.reactors.iter().any(|r| r.reactor_id == DOOR)
            && !t.field.alive(MOB).is_empty()
    })
    .await;
    let shut = take(&mut team[0], room, "pt00").await;
    let line = net::content::text().get(game_data::server_content::keys::MAGATIA_PORTAL_SHUT, &[]);
    assert!(
        shut.warps.is_empty() && shut.broadcasts.contains(&(PINK, line)),
        "{shut:?}"
    );

    // A beaker takes a stone every five seconds, and the run lasts 45
    // here, so six stones go on each of the three at once.
    let beakers: Vec<_> = (seen.field.reactors.iter())
        .filter(|r| r.reactor_id == BEAKER)
        .copied()
        .collect();
    let door = *(seen.field.reactors.iter())
        .find(|r| r.reactor_id == DOOR)
        .unwrap();
    let mobs = seen.field.alive(MOB);
    let c = &mut team[0].connection;
    for beaker in &beakers {
        send(c, walk_to(beaker.x, beaker.y - 10)).await;
        for _ in 0..6 {
            send(c, throw_down(1, 1)).await;
        }
    }

    // Each fills a state at a time; only the third to fill clears the
    // stage, which kills the mobs and opens the door.
    let full = |t: &Trace, b: &MapReactor| t.field.reactor_hits.contains(&(b.object_id, 6));
    let early = Cell::new(false);
    until_within(c, &mut seen, Duration::from_secs(35), |t| {
        let all = beakers.iter().all(|b| full(t, b));
        early.set(early.get() || (cleared(t) && !all));
        all && cleared(t)
            && t.gains.contains(&20_000)
            && t.field.reactor_hits.contains(&(door.object_id, 1))
            && mobs.iter().all(|oid| t.field.killed.contains(oid))
    })
    .await;
    assert!(!early.get(), "cleared before the third beaker filled");
    for beaker in &beakers {
        let states: Vec<u8> = (seen.field.reactor_hits.iter())
            .filter(|&&(oid, _)| oid == beaker.object_id)
            .map(|&(_, state)| state)
            .take(6)
            .collect();
        assert_eq!(states, [1, 2, 3, 4, 5, 6], "{seen:?}");
    }
    for player in team[1..].iter_mut() {
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, |t| {
            t.gains.contains(&20_000)
        })
        .await;
    }
    let went = take(&mut team[0], room, "pt00").await;
    assert_eq!(went.warps, [stages[3]], "{went:?}");
}
