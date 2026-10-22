//! Ludibrium's party quest, its summoning reactors: the other pass put on
//! one of stage 7's brings out Rombad, and a pass put on the boss's brings
//! out Alishar, each with a pink line to everyone there.

use game_data::server_content::keys;
use integration_harness::events::{enter_lpq, party, waiting_in};
use integration_harness::hunting::{walk_to, MapReactor};
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{LPQ, LPQ_BOX_PASS, LPQ_PASS};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const ETC: u8 = 4;
const ROMBAD_REACTOR: i32 = 2201002;
const ROMBAD: i32 = 9300010;
const ALISHAR_REACTOR: i32 = 2201003;
const ALISHAR: i32 = 9300012;

/// Drop one of what lies in the Etc tab's first slot.
fn throw() -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemMove as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(ETC).unwrap();
    p.write_short(1).unwrap();
    p.write_short(0).unwrap();
    p.write_short(1).unwrap();
    p
}

/// On `stage`'s map, put the item in hand on reactor `reactor_id`, which
/// brings out `mob_id` with the pink line `notice`.
async fn summon(player: &mut WorldSession, stage: u8, reactor_id: i32, mob_id: i32, notice: &str) {
    let c = &mut player.connection;
    let map = LPQ.stage_map(stage);
    let mut trace = Trace::default();
    assert_eq!(warp(c, map, "", &mut trace).await, map);
    until(c, &mut trace, |t| {
        t.field.reactors.iter().any(|r| r.reactor_id == reactor_id)
    })
    .await;
    let reactor: MapReactor = *(trace.field.reactors.iter())
        .find(|r| r.reactor_id == reactor_id)
        .unwrap();
    assert!(!trace.field.mobs.iter().any(|&(_, id)| id == mob_id));
    send(c, walk_to(reactor.x, reactor.y - 10)).await;
    send(c, throw()).await;
    until(c, &mut trace, |t| {
        t.field.mobs.iter().any(|&(_, id)| id == mob_id)
    })
    .await;
    let notice = net::content::text().get(notice, &[]);
    assert!(trace.broadcasts.contains(&(5, notice)), "{trace:?}");
    assert!(
        (trace.field.reactors_removed.iter()).any(|&(oid, _)| oid == reactor.object_id),
        "{trace:?}"
    );
}

#[tokio::test]
async fn passes_put_on_their_reactors_bring_out_rombad_and_alishar() {
    let mut a = vec![
        waiting_in(LPQ.lobby, 35, &[(LPQ_BOX_PASS, 1)]).await,
        waiting_in(LPQ.lobby, 35, &[(LPQ_PASS, 1)]).await,
    ];
    for _ in 0..3 {
        a.push(waiting_in(LPQ.lobby, 35, &[]).await);
    }
    party(&mut a).await;
    enter_lpq(&mut a).await;

    // By debug warp, which no gate checks.
    summon(
        &mut a[0],
        7,
        ROMBAD_REACTOR,
        ROMBAD,
        keys::LPQ_NOTICE_ROMBAD,
    )
    .await;
    let last = LPQ.last_stage();
    summon(
        &mut a[1],
        last,
        ALISHAR_REACTOR,
        ALISHAR,
        keys::LPQ_NOTICE_ALISHAR,
    )
    .await;
}
