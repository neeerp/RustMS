//! Ludibrium's party quest, stage 2's boxes: each drops a pass, but one is
//! a trap, which sends everyone on the map into the room below with a
//! pink line, in the party's own copy.

use game_data::server_content::keys;
use integration_harness::events::{enter_lpq, party, waiting_in};
use integration_harness::hunting::{hit_reactor, walk_to, MapReactor};
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{LPQ, LPQ_PASS};

const BOX: i32 = 2202003;
const TRAP: i32 = 2200002;
const TRAP_ROOM: i32 = 922_010_201;

fn reactor(trace: &Trace, reactor_id: i32) -> MapReactor {
    *(trace.field.reactors.iter())
        .find(|r| r.reactor_id == reactor_id)
        .unwrap_or_else(|| panic!("{reactor_id}: {:?}", trace.field.reactors))
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
async fn a_box_drops_a_pass_and_the_trap_takes_everyone_down() {
    let mut a = Vec::new();
    for _ in 0..5 {
        a.push(waiting_in(LPQ.lobby, 35, &[]).await);
    }
    party(&mut a).await;
    enter_lpq(&mut a).await;

    // Past stage 1 by debug warp, which no gate checks.
    let map = LPQ.stage_map(2);
    let mut traces = Vec::new();
    for player in a.iter_mut() {
        let mut trace = Trace::default();
        assert_eq!(warp(&mut player.connection, map, "", &mut trace).await, map);
        until(&mut player.connection, &mut trace, has(TRAP)).await;
        traces.push(trace);
    }

    // A box drops a pass.
    let mut trace = std::mem::take(&mut traces[0]);
    let found = reactor(&trace, BOX);
    break_open(&mut a[0], &mut trace, found).await;
    until(&mut a[0].connection, &mut trace, |t| {
        t.field.drops.iter().any(|d| !d.meso && d.value == LPQ_PASS)
    })
    .await;

    // The trap takes everyone on the map down to its room, still in the
    // party's copy, with the line first.
    let trap = reactor(&trace, TRAP);
    break_open(&mut a[1], &mut traces[1], trap).await;
    let notice = net::content::text().get(keys::LPQ_NOTICE_TRAP, &[]);
    for (player, trace) in a.iter_mut().zip(traces) {
        let mut trace = trace;
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&TRAP_ROOM)
        })
        .await;
        assert!(trace.broadcasts.contains(&(5, notice.clone())), "{trace:?}");
    }

    // The room's way out leads back up to stage 2, in the same copy: the
    // trap is gone from it, and the broken box too.
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut a[0].connection, -1, "out00", &mut trace).await,
        map
    );
    until(&mut a[0].connection, &mut trace, has(BOX)).await;
    read_for(&mut a[0].connection, &mut trace, 300).await;
    let ids: Vec<_> = trace.field.reactors.iter().map(|r| r.object_id).collect();
    assert!(
        !trace.field.reactors.iter().any(|r| r.reactor_id == TRAP),
        "{ids:?}"
    );
    assert!(!ids.contains(&found.object_id), "{ids:?}");
}

/// Whether the field record shows a reactor `reactor_id`.
fn has(reactor_id: i32) -> impl Fn(&Trace) -> bool {
    move |t| t.field.reactors.iter().any(|r| r.reactor_id == reactor_id)
}
