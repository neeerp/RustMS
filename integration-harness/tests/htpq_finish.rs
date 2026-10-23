//! Horntail's party quest, its end: in the lamp's room only the leader may
//! take the way on, and only once the lamp is lit or dark; three hits make
//! it dark, and the way takes the whole team to the dark map. There a crate
//! breaks in four hits for a Red Crystal Key, and the leader hands the
//! gatekeeper the Blue Key and six Red ones, which wins the quest and takes
//! the team to the Entrance to Horntail's Cave. The harness world runs
//! events 60 times as fast (`RUSTMS_EVENT_SCALE`), so the quest lasts 25
//! seconds.

use game_data::server_content::keys;
use integration_harness::events::{
    enter_htpq, maps, party, said, talk, told, waiting_in, HTPQ_KEEPER,
};
use integration_harness::hunting::{hit_reactor, pickup, walk_to, MapReactor};
use integration_harness::trace::{send, until, Trace};
use integration_harness::WorldSession;
use net::events::{
    BLUE_CRYSTAL_KEY, CRYSTAL_KEYS, HTPQ, HTPQ_DARK, HTPQ_ENTRY, HTPQ_LAMP, RED_CRYSTAL_KEY,
    RED_KEYS_WANTED,
};
use net::packet::op::RecvOpcode;
use net::reactors::HTPQ_LAMP as LAMP;
use net::scripts::HTPQ_LAMP_DOOR;
use packet::io::write::PktWrite;
use packet::Packet;

const HORNTAIL_CAVE_ENTRANCE: i32 = 240_050_400;
const CRATE: i32 = 2402006;

/// CHANGE_MAP_SPECIAL: press Up on the lamp's room's way on.
fn way_on() -> Packet {
    let field = maps().field(HTPQ_LAMP).unwrap();
    let portal = field
        .portals()
        .find(|p| p.script.as_deref() == Some(HTPQ_LAMP_DOOR))
        .unwrap();
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(&portal.name).unwrap();
    p.write_short(0).unwrap();
    p
}

/// Wait until `player` is moved to `map`, carrying on `t`.
async fn moved_to(player: &mut WorldSession, map: i32, mut t: Trace) -> Trace {
    until(&mut player.connection, &mut t, |t| t.warps.contains(&map)).await;
    t
}

/// `player` presses Up on the way on and is told `key` in pink.
async fn kept_back(player: &mut WorldSession, key: &str) {
    let line = net::content::text().get(key, &[]);
    let mut t = told(player, way_on()).await;
    until(&mut player.connection, &mut t, |t| {
        t.broadcasts.contains(&(5, line.clone()))
    })
    .await;
    assert!(t.warps.is_empty(), "{t:?}");
}

/// The first `reactor_id` shown on `player`'s map, read into `seen` before
/// any other trace can swallow its spawn.
async fn reactor(player: &mut WorldSession, seen: &mut Trace, reactor_id: i32) -> MapReactor {
    let on_map = |t: &Trace| {
        t.field
            .reactors
            .iter()
            .find(|r| r.reactor_id == reactor_id)
            .copied()
    };
    until(&mut player.connection, seen, |t| on_map(t).is_some()).await;
    on_map(seen).unwrap()
}

/// Hit `object_id` until it reaches `state`, a hit at a time.
async fn hit_to(player: &mut WorldSession, seen: &mut Trace, object_id: i32, states: &[u8]) {
    for &state in states {
        send(&mut player.connection, hit_reactor(object_id)).await;
        until(&mut player.connection, seen, |t| {
            t.field.reactor_hits.contains(&(object_id, state))
        })
        .await;
    }
}

#[tokio::test]
async fn the_dark_lamp_leads_to_the_crates_and_the_keys_win_it() {
    // The leader holds the five Crystal Keys, the Blue Key, and all but one
    // of the Red Keys.
    let mut held: Vec<_> = CRYSTAL_KEYS.iter().map(|&key| (key, 1)).collect();
    held.push((BLUE_CRYSTAL_KEY, 1));
    held.push((RED_CRYSTAL_KEY, RED_KEYS_WANTED - 1));
    let mut a = vec![waiting_in(HTPQ.lobby, 120, &held).await];
    for _ in 0..5 {
        a.push(waiting_in(HTPQ.lobby, 120, &[]).await);
    }
    party(&mut a).await;
    enter_htpq(&mut a).await;

    // The five keys take the team on to the lamp.
    let trace = told(&mut a[0], talk(HTPQ_ENTRY, HTPQ_KEEPER)).await;
    let mut seen = moved_to(&mut a[0], HTPQ_LAMP, trace).await;
    for player in a[1..].iter_mut() {
        moved_to(player, HTPQ_LAMP, Trace::default()).await;
    }

    let lamp = reactor(&mut a[0], &mut seen, LAMP).await;
    assert_eq!(lamp.state, 0);

    // Only the leader may go on, and only once the lamp is lit or dark.
    kept_back(&mut a[1], keys::HTPQ_LAMP_LEADER_ONLY).await;
    kept_back(&mut a[0], keys::HTPQ_LAMP_HIT).await;

    // Three hits make it dark: the way on takes the whole team into the dark.
    send(&mut a[0].connection, walk_to(lamp.x, lamp.y)).await;
    hit_to(&mut a[0], &mut seen, lamp.object_id, &[1, 2, 3]).await;
    assert!(seen.field.reactors_removed.is_empty(), "{seen:?}");
    let trace = told(&mut a[0], way_on()).await;
    let mut seen = moved_to(&mut a[0], HTPQ_DARK, trace).await;
    for player in a[1..].iter_mut() {
        moved_to(player, HTPQ_DARK, Trace::default()).await;
    }
    let crate_ = reactor(&mut a[0], &mut seen, CRATE).await;

    // Short of a Red Key, the gatekeeper waits.
    let trace = told(&mut a[0], talk(HTPQ_DARK, HTPQ_KEEPER)).await;
    assert!(said(&trace, keys::HTPQ_KEEPER_LAST_KEYS_SHORT), "{trace:?}");
    assert!(trace.removed.is_empty(), "{trace:?}");

    // A crate breaks in four hits for the last Red Key.
    send(&mut a[0].connection, walk_to(crate_.x, crate_.y)).await;
    hit_to(&mut a[0], &mut seen, crate_.object_id, &[1, 2, 3]).await;
    send(&mut a[0].connection, hit_reactor(crate_.object_id)).await;
    until(&mut a[0].connection, &mut seen, |t| {
        t.field.item_drop(RED_CRYSTAL_KEY).is_some()
    })
    .await;
    let key = seen.field.item_drop(RED_CRYSTAL_KEY).unwrap();
    send(&mut a[0].connection, pickup(key.object_id, key.x, key.y)).await;
    // It stacks with the others, so the slot's count goes up to six.
    until(&mut a[0].connection, &mut seen, |t| {
        t.quantities.iter().any(|&(_, _, n)| n == RED_KEYS_WANTED)
    })
    .await;

    // The Blue Key and six Red ones win it: everyone goes on to the Entrance
    // to Horntail's Cave.
    let trace = told(&mut a[0], talk(HTPQ_DARK, HTPQ_KEEPER)).await;
    let seen = moved_to(&mut a[0], HORNTAIL_CAVE_ENTRANCE, trace).await;
    assert!(seen.removed.len() >= 2, "{seen:?}");
    for player in a[1..].iter_mut() {
        moved_to(player, HORNTAIL_CAVE_ENTRANCE, Trace::default()).await;
    }
}
