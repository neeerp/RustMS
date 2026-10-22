//! Orbis's party quest, the garden: once the statue has its six pieces
//! Eak takes the team there. A Nependeath's seed set in a pot grows
//! another Nependeath or a Royal one; a Royal's seed set where a pot stood
//! brings out Papa Pixie, whose seed in the Grass of Life's pot, and four
//! blows, grows the Grass and clears the garden. Only the leader holding
//! it takes the team back to the hub. The harness world pins the pots to
//! one Nependeath then Royals (`RUSTMS_OPQ_POTS`), and runs events 60
//! times as fast, so the quest lasts a minute.

use game_data::server_content::keys;
use integration_harness::events::{
    armed_in, break_open, cleared, enter_opq, maps, party, said, talk, throw_down, told,
    waiting_in, EAK,
};
use integration_harness::hunting::{attack, pickup, walk_to, Field, MapReactor};
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{GRASS_OF_LIFE, OPQ, OPQ_GARDEN, OPQ_HUB};
use net::packet::op::RecvOpcode;
use net::reactors::{NEPENDEATH, PAPA_PIXIE, ROYAL_NEPENDEATH};
use net::scripts::OPQ_GARDEN_OUT;
use packet::io::write::PktWrite;
use packet::Packet;

const POT: i32 = 2001000;
const TRIGGER: i32 = 2001016;
const GRASS_POT: i32 = 2002003;
const SEED: i32 = 4001053;
const ROYAL_SEED: i32 = 4001074;
const PIXIE_SEED: i32 = 4001054;
/// The hub's scars and the piece each takes, as the pieces sit in the
/// holder's Etc slots 1 to 6 (4001044 to 4001049).
const SCARS: [(i32, i16); 6] = [
    (2008000, 3),
    (2008001, 6),
    (2008002, 4),
    (2008003, 1),
    (2008004, 5),
    (2008005, 2),
];

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// The name of the garden's way out.
fn way_out() -> String {
    let field = maps().field(OPQ_GARDEN).unwrap();
    let portal = field
        .portals()
        .find(|p| p.script.as_deref() == Some(OPQ_GARDEN_OUT));
    portal.unwrap().name.clone()
}

fn reactor_at(seen: &Trace, reactor_id: i32, (x, y): (i16, i16)) -> MapReactor {
    *seen
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == reactor_id && (r.x, r.y) == (x, y))
        .unwrap()
}

fn pink(trace: &Trace, key: &str) -> bool {
    let text = net::content::text().get(key, &[]);
    trace.broadcasts.iter().any(|(_, said)| *said == text)
}

/// Kill the mob `oid` in blows of 30000.
async fn kill(player: &mut WorldSession, seen: &mut Trace, oid: i32, hp: i32) {
    for _ in 0..(hp + 29_999) / 30_000 {
        send(&mut player.connection, attack(oid, 30_000)).await;
    }
    until(&mut player.connection, seen, |t| {
        t.field.killed.contains(&oid)
    })
    .await;
}

/// Pick up the drop of `item` still lying on the field, and the Etc slot
/// it went to.
async fn pick_up(player: &mut WorldSession, seen: &mut Trace, item: i32) -> i16 {
    let c = &mut player.connection;
    until(c, seen, |t| {
        t.field
            .drops
            .iter()
            .any(|d| !d.meso && d.value == item && t.field.removals(d.object_id).is_empty())
    })
    .await;
    let drop = *seen
        .field
        .drops
        .iter()
        .find(|d| !d.meso && d.value == item && seen.field.removals(d.object_id).is_empty())
        .unwrap();
    let before = seen.added.len();
    send(c, walk_to(drop.x, drop.y)).await;
    send(c, pickup(drop.object_id, drop.x, drop.y)).await;
    until(c, seen, |t| t.added[before..].iter().any(|a| a.2 == item)).await;
    seen.added[before..].iter().find(|a| a.2 == item).unwrap().1
}

/// Set the item in Etc `slot` down on `reactor`, and wait to see it take
/// it.
async fn set_in(player: &mut WorldSession, seen: &mut Trace, reactor: MapReactor, slot: i16) {
    let c = &mut player.connection;
    send(c, walk_to(reactor.x, reactor.y - 10)).await;
    send(c, throw_down(slot, 1)).await;
    until(c, seen, |t| {
        t.field.reactor_hits.contains(&(reactor.object_id, 1))
            || t.field
                .reactors_removed
                .iter()
                .any(|&(oid, _)| oid == reactor.object_id)
    })
    .await;
}

/// Wait for a new `mob_id` past the first `before` mobs `seen` has seen.
async fn grows(player: &mut WorldSession, seen: &mut Trace, before: usize, mob_id: i32) -> i32 {
    until(&mut player.connection, seen, |t| {
        t.field.mobs[before..].iter().any(|&(_, id)| id == mob_id)
    })
    .await;
    let &(oid, _) = seen.field.mobs[before..]
        .iter()
        .find(|&&(_, id)| id == mob_id)
        .unwrap();
    assert!(
        seen.field.mobs[before..]
            .iter()
            .all(|&(_, id)| id == mob_id),
        "{seen:?}"
    );
    oid
}

#[tokio::test]
async fn the_gardens_seeds_bring_out_papa_pixie_and_the_grass_leads_back() {
    let mut a = vec![armed_in(OPQ.lobby, 51).await];
    let pieces: Vec<_> = (4001044..=4001049).map(|piece| (piece, 1)).collect();
    a.push(waiting_in(OPQ.lobby, 51, &pieces).await);
    for _ in 0..3 {
        a.push(waiting_in(OPQ.lobby, 51, &[]).await);
    }
    party(&mut a).await;
    enter_opq(&mut a).await;

    // Until the statue has its pieces Eak only asks for them.
    let mut seen = Trace::default();
    for player in a.iter_mut().take(2) {
        assert_eq!(
            warp(&mut player.connection, OPQ_HUB, "", &mut seen).await,
            OPQ_HUB
        );
    }
    let trace = told(&mut a[0], talk(OPQ_HUB, EAK)).await;
    assert!(said(&trace, keys::OPQ_EAK_GATHER), "{trace:?}");
    let holder = &mut a[1];
    until(&mut holder.connection, &mut seen, |t| {
        t.field
            .reactors
            .iter()
            .filter(|r| (2008000..=2008005).contains(&r.reactor_id))
            .count()
            == 6
    })
    .await;
    for (scar, slot) in SCARS {
        let at = *seen
            .field
            .reactors
            .iter()
            .find(|r| r.reactor_id == scar)
            .unwrap();
        set_in(holder, &mut seen, at, slot).await;
    }

    // Now the leader's word takes everyone to the garden.
    let mut seen = told(&mut a[0], talk(OPQ_HUB, EAK)).await;
    until(&mut a[0].connection, &mut seen, |t| {
        t.warps.contains(&OPQ_GARDEN)
    })
    .await;
    for player in &mut a[1..] {
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&OPQ_GARDEN)
        })
        .await;
    }
    seen.field = Field::default();
    until(&mut a[0].connection, &mut seen, |t| {
        t.field.reactors.len() == 18 && !t.field.alive(NEPENDEATH).is_empty()
    })
    .await;
    let trace = told(&mut a[1], talk(OPQ_GARDEN, EAK)).await;
    assert!(said(&trace, keys::OPQ_EAK_GARDEN), "{trace:?}");

    // A Nependeath's seed in a pot grows another Nependeath; that one's,
    // in the next pot, a Royal.
    let first = seen.field.alive(NEPENDEATH)[0];
    kill(&mut a[0], &mut seen, first, 10_000).await;
    let slot = pick_up(&mut a[0], &mut seen, SEED).await;
    let pot = reactor_at(&seen, POT, (200, -338));
    let before = seen.field.mobs.len();
    set_in(&mut a[0], &mut seen, pot, slot).await;
    let grown = grows(&mut a[0], &mut seen, before, NEPENDEATH).await;
    kill(&mut a[0], &mut seen, grown, 10_000).await;
    let slot = pick_up(&mut a[0], &mut seen, SEED).await;
    let pot = reactor_at(&seen, POT, (315, -339));
    let before = seen.field.mobs.len();
    set_in(&mut a[0], &mut seen, pot, slot).await;
    let royal = grows(&mut a[0], &mut seen, before, ROYAL_NEPENDEATH).await;

    // The Royal falls where it grew, and its seed on the trigger under the
    // pot: every mob goes, Papa Pixie comes, and everyone is told.
    let trigger = reactor_at(&seen, TRIGGER, (318, -334));
    kill(&mut a[0], &mut seen, royal, 10_000).await;
    until(&mut a[0].connection, &mut seen, |t| {
        !t.field.alive(PAPA_PIXIE).is_empty() && pink(t, keys::OPQ_NOTICE_PAPA_PIXIE)
    })
    .await;
    let seed = seen.field.item_drop(ROYAL_SEED).unwrap();
    assert_eq!(seen.field.removals(seed.object_id), [None], "{seen:?}");
    assert!(
        seen.field
            .reactors_removed
            .iter()
            .any(|&(oid, _)| oid == trigger.object_id),
        "{seen:?}"
    );
    assert!(seen.field.alive(NEPENDEATH).is_empty(), "{seen:?}");

    // His seed in the Grass of Life's pot, four blows, and the Grass grows
    // and the garden is cleared.
    let papa = seen.field.alive(PAPA_PIXIE)[0];
    kill(&mut a[0], &mut seen, papa, 672_000).await;
    let slot = pick_up(&mut a[0], &mut seen, PIXIE_SEED).await;
    let grass_pot = reactor_at(&seen, GRASS_POT, (-755, 19));
    set_in(&mut a[0], &mut seen, grass_pot, slot).await;
    break_open(&mut a[0], &mut seen, grass_pot, GRASS_OF_LIFE).await;
    until(&mut a[0].connection, &mut seen, cleared).await;

    // Without it in hand, even the leader stays; with it, the leader
    // takes the team to the hub, where Eak wants it at the statue.
    let out = way_out();
    for player in a.iter_mut().take(2) {
        let trace = told(player, enter(&out)).await;
        assert!(trace.arrivals.is_empty(), "{trace:?}");
        assert!(pink(&trace, keys::OPQ_NOTICE_GARDEN_OUT), "{trace:?}");
    }
    pick_up(&mut a[0], &mut seen, GRASS_OF_LIFE).await;
    let trace = told(&mut a[0], enter(&out)).await;
    assert_eq!(trace.portal_sounds, 1, "{trace:?}");
    for player in &mut a {
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&OPQ_HUB)
        })
        .await;
    }
    let trace = told(&mut a[0], talk(OPQ_HUB, EAK)).await;
    assert!(said(&trace, keys::OPQ_EAK_GRASS), "{trace:?}");
}
