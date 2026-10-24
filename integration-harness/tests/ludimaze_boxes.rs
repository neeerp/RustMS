//! The Ludibrium Maze's boxes: each of the maze's rooms has three, and one
//! broken brings out two lots of mobs by the box and the room (Cosmic
//! `8091000` to `8091004`, `8098000`), with a pink line to everyone there.
//! The maze lasts 15 seconds here (`RUSTMS_EVENT_SCALE`).

use game_data::server_content::keys;
use integration_harness::events::{party, talk, told, waiting_in};
use integration_harness::hunting::{hit_reactor, walk_to};
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until};
use net::events::{LUDIBRIUM, MAZE_GOAL, MAZE_ROOM};
use net::npc_scripts::ROLLY;
use tokio::time::Duration;

const PINK: u8 = 5;

#[tokio::test]
async fn a_box_broken_brings_out_its_rooms_mobs_with_a_pink_line() {
    let mut team = Vec::new();
    for _ in 0..3 {
        team.push(waiting_in(LUDIBRIUM, 51, &[]).await);
    }
    party(&mut team).await;

    // The leader takes the party in once any party before them is out.
    let in_room = |map: &i32| (MAZE_ROOM..MAZE_GOAL).contains(map);
    let mut seen = None;
    for _ in 0..20 {
        told(&mut team[0], talk(LUDIBRIUM, ROLLY)).await;
        let trace = told(&mut team[0], answer(4, 1, Some(0))).await;
        if !trace.prompts.iter().any(|&(from, _)| from == ROLLY) {
            seen = Some(trace);
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    let mut trace = seen.expect("Rolly never let the party in");
    let c = &mut team[0].connection;
    until(c, &mut trace, |t| t.warps.iter().any(in_room)).await;
    let room = *trace.warps.iter().find(|map| in_room(map)).unwrap();
    trace.field = Default::default();
    until(c, &mut trace, |t| t.field.reactors.len() == 3).await;

    // Four hits break a box: out come its two lots of mobs, and a pink line.
    let reactor = trace.field.reactors[0];
    send(c, walk_to(reactor.x, reactor.y)).await;
    read_for(c, &mut trace, 300).await;
    for _ in 0..4 {
        send(c, hit_reactor(reactor.object_id)).await;
        read_for(c, &mut trace, 150).await;
    }
    let lots = net::reactors::maze_box(reactor.reactor_id, room);
    let summoned = net::content::text().get(keys::LUDIMAZE_SUMMONED, &[]);
    until(c, &mut trace, |t| {
        t.field
            .reactors_removed
            .iter()
            .any(|&(oid, _)| oid == reactor.object_id)
            && t.broadcasts.contains(&(PINK, summoned.clone()))
            && lots.iter().all(|&(mob_id, count)| {
                t.field.mobs.iter().filter(|m| m.1 == mob_id).count() >= count
            })
    })
    .await;
    read_for(c, &mut trace, 500).await;
    assert!(
        trace.field.drops.is_empty(),
        "a maze box drops nothing: {trace:?}"
    );
}
