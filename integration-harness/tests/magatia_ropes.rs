//! Romeo and Juliet's party quest, its fifth and sixth stages: Yulete waits
//! in his office, and what he says depends on whether Romeo read the
//! letter and on who stands near him; ten seconds after the party first
//! comes in he leaves, saying so to everyone, and sets his guards on them.
//! Their last kill clears the stage and opens the office's way on. On the
//! landing beyond, each of the four ways leads one member into a rope room
//! of their own; in it the right rope of each row leads a row up and any
//! other back down, and the first to reach the top clears the stage.
//! The harness pins the right ropes (`RUSTMS_MAGATIA_ROPES`).

use game_data::server_content::keys;
use integration_harness::events::told;
use integration_harness::events::{armed_in, armed_with, cleared, maps, party, talk, talk_to};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, until_within, warp, Trace};
use integration_harness::WorldSession;
use net::events::{EventKind, MagatiaSide};
use net::npc_scripts::{JULIET, ROMEO, ROMEO_HUB, YULETE};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const LETTER: i32 = 4001131;
/// Yulete's guards.
const GUARDS: [i32; 2] = [9300143, 9300144];
/// The office's way on (`rnj6_out`).
const OFFICE_DOOR: i32 = 2618005;
/// The pinned right rope of each row, from the bottom.
const ROPES: [u8; 10] = [0, 1, 2, 3, 0, 1, 2, 3, 0, 1];
const PINK: u8 = 5;

fn text(key: &str) -> String {
    net::content::text().get(key, &[])
}

/// Four at level 75 in `side`'s lobby, the leader holding `items`, partied
/// up and taken in by `host`.
async fn four_in(side: MagatiaSide, host: i32, items: &[(i32, i16)]) -> Vec<WorldSession> {
    let event = EventKind::Magatia(side).event();
    let mut team = vec![armed_with(event.lobby, 75, items).await];
    for _ in 0..3 {
        team.push(armed_in(event.lobby, 75).await);
    }
    party(&mut team).await;
    let mut seen = None;
    for _ in 0..60 {
        told(&mut team[0], talk(event.lobby, host)).await;
        let trace = told(&mut team[0], answer(4, 1, Some(0))).await;
        if !trace.prompts.iter().any(|&(from, _)| from == host) {
            seen = Some(trace);
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    assert!(seen.is_some(), "{host} never let the party in");
    for player in team.iter_mut() {
        let mut trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&event.entry)
        })
        .await;
    }
    team
}

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `name`.
fn enter(name: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(name).unwrap();
    p.write_short(0).unwrap();
    p
}

/// Walk to `map`'s portal `name` and press Up; what came of it.
async fn take(player: &mut WorldSession, map: i32, name: &str) -> Trace {
    let portal = maps().field(map).unwrap().portal_by_name(name).unwrap();
    send(
        &mut player.connection,
        walk_to(portal.x as i16, portal.y as i16),
    )
    .await;
    let mut trace = Trace::default();
    let c = &mut player.connection;
    send(c, enter(name)).await;
    until(c, &mut trace, |t| {
        !t.warps.is_empty() || !t.broadcasts.is_empty() || !t.arrivals.is_empty()
    })
    .await;
    read_for(c, &mut trace, 300).await;
    trace
}

/// The portal of the rope room `room` that the rope `name` led to, after
/// its sound.
async fn climb(player: &mut WorldSession, room: i32, name: &str) -> u8 {
    let trace = take(player, room, name).await;
    assert_eq!(trace.portal_sounds, 1, "{name}: {trace:?}");
    let [(to, portal, _)] = trace.arrivals[..] else {
        panic!("{name}: {:?}", trace.arrivals);
    };
    assert_eq!(to, room, "{name}");
    portal
}

#[tokio::test]
async fn yulete_caught_out_leaves_his_guards_whose_last_kill_opens_his_office() {
    let side = MagatiaSide::Zenumist;
    let stages = side.stages();
    let mut team = four_in(side, ROMEO, &[(LETTER, 1)]).await;

    // Romeo reads the letter in the hub, so Yulete has been found out.
    let mut seen = Trace::default();
    let c = &mut team[0].connection;
    assert_eq!(warp(c, stages[3], "", &mut seen).await, stages[3]);
    let read = told(&mut team[0], talk(stages[3], ROMEO_HUB)).await;
    assert_eq!(read.prompt_texts, [text(keys::MAGATIA_LETTER)], "{read:?}");
    send(&mut team[0].connection, answer(0, 1, None)).await;

    // In his office, talked to from across the room, he is caught.
    let office = stages[4];
    let mut seen = Trace::default();
    let c = &mut team[0].connection;
    assert_eq!(warp(c, office, "", &mut seen).await, office);
    until(c, &mut seen, |t| {
        t.field.npcs.iter().any(|&(_, id)| id == YULETE)
            && t.field.reactors.iter().any(|r| r.reactor_id == OFFICE_DOOR)
    })
    .await;
    let (yulete, _) = *(seen.field.npcs.iter())
        .find(|&&(_, id)| id == YULETE)
        .unwrap();
    let door = *(seen.field.reactors.iter())
        .find(|r| r.reactor_id == OFFICE_DOOR)
        .unwrap();
    let said = told(&mut team[0], talk_to(yulete)).await;
    assert_eq!(said.prompts, [(YULETE, 0)], "{said:?}");
    assert_eq!(said.prompt_texts, [text(keys::MAGATIA_YULETE_CAUGHT)]);
    send(&mut team[0].connection, answer(0, 1, None)).await;

    // He leaves on his own, telling everyone, and five of each guard come
    // out at each of three spots.
    let left = (PINK, text(keys::MAGATIA_YULETE_DISGUSTED));
    let c = &mut team[0].connection;
    until_within(c, &mut seen, Duration::from_secs(20), |t| {
        t.broadcasts.contains(&left)
            && t.field.npcs_removed.contains(&yulete)
            && GUARDS
                .iter()
                .map(|&id| t.field.alive(id).len())
                .sum::<usize>()
                == 30
    })
    .await;
    for player in team[1..].iter_mut() {
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, |t| {
            t.broadcasts.contains(&left)
        })
        .await;
    }
    let shut = take(&mut team[0], office, "out00").await;
    let line = (PINK, text(keys::MAGATIA_PORTAL_SHUT));
    assert!(
        shut.warps.is_empty() && shut.broadcasts.contains(&line),
        "{shut:?}"
    );

    // The last kill clears the stage, for 20000 EXP each, and opens the
    // way on.
    let guards: Vec<i32> = (GUARDS.iter())
        .flat_map(|&id| seen.field.alive(id))
        .collect();
    let c = &mut team[0].connection;
    for &oid in &guards {
        send(c, attack(oid, 20_000)).await;
        send(c, attack(oid, 20_000)).await;
    }
    until(c, &mut seen, |t| {
        guards.iter().all(|oid| t.field.killed.contains(oid))
            && cleared(t)
            && t.gains.contains(&20_000)
            && t.field.reactor_hits.contains(&(door.object_id, 1))
    })
    .await;
    for player in team[1..].iter_mut() {
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, |t| {
            t.gains.contains(&20_000)
        })
        .await;
    }
    let went = take(&mut team[0], office, "out00").await;
    assert_eq!(went.warps, [stages[5]], "{went:?}");
}

#[tokio::test]
async fn each_member_takes_a_rope_room_and_the_first_to_the_top_clears_the_ropes() {
    let side = MagatiaSide::Alcadno;
    let stages = side.stages();
    let landing = stages[5];
    let mut team = four_in(side, JULIET, &[]).await;
    for player in team.iter_mut() {
        let mut seen = Trace::default();
        let c = &mut player.connection;
        assert_eq!(warp(c, landing, "", &mut seen).await, landing);
    }

    // Each way leads into its own room, and only once.
    let went = take(&mut team[0], landing, "pt00").await;
    assert_eq!(went.warps, [landing + 1], "{went:?}");
    let taken = take(&mut team[1], landing, "pt00").await;
    assert!(
        taken.warps.is_empty()
            && (taken.broadcasts).contains(&(PINK, text(keys::MAGATIA_ROOM_TAKEN))),
        "{taken:?}"
    );
    for (n, player) in team.iter_mut().enumerate().skip(1) {
        let went = take(player, landing, &format!("pt0{n}")).await;
        assert_eq!(went.warps, [landing + 1 + n as i32], "{went:?}");
    }

    // A wrong rope leads back down; the right one a row up, and from the
    // top row to the top.
    let room = landing + 1;
    let p = &mut team[0];
    assert_eq!(climb(p, room, "pt01").await, 2);
    for (row, &rope) in ROPES.iter().enumerate().take(ROPES.len() - 1) {
        let up = climb(p, room, &format!("pt{row}{rope}")).await;
        assert_eq!(up, 15 + 4 * (row as u8 + 1) + rope, "row {row}");
    }
    let last = ROPES.len() - 1;
    let mut seen = Trace::default();
    let name = format!("pt{last}{}", ROPES[last]);
    let portal = maps().field(room).unwrap().portal_by_name(&name).unwrap();
    let c = &mut team[0].connection;
    send(c, walk_to(portal.x as i16, portal.y as i16)).await;
    send(c, enter(&name)).await;
    until(c, &mut seen, |t| {
        t.arrivals
            .iter()
            .any(|&(to, portal, _)| (to, portal) == (room, 1))
            && t.gains.contains(&20_000)
    })
    .await;
    // The ropes' clear is quiet, and pays everyone.
    read_for(&mut team[0].connection, &mut seen, 300).await;
    assert!(!cleared(&seen), "{:?}", seen.field_effects);
    for player in team[1..].iter_mut() {
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, |t| {
            t.gains.contains(&20_000)
        })
        .await;
    }
}
