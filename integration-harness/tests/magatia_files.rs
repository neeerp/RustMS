//! Romeo and Juliet's party quest, its fourth stage: in the hub of files a
//! Card Key opens a door into a room beside it for a moment, and with it
//! that room's way back; a box in the room holds one of the two files.
//! Romeo, shown the side's letter, is shocked, which every member hears;
//! handed both files by the leader, he clears the stage, sends the hub's
//! monsters away and opens its way on.

use game_data::server_content::keys;
use integration_harness::events::{
    armed_in, armed_with, cleared, maps, party, talk, throw_down, told,
};
use integration_harness::hunting::{hit_reactor, pickup, walk_to, MapReactor};
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, until_within, warp, Trace};
use integration_harness::WorldSession;
use net::events::{EventKind, MagatiaSide, ZENUMIST_LOBBY};
use net::npc_scripts::{ROMEO, ROMEO_HUB};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const CARD_KEY: i32 = 4001133;
const LETTER: i32 = 4001131;
const FILES: [i32; 2] = [4001134, 4001135];
/// The hub's west door (`rnj3_out1`), the west room's way back
/// (`rnj31_out`), and the hub's way on (`rnj3_out3`).
const WEST_DOOR: i32 = 2618002;
const WEST_EXIT: i32 = 2618003;
const WAY_ON: i32 = 2618007;
/// The west room's box that holds the first file.
const FILE_BOX: i32 = 2612002;
/// The hub's mobs.
const MOB: i32 = 9300148;
const PINK: u8 = 5;

fn text(key: &str) -> String {
    net::content::text().get(key, &[])
}

/// Four at level 75 in Romeo's room, the leader holding a Card Key, the
/// letter and the second file, partied up and taken in.
async fn four_in() -> Vec<WorldSession> {
    let event = EventKind::Magatia(MagatiaSide::Zenumist).event();
    let items = [(CARD_KEY, 1), (LETTER, 1), (FILES[1], 1)];
    let mut team = vec![armed_with(ZENUMIST_LOBBY, 75, &items).await];
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

/// The way is shut, in pink.
fn shut(trace: &Trace) -> bool {
    trace.warps.is_empty()
        && trace
            .broadcasts
            .contains(&(PINK, text(keys::MAGATIA_PORTAL_SHUT)))
}

/// Go to `map` and wait to see all of `reactors` there; what was seen.
async fn arrive(player: &mut WorldSession, map: i32, reactors: &[i32]) -> Trace {
    let mut seen = Trace::default();
    let c = &mut player.connection;
    assert_eq!(warp(c, map, "", &mut seen).await, map);
    until(c, &mut seen, |t| {
        (reactors.iter()).all(|&id| t.field.reactors.iter().any(|r| r.reactor_id == id))
    })
    .await;
    seen
}

fn reactor(seen: &Trace, reactor_id: i32) -> MapReactor {
    *(seen.field.reactors.iter())
        .find(|r| r.reactor_id == reactor_id)
        .unwrap()
}

#[tokio::test]
async fn a_card_key_opens_a_side_room_and_both_files_clear_the_hub() {
    let stages = MagatiaSide::Zenumist.stages();
    let (hub, west) = (stages[3], stages[3] + 1);
    let mut team = four_in().await;

    // One waits in the west room, whose way back is shut, and the leader
    // goes to the hub, whose door into it is shut too.
    let mut waiting = arrive(&mut team[1], west, &[WEST_EXIT, FILE_BOX]).await;
    let exit = reactor(&waiting, WEST_EXIT);
    assert!(shut(&take(&mut team[1], west, "out00").await));
    let mut seen = arrive(&mut team[0], hub, &[WEST_DOOR, WAY_ON]).await;
    until(&mut team[0].connection, &mut seen, |t| {
        t.field.alive(MOB).len() == 23
    })
    .await;
    let mobs = seen.field.alive(MOB);
    let door = reactor(&seen, WEST_DOOR);
    let way_on = reactor(&seen, WAY_ON);
    assert!(shut(&take(&mut team[0], hub, "in00").await));
    assert!(shut(&take(&mut team[0], hub, "pt00").await));

    // A Card Key opens the door, and the room's way back with it, both for
    // a moment: the one waiting comes out as the leader goes in.
    let c = &mut team[0].connection;
    send(c, walk_to(door.x, door.y - 10)).await;
    send(c, throw_down(1, 1)).await;
    until_within(c, &mut seen, Duration::from_secs(10), |t| {
        t.field.reactor_hits.contains(&(door.object_id, 1))
    })
    .await;
    until(&mut team[1].connection, &mut waiting, |t| {
        t.field.reactor_hits.contains(&(exit.object_id, 1))
    })
    .await;
    let out = take(&mut team[1], west, "out00").await;
    assert_eq!(out.warps, [hub], "{out:?}");
    let went = take(&mut team[0], hub, "in00").await;
    assert_eq!(went.warps, [west], "{went:?}");

    // The way back shuts again on its own, before or after the leader
    // came in.
    let mut room = went;
    let c = &mut team[0].connection;
    until(c, &mut room, |t| {
        let shown_shut =
            (t.field.reactors.iter()).any(|r| r.object_id == exit.object_id && r.state == 0);
        t.field.reactors.iter().any(|r| r.reactor_id == FILE_BOX)
            && (shown_shut || t.field.reactor_hits.contains(&(exit.object_id, 0)))
    })
    .await;
    assert!(shut(&take(&mut team[0], west, "out00").await));

    // Four hits break the room's box, which holds the first file.
    let file_box = reactor(&room, FILE_BOX);
    let c = &mut team[0].connection;
    send(c, walk_to(file_box.x, file_box.y)).await;
    for state in 1..=3u8 {
        send(c, hit_reactor(file_box.object_id)).await;
        until(c, &mut room, |t| {
            t.field.reactor_hits.contains(&(file_box.object_id, state))
        })
        .await;
    }
    send(c, hit_reactor(file_box.object_id)).await;
    until(c, &mut room, |t| t.field.item_drop(FILES[0]).is_some()).await;
    let file = room.field.item_drop(FILES[0]).unwrap();
    send(c, pickup(file.object_id, file.x, file.y)).await;
    until(c, &mut room, |t| {
        t.added.iter().any(|&(_, _, item)| item == FILES[0])
    })
    .await;

    // Back in the hub, Romeo reads the letter, and everyone hears of it.
    let mut seen = Trace::default();
    let c = &mut team[0].connection;
    assert_eq!(warp(c, hub, "", &mut seen).await, hub);
    let read = told(&mut team[0], talk(hub, ROMEO_HUB)).await;
    assert_eq!(read.prompt_texts, [text(keys::MAGATIA_LETTER)], "{read:?}");
    send(&mut team[0].connection, answer(0, 1, None)).await;
    let shocked = (text(keys::MAGATIA_ROMEO_SHOCKED), 0);
    for player in team[2..].iter_mut() {
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, |t| {
            t.item_lines.contains(&shocked)
        })
        .await;
    }

    // Handed both files, he clears the stage, for no EXP, sends the hub's
    // monsters away and opens its way on.
    let taken = told(&mut team[0], talk(hub, ROMEO_HUB)).await;
    assert_eq!(taken.prompt_texts, [text(keys::MAGATIA_FILES)], "{taken:?}");
    send(&mut team[0].connection, answer(0, 1, None)).await;
    let mut seen = taken;
    until(&mut team[0].connection, &mut seen, |t| {
        cleared(t)
            && mobs.iter().all(|oid| t.field.killed.contains(oid))
            && t.field.reactor_hits.contains(&(way_on.object_id, 1))
    })
    .await;
    let etc_taken = (seen.removed.iter()).filter(|&&(bag, _)| bag == 4).count();
    assert_eq!(etc_taken, FILES.len(), "{:?}", seen.removed);
    let went = take(&mut team[0], hub, "pt00").await;
    assert_eq!(went.warps, [stages[4]], "{went:?}");
}
