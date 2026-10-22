//! Orbis's party quest, the Salion stage: each Cellion's drop lands on the
//! spot it came out at, which takes it and breaks, and the next Cellion
//! comes out at another spot (the field unit tests follow it to the spot
//! that drops the piece of the statue). The harness world runs events 60
//! times as fast (`RUSTMS_EVENT_SCALE`), so the quest lasts a minute.

use integration_harness::events::{armed_in, enter_opq, party, waiting_in};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::OPQ;
use net::reactors::{salion_next, CELLION, SALION_SPOTS};

const SALION: i32 = 920010300;

/// Kill the Cellion `oid`, standing at `(x, y)`.
async fn kill(player: &mut WorldSession, seen: &mut Trace, oid: i32, (x, y): (i16, i16)) {
    send(&mut player.connection, walk_to(x, y - 10)).await;
    // Its 4400 HP, in blows the sword's cap allows.
    for _ in 0..3 {
        send(&mut player.connection, attack(oid, 2200)).await;
    }
    until(&mut player.connection, seen, |t| {
        t.field.killed.contains(&oid)
    })
    .await;
}

#[tokio::test]
async fn each_cellions_drop_breaks_its_spot_and_the_next_comes_out_further_on() {
    let mut a = vec![armed_in(OPQ.lobby, 51).await];
    for _ in 0..4 {
        a.push(waiting_in(OPQ.lobby, 51, &[]).await);
    }
    party(&mut a).await;
    enter_opq(&mut a).await;
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut a[0].connection, SALION, "", &mut seen).await,
        SALION
    );
    until(&mut a[0].connection, &mut seen, |t| {
        t.field.reactors.len() == 14 && !t.field.alive(CELLION).is_empty()
    })
    .await;
    let spot = |seen: &Trace, reactor_id| {
        *seen
            .field
            .reactors
            .iter()
            .find(|r| r.reactor_id == reactor_id)
            .unwrap()
    };

    // The map's Cellion stands at the first spot; its drop breaks that
    // spot, and the next comes out at 2001013's.
    for reactor_id in [SALION_SPOTS, salion_next(1)] {
        let at = spot(&seen, reactor_id);
        let cellion = seen.field.alive(CELLION)[0];
        kill(&mut a[0], &mut seen, cellion, (at.x, at.y)).await;
        until(&mut a[0].connection, &mut seen, |t| {
            t.field
                .reactors_removed
                .iter()
                .any(|&(oid, _)| oid == at.object_id)
                && t.field.alive(CELLION).len() == 1
        })
        .await;
        assert!(seen.field.item_drop(4001074).is_some(), "{seen:?}");
        // No piece of the statue yet: four Cellions go first at least.
        assert!(seen.field.item_drop(4001045).is_none(), "{seen:?}");
    }
}
