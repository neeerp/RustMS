//! Orbis's party quest, the music room and the sealed room. In the first
//! the music box plays only the day's record, after which Eak brings out
//! a stone box and clears the room; in the second he wants three of the
//! team on its platforms, so many on each, and hints how near they are.
//! The harness world pins the day to Tuesday (`RUSTMS_OPQ_DAY`) and the
//! platforms to two, one and none (`RUSTMS_OPQ_SPLIT`), and runs events
//! 60 times as fast (`RUSTMS_EVENT_SCALE`), so the quest lasts a minute.

use game_data::server_content::keys;
use integration_harness::events::{enter_opq, party, talk, throw_down, waiting_in, EAK};
use integration_harness::hunting::{hit_reactor, walk_to, MapReactor};
use integration_harness::trace::{read_for, send, step, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{OPQ, OPQ_HEAD_COUNT_ROOM, OPQ_MUSIC_ROOM};
use net::reactors::MUSIC;
use packet::Packet;

const MONDAY_RECORD: i32 = 4001057;
const TUESDAY_RECORD: i32 = 4001058;
const STONE3: i32 = 2002011;
const STONE4: i32 = 2002012;

/// The trace of `player` sending `packet`, up to the barrier.
async fn told(player: &mut WorldSession, packet: Packet) -> Trace {
    let name = player.character_name.clone();
    step(&mut player.connection, &name, packet).await
}

fn said(trace: &Trace, key: &str) -> bool {
    let text = net::content::text().get(key, &[]);
    trace.prompt_texts.iter().any(|said| said.contains(&text))
}

fn cleared(trace: &Trace) -> bool {
    trace
        .field_effects
        .iter()
        .any(|(_, e)| e == "quest/party/clear")
}

fn wrong(trace: &Trace) -> bool {
    trace
        .field_effects
        .iter()
        .any(|(_, e)| e == "quest/party/wrong_kor")
}

/// Stand on the sealed room's platform `n` (Map.nx `area` n + 1), or off
/// them all for `None`, before anything else this player does.
async fn stand(player: &mut WorldSession, n: Option<usize>) {
    let (x, y) = n.map_or((84, -80), |n| [(-160, -800), (-40, -920), (80, -800)][n]);
    told(player, walk_to(x, y)).await;
}

/// Go to `map` and wait to see its reactor `reactor_id`.
async fn go_to(player: &mut WorldSession, map: i32, reactor_id: i32) -> (Trace, MapReactor) {
    let mut seen = Trace::default();
    assert_eq!(warp(&mut player.connection, map, "", &mut seen).await, map);
    until(&mut player.connection, &mut seen, |t| {
        t.field.reactors.iter().any(|r| r.reactor_id == reactor_id)
    })
    .await;
    let reactor = *seen
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == reactor_id)
        .unwrap();
    (seen, reactor)
}

/// Break open the stone box `stone`, out after Eak's fade-in, for `piece`:
/// it takes four hits.
async fn break_open(player: &mut WorldSession, seen: &mut Trace, stone: MapReactor, piece: i32) {
    let c = &mut player.connection;
    send(c, walk_to(stone.x, stone.y)).await;
    for state in 2..=4u8 {
        send(c, hit_reactor(stone.object_id)).await;
        until(c, seen, |t| {
            t.field.reactor_hits.contains(&(stone.object_id, state))
        })
        .await;
    }
    send(c, hit_reactor(stone.object_id)).await;
    until(c, seen, |t| {
        t.field
            .reactors_removed
            .iter()
            .any(|&(oid, _)| oid == stone.object_id)
            && t.field.item_drop(piece).is_some()
    })
    .await;
}

#[tokio::test]
async fn the_days_record_and_the_right_heads_bring_out_the_stone_boxes() {
    let records = [(MONDAY_RECORD, 1), (TUESDAY_RECORD, 1)];
    let mut a = vec![waiting_in(OPQ.lobby, 51, &records).await];
    for _ in 0..4 {
        a.push(waiting_in(OPQ.lobby, 51, &[]).await);
    }
    party(&mut a).await;
    enter_opq(&mut a).await;

    // The music room: any member hears what Eak wants.
    let (mut seen, music) = go_to(&mut a[0], OPQ_MUSIC_ROOM, MUSIC).await;
    until(&mut a[0].connection, &mut seen, |t| {
        t.field.reactors.iter().any(|r| r.reactor_id == STONE3)
    })
    .await;
    let stone3 = *seen
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == STONE3)
        .unwrap();
    assert_eq!(stone3.state, 0);
    go_to(&mut a[1], OPQ_MUSIC_ROOM, MUSIC).await;
    let trace = told(&mut a[1], talk(OPQ_MUSIC_ROOM, EAK)).await;
    assert!(said(&trace, keys::OPQ_EAK_MUSIC_ASK), "{trace:?}");

    // Monday's record stays where it lands; Tuesday's plays.
    let c = &mut a[0].connection;
    send(c, walk_to(music.x, music.y - 10)).await;
    send(c, throw_down(1, 1)).await;
    until(c, &mut seen, |t| t.field.item_drop(MONDAY_RECORD).is_some()).await;
    read_for(c, &mut seen, 600).await;
    let monday = seen.field.item_drop(MONDAY_RECORD).unwrap();
    assert!(seen.field.removals(monday.object_id).is_empty(), "{seen:?}");
    assert!(seen.field.reactor_hits.is_empty(), "{seen:?}");
    send(c, throw_down(2, 1)).await;
    until(c, &mut seen, |t| {
        t.field.reactor_hits.contains(&(music.object_id, 3))
    })
    .await;

    // Now Eak brings the stone box out and clears the room, once.
    let mut trace = told(&mut a[1], talk(OPQ_MUSIC_ROOM, EAK)).await;
    assert!(said(&trace, keys::OPQ_EAK_MUSIC_PLAYED), "{trace:?}");
    until(&mut a[1].connection, &mut trace, cleared).await;
    until(&mut a[0].connection, &mut seen, |t| {
        t.field.reactor_hits.contains(&(stone3.object_id, 1)) && cleared(t)
    })
    .await;
    let trace = told(&mut a[1], talk(OPQ_MUSIC_ROOM, EAK)).await;
    assert!(said(&trace, keys::OPQ_EAK_MUSIC_DONE), "{trace:?}");
    assert!(!cleared(&trace), "{trace:?}");
    break_open(&mut a[0], &mut seen, stone3, 4001046).await;

    // The sealed room: Eak wants two on the first platform, one on the
    // second and none on the third (Map.nx `area` 1-3).
    let mut traces = Vec::new();
    for player in a[..4].iter_mut() {
        traces.push(go_to(player, OPQ_HEAD_COUNT_ROOM, STONE4).await);
    }
    let stone4 = traces[0].1;
    // None, one and two: only the second platform is right.
    for (player, on) in [(0, 2), (1, 2), (2, 1)] {
        stand(&mut a[player], Some(on)).await;
    }
    let trace = told(&mut a[3], talk(OPQ_HEAD_COUNT_ROOM, EAK)).await;
    assert!(said(&trace, keys::OPQ_EAK_HEADS_ONE_RIGHT), "{trace:?}");
    assert!(wrong(&trace), "{trace:?}");
    // One, none and two: none is.
    stand(&mut a[0], Some(0)).await;
    stand(&mut a[2], Some(2)).await;
    let trace = told(&mut a[3], talk(OPQ_HEAD_COUNT_ROOM, EAK)).await;
    assert!(said(&trace, keys::OPQ_EAK_HEADS_ALL_WRONG), "{trace:?}");
    assert!(wrong(&trace), "{trace:?}");
    // Four on them is not three: he only says so.
    stand(&mut a[2], Some(1)).await;
    stand(&mut a[3], Some(1)).await;
    let trace = told(&mut a[3], talk(OPQ_HEAD_COUNT_ROOM, EAK)).await;
    assert!(said(&trace, keys::OPQ_EAK_HEADS_FEW), "{trace:?}");
    assert!(!wrong(&trace), "{trace:?}");
    // Two, one and none, with the fourth off them.
    stand(&mut a[3], None).await;
    stand(&mut a[1], Some(0)).await;
    let mut trace = told(&mut a[3], talk(OPQ_HEAD_COUNT_ROOM, EAK)).await;
    assert!(said(&trace, keys::OPQ_EAK_HEADS_RIGHT), "{trace:?}");
    until(&mut a[3].connection, &mut trace, cleared).await;
    let (seen, _) = &mut traces[0];
    until(&mut a[0].connection, seen, |t| {
        t.field.reactor_hits.contains(&(stone4.object_id, 1)) && cleared(t)
    })
    .await;
    let trace = told(&mut a[3], talk(OPQ_HEAD_COUNT_ROOM, EAK)).await;
    assert!(said(&trace, keys::OPQ_EAK_HEADS_DONE), "{trace:?}");
    assert!(!cleared(&trace), "{trace:?}");
    break_open(&mut a[0], seen, stone4, 4001047).await;
}
