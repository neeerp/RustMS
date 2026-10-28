//! Crimsonwood Keep's fourth stage (report b269): its flames and stirges go
//! off as players step on them (TOUCH_REACTOR). A flame lights while anyone
//! stands on it, and puts out the fires it governs and lights the others;
//! it flips them back when the last one steps off, or leaves. Five standing on
//! the two stirges at once see off every mob there, once.

use game_data::server_content::keys;
use integration_harness::events::{go_to, waiting_in};
use integration_harness::hunting::{touch_reactor, walk_to, MapReactor};
use integration_harness::trace::{barrier, read_for, send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{cwk_fires, cwk_flame, cwk_flip, CWK_ENTRY, CWK_STIRGE, CWK_STIRGE_STANDING};

const STAGE4: i32 = CWK_ENTRY + 300;
const HENESYS: i32 = 100_000_000;
/// The flame `i_a`, at (-137, 495).
const FLAME: i32 = 6_109_014;
const BLOOD_STIRGE: i32 = 9_400_595;

/// Where the fire `name` stands last, as `t` saw it, or `None` if no one
/// lit it.
fn fire(t: &Trace, name: &str) -> Option<i32> {
    let mut fires = t.field.obstacles.iter().rev();
    fires.find(|(n, _)| n == name).map(|&(_, state)| state)
}

/// Whether `t` sees the fires governed by `flame` as `states` has them.
fn fires_as(t: &Trace, flame: i32, states: impl Fn(&str) -> Option<i32>) -> bool {
    let names = cwk_flame(flame).unwrap();
    names.iter().all(|name| fire(t, name) == states(name))
}

/// The fires as the stage starts them.
fn lit(name: &str) -> Option<i32> {
    let fires = cwk_fires();
    let mut lit = fires.iter();
    lit.find(|(n, _)| n == name).map(|&(_, state)| state)
}

/// The fires as `lit`, flipped.
fn flipped(name: &str) -> Option<i32> {
    Some(cwk_flip(lit(name)))
}

/// The fires flipped back: those lit lit again, the others out.
fn back(name: &str) -> Option<i32> {
    Some(cwk_flip(flipped(name)))
}

/// The reactor's moves to `state`, as `t` saw them.
fn moves(t: &Trace, reactor: MapReactor, state: u8) -> usize {
    let hits = t.field.reactor_hits.iter();
    hits.filter(|&&hit| hit == (reactor.object_id, state))
        .count()
}

/// A new player in Henesys, gone to the fourth stage to see `reactor_id`
/// and the stage's fires.
async fn at_stage(reactor_id: i32) -> (WorldSession, Trace, MapReactor) {
    let mut player = waiting_in(HENESYS, 30, &[]).await;
    let (mut seen, reactor) = go_to(&mut player, STAGE4, reactor_id).await;
    until(&mut player.connection, &mut seen, |t| {
        !t.field.obstacles.is_empty()
    })
    .await;
    (player, seen, reactor)
}

/// Step on `reactor`, and wait till the field has seen it, recording what
/// comes meanwhile in `seen`.
async fn step_on(player: &mut WorldSession, seen: &mut Trace, reactor: MapReactor) {
    let c = &mut player.connection;
    send(c, walk_to(reactor.x, reactor.y)).await;
    send(c, touch_reactor(reactor.object_id, true)).await;
    barrier(c, &player.character_name, seen).await;
}

#[tokio::test]
async fn the_fourth_stage_s_flames_and_stirges_go_off_underfoot() {
    let (mut a, mut at, flame) = at_stage(FLAME).await;
    let (mut b, mut bt, _) = at_stage(FLAME).await;

    // Those who come see the lit fires.
    let mut fires = at.field.obstacles.clone();
    fires.sort();
    let mut lit_fires = cwk_fires();
    lit_fires.sort();
    assert_eq!(fires, lit_fires);
    assert_eq!(fire(&at, "a1"), Some(1));
    assert_eq!(fire(&at, "b2"), None);

    // A flame can't be stepped on from across the map.
    let c = &mut b.connection;
    send(c, walk_to(flame.x + 1000, flame.y)).await;
    send(c, touch_reactor(flame.object_id, true)).await;
    read_for(c, &mut bt, 1500).await;
    assert_eq!(moves(&bt, flame, 1), 0, "{bt:?}");

    // The first one on lights it and flips its fires, for all to see.
    step_on(&mut a, &mut at, flame).await;
    until(&mut b.connection, &mut bt, |t| {
        moves(t, flame, 1) == 1 && fires_as(t, FLAME, flipped)
    })
    .await;
    until(&mut a.connection, &mut at, |t| {
        moves(t, flame, 1) == 1 && fires_as(t, FLAME, flipped)
    })
    .await;
    assert_eq!(fire(&at, "a1"), Some(2));
    assert_eq!(fire(&at, "b2"), Some(1));

    // Another on, and the first off, change nothing while one stands.
    let hits = at.field.reactor_hits.len();
    let blocks = at.field.obstacles.len();
    step_on(&mut b, &mut bt, flame).await;
    send(&mut a.connection, touch_reactor(flame.object_id, false)).await;
    read_for(&mut a.connection, &mut at, 1500).await;
    assert_eq!(at.field.reactor_hits.len(), hits, "{at:?}");
    assert_eq!(at.field.obstacles.len(), blocks, "{at:?}");

    // The last one leaving steps off it, and it goes back.
    let mut gone = Trace::default();
    assert_eq!(
        warp(&mut b.connection, HENESYS, "", &mut gone).await,
        HENESYS
    );
    until(&mut a.connection, &mut at, |t| {
        moves(t, flame, 0) == 1 && fires_as(t, FLAME, back)
    })
    .await;

    // The stirges: fewer than five on them change nothing.
    until(&mut a.connection, &mut at, |t| {
        !t.field.alive(BLOOD_STIRGE).is_empty()
    })
    .await;
    let stirges: Vec<MapReactor> = (at.field.reactors.iter())
        .filter(|r| r.reactor_id == CWK_STIRGE)
        .copied()
        .collect();
    assert_eq!(stirges.len(), 2, "{:?}", at.field.reactors);
    let mut team = vec![a];
    for _ in 1..CWK_STIRGE_STANDING {
        team.push(at_stage(CWK_STIRGE).await.0);
    }
    let last = team.len() - 1;
    for (n, player) in team[..last].iter_mut().enumerate() {
        let mut seen = Trace::default();
        let seen = if n == 0 { &mut at } else { &mut seen };
        step_on(player, seen, stirges[n % 2]).await;
    }
    read_for(&mut team[0].connection, &mut at, 1500).await;
    assert!(at.item_lines.is_empty(), "{at:?}");
    assert!(at.field.killed.is_empty(), "{at:?}");

    // The fifth, on either, sees off every mob, and says so.
    let alive = at.field.alive(BLOOD_STIRGE);
    step_on(&mut team[last], &mut Trace::default(), stirges[0]).await;
    let gone_line = (net::content::text().get(keys::CWK_STIRGES_GONE, &[]), 0);
    until(&mut team[0].connection, &mut at, |t| {
        t.item_lines.contains(&gone_line) && t.field.alive(BLOOD_STIRGE).is_empty()
    })
    .await;
    assert!(alive.iter().all(|oid| at.field.killed.contains(oid)));

    // Just the once.
    let c = &mut team[last].connection;
    send(c, touch_reactor(stirges[0].object_id, false)).await;
    send(c, touch_reactor(stirges[0].object_id, true)).await;
    read_for(&mut team[0].connection, &mut at, 1500).await;
    let heard = at.item_lines.iter().filter(|&l| *l == gone_line).count();
    assert_eq!(heard, 1, "{at:?}");
}
