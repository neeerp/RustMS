//! Orbis's party quest, the jails under the sealed room (report b185).
//! Their doors stay shut until the event is won; inside, each lever drops
//! its trap's spears, which hurt the Lucidas the clients see them touch
//! (FIELD_DAMAGE_MOB), and the way on opens to the right levers only. The
//! harness world pins the first jail's to levers 1, 3 and 5
//! (`RUSTMS_OPQ_JAIL1`).

use game_data::server_content::keys;
use integration_harness::events::{cleared, enter_opq, party, told, waiting_in, wrong};
use integration_harness::hunting::{hit_reactor, trap_hit, walk_to, MapReactor};
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::OPQ;
use net::packet::op::RecvOpcode;
use net::reactors::{OPQ_CRITICAL_BOX, OPQ_LEVER};
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::{sleep, Duration};

const SEALED_ROOM: i32 = 920_010_900;
const JAIL: i32 = 920_010_910;
const BOX_ROOM: i32 = 920_010_912;
const LUCIDA: i32 = 9_300_044;
/// Where Map.nx puts the first jail's levers, `lever1` to `lever7`.
const LEVERS: [(i16, i16); 7] = [
    (-280, 471),
    (195, 466),
    (-70, 505),
    (-136, 99),
    (32, -53),
    (-116, -50),
    (-291, 99),
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

fn pink(trace: &Trace, key: &str, vars: &[(&str, &str)]) -> bool {
    let text = net::content::text().get(key, vars);
    trace
        .broadcasts
        .iter()
        .any(|(kind, said)| *kind == 5 && *said == text)
}

fn lever(seen: &Trace, name: usize) -> MapReactor {
    let at = LEVERS[name - 1];
    *seen
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == OPQ_LEVER && (r.x, r.y) == at)
        .unwrap()
}

/// Pull the lever `lever<name>`, seen in `seen`, and wait to see it move.
async fn pull(player: &mut WorldSession, seen: &mut Trace, name: usize) {
    let lever = lever(seen, name);
    let moves = |t: &Trace| {
        let hits = t.field.reactor_hits.iter();
        hits.filter(|&&(oid, _)| oid == lever.object_id).count()
    };
    let before = moves(seen);
    let c = &mut player.connection;
    send(c, walk_to(lever.x, lever.y)).await;
    send(c, hit_reactor(lever.object_id)).await;
    until(c, seen, |t| moves(t) > before).await;
}

/// Warp `player` to the first jail and wait to see its levers and
/// Lucidas.
async fn to_jail(player: &mut WorldSession) -> Trace {
    let mut seen = Trace::default();
    let c = &mut player.connection;
    assert_eq!(warp(c, JAIL, "", &mut seen).await, JAIL);
    until(c, &mut seen, |t| {
        let levers = t
            .field
            .reactors
            .iter()
            .filter(|r| r.reactor_id == OPQ_LEVER);
        levers.count() == 7 && t.field.alive(LUCIDA).len() == 8
    })
    .await;
    seen
}

#[tokio::test]
async fn the_jails_levers_drop_spears_on_the_lucidas_and_open_the_way_on() {
    let mut a = vec![];
    for _ in 0..5 {
        a.push(waiting_in(OPQ.lobby, 51, &[]).await);
    }
    party(&mut a).await;
    enter_opq(&mut a).await;

    // Until the event is won the sealed room's doors stay shut.
    let mut trace = Trace::default();
    let sealed = warp(&mut a[0].connection, SEALED_ROOM, "", &mut trace).await;
    assert_eq!(sealed, SEALED_ROOM);
    let trace = told(&mut a[0], enter("in01")).await;
    assert!(trace.arrivals.is_empty(), "{trace:?}");
    assert!(
        pink(&trace, keys::OPQ_NOTICE_STORAGE_CLOSED, &[]),
        "{trace:?}"
    );

    let mut seen = to_jail(&mut a[0]).await;
    let mut other = to_jail(&mut a[1]).await;
    let lucida = seen.field.alive(LUCIDA)[0];

    // No trap has moved yet, so no trap hurts.
    told(&mut a[1], trap_hit(lucida, 1000)).await;

    // A lever drops its trap's spears for everyone here, and for those
    // who come later.
    pull(&mut a[0], &mut seen, 1).await;
    until(&mut a[1].connection, &mut other, |t| {
        t.field.obstacles == [("trap1".to_string(), 1)]
    })
    .await;
    until(&mut a[0].connection, &mut seen, |t| {
        !t.field.obstacles.is_empty()
    })
    .await;
    assert_eq!(seen.field.obstacles, [("trap1".to_string(), 1)]);
    assert!(seen.field.mobs_hurt.is_empty(), "{seen:?}");
    let mut late = to_jail(&mut a[2]).await;
    until(&mut a[2].connection, &mut late, |t| {
        !t.field.obstacles.is_empty()
    })
    .await;
    assert_eq!(late.field.obstacles, [("trap1".to_string(), 1)]);

    // Now they do, as much as a spear may roll, once a second for each
    // Lucida, however many clients tell of it; three hits fell one.
    told(&mut a[1], trap_hit(lucida, 1101)).await;
    told(&mut a[1], trap_hit(lucida, 1100)).await;
    send(&mut a[0].connection, trap_hit(lucida, 1050)).await;
    let hurt = |t: &Trace| t.field.mobs_hurt.clone();
    until(&mut a[0].connection, &mut seen, |t| !hurt(t).is_empty()).await;
    assert_eq!(hurt(&seen), [(lucida, 1100, 1900)], "{seen:?}");
    for _ in 0..2 {
        sleep(Duration::from_millis(1000)).await;
        send(&mut a[1].connection, trap_hit(lucida, 1000)).await;
    }
    until(&mut a[0].connection, &mut seen, |t| {
        t.field.killed.contains(&lucida)
    })
    .await;
    let hits = [(lucida, 1100, 1900), (lucida, 1000, 900), (lucida, 1000, 0)];
    assert_eq!(hurt(&seen), hits);

    // With two of the three levers down the way on stays shut, and the
    // whole jail hears so.
    let trace = told(&mut a[0], enter("in00")).await;
    assert!(trace.arrivals.is_empty(), "{trace:?}");
    assert!(wrong(&trace), "{trace:?}");
    assert!(
        trace
            .field_effects
            .contains(&(4, "Party1/Failed".to_string())),
        "{trace:?}"
    );
    let vars = [("n", "2")];
    assert!(
        pink(&trace, keys::OPQ_NOTICE_JAIL_LEVERS, &vars),
        "{trace:?}"
    );
    until(&mut a[1].connection, &mut other, wrong).await;

    // With them right it opens to the critical box's room.
    pull(&mut a[0], &mut seen, 3).await;
    pull(&mut a[0], &mut seen, 5).await;
    let mut room = told(&mut a[0], enter("in00")).await;
    assert!(cleared(&room), "{room:?}");
    assert_eq!(room.portal_sounds, 1, "{room:?}");
    let [(to, 0, _)] = room.arrivals[..] else {
        panic!("{room:?}");
    };
    assert_eq!(to, BOX_ROOM);
    until(&mut a[1].connection, &mut other, cleared).await;

    // And stays open, however the levers stand after, with no more cheers.
    pull(&mut a[1], &mut other, 2).await;
    let trace = told(&mut a[1], enter("in00")).await;
    assert!(
        matches!(trace.arrivals[..], [(BOX_ROOM, 0, _)]),
        "{trace:?}"
    );
    assert!(!cleared(&trace), "{trace:?}");

    // The critical box stands ready for its four blows, and sprays.
    until(&mut a[0].connection, &mut room, |t| {
        t.field
            .reactors
            .iter()
            .any(|r| r.reactor_id == OPQ_CRITICAL_BOX)
    })
    .await;
    let critical = *room
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == OPQ_CRITICAL_BOX)
        .unwrap();
    assert_eq!(critical.state, 1);
    let c = &mut a[0].connection;
    send(c, walk_to(critical.x, critical.y)).await;
    for state in 2..=4u8 {
        send(c, hit_reactor(critical.object_id)).await;
        until(c, &mut room, |t| {
            t.field.reactor_hits.contains(&(critical.object_id, state))
        })
        .await;
    }
    send(c, hit_reactor(critical.object_id)).await;
    until(c, &mut room, |t| {
        let gone = t.field.reactors_removed.iter();
        gone.clone().any(|&(oid, _)| oid == critical.object_id) && t.field.drops.len() >= 15
    })
    .await;
}
