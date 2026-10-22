//! Ludibrium's party quest, stages 3 and 5: each of stage 3's boxes lets
//! out three Bloctopuses, and stage 5's rooms, in the party's copy, hold
//! boxes that drop passes.

use integration_harness::events::{enter_lpq, party, waiting_in};
use integration_harness::hunting::{hit_reactor, walk_to, MapReactor};
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{LPQ, LPQ_PASS};

const BLOCTOPUS_BOX: i32 = 2201001;
const BLOCTOPUS: i32 = 9300007;
const PASS_BOX: i32 = 2202003;
const ROOM: i32 = 922_010_501;

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

/// The first reactor `reactor_id` the field record shows.
fn reactor(trace: &Trace, reactor_id: i32) -> MapReactor {
    *(trace.field.reactors.iter())
        .find(|r| r.reactor_id == reactor_id)
        .unwrap_or_else(|| panic!("{reactor_id}: {:?}", trace.field.reactors))
}

fn bloctopuses(trace: &Trace) -> usize {
    let mobs = trace.field.mobs.iter();
    mobs.filter(|&&(_, mob_id)| mob_id == BLOCTOPUS).count()
}

#[tokio::test]
async fn stage_3s_boxes_let_out_bloctopuses_and_stage_5s_rooms_hold_passes() {
    let mut a = Vec::new();
    for _ in 0..5 {
        a.push(waiting_in(LPQ.lobby, 35, &[]).await);
    }
    party(&mut a).await;
    enter_lpq(&mut a).await;

    // Stage 3, by debug warp: a box lets out three Bloctopuses.
    let map = LPQ.stage_map(3);
    let mut trace = Trace::default();
    let c = &mut a[0].connection;
    assert_eq!(warp(c, map, "", &mut trace).await, map);
    until(c, &mut trace, |t| {
        t.field
            .reactors
            .iter()
            .any(|r| r.reactor_id == BLOCTOPUS_BOX)
    })
    .await;
    read_for(c, &mut trace, 300).await;
    let before = bloctopuses(&trace);
    let found = reactor(&trace, BLOCTOPUS_BOX);
    break_open(&mut a[0], &mut trace, found).await;
    until(&mut a[0].connection, &mut trace, |t| {
        bloctopuses(t) >= before + 3
    })
    .await;

    // Stage 5: a room off it, in the party's copy, has boxes that drop
    // passes, and its way out leads back.
    let map = LPQ.stage_map(5);
    let mut trace = Trace::default();
    let c = &mut a[1].connection;
    assert_eq!(warp(c, map, "", &mut trace).await, map);
    let mut trace = Trace::default();
    assert_eq!(warp(c, -1, "in01", &mut trace).await, ROOM);
    until(c, &mut trace, |t| {
        t.field.reactors.iter().any(|r| r.reactor_id == PASS_BOX)
    })
    .await;
    let found = reactor(&trace, PASS_BOX);
    break_open(&mut a[1], &mut trace, found).await;
    until(&mut a[1].connection, &mut trace, |t| {
        t.field.drops.iter().any(|d| !d.meso && d.value == LPQ_PASS)
    })
    .await;
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut a[1].connection, -1, "out00", &mut trace).await,
        map
    );
}
