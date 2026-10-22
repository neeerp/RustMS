//! Orbis's party quest, the lounge's climb and the tall room. Their pads
//! lead a row up or back down; at the tall room's top are five levers,
//! two of which Eak wants up before he brings out its stone box. The
//! harness world pins the lounge's right pads to 2 then 3
//! (`RUSTMS_OPQ_LOUNGE`), the tall room's to 1, 2, 3, 4 over and over
//! (`RUSTMS_OPQ_TOWER`) and Eak's levers to the first and fourth
//! (`RUSTMS_OPQ_LEVERS`), and runs events 60 times as fast, so the quest
//! lasts a minute.

use game_data::server_content::keys;
use integration_harness::events::{
    break_open, cleared, enter_opq, go_to, party, said, talk, told, waiting_in, wrong, EAK,
};
use integration_harness::hunting::{hit_reactor, walk_to, MapReactor};
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{OPQ, OPQ_LEVER_ROOM, OPQ_LOUNGE};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const LEVER: i32 = 2008007;
const STONE6: i32 = 2002013;
/// Where Map.nx puts the levers named "1" to "5".
const LEVERS: [(i16, i16); 5] = [
    (-283, -4980),
    (207, -4983),
    (-49, -4980),
    (77, -5053),
    (-164, -5051),
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

/// The portal of `map` that stepping on the pad `pad` led to, after its
/// sound.
async fn climb(player: &mut WorldSession, map: i32, pad: &str) -> u8 {
    let trace = told(player, enter(pad)).await;
    assert_eq!(trace.portal_sounds, 1, "{pad}: {trace:?}");
    let [(to, portal, _)] = trace.arrivals[..] else {
        panic!("{pad}: {:?}", trace.arrivals);
    };
    assert_eq!(to, map, "{pad}");
    portal
}

/// Pull the lever named `name` (1-5), seen in `seen`, and wait to see it
/// move.
async fn pull(player: &mut WorldSession, seen: &mut Trace, name: usize) {
    let (x, y) = LEVERS[name - 1];
    let lever = lever(seen, name);
    let moves = |t: &Trace| {
        t.field
            .reactor_hits
            .iter()
            .filter(|&&(oid, _)| oid == lever.object_id)
            .count()
    };
    let before = moves(seen);
    let c = &mut player.connection;
    send(c, walk_to(x, y)).await;
    send(c, hit_reactor(lever.object_id)).await;
    until(c, seen, |t| moves(t) > before).await;
}

fn lever(seen: &Trace, name: usize) -> MapReactor {
    let (x, y) = LEVERS[name - 1];
    *seen
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == LEVER && (r.x, r.y) == (x, y))
        .unwrap()
}

/// What Eak says to `player` in the tall room.
async fn ask_eak(player: &mut WorldSession) -> Trace {
    told(player, talk(OPQ_LEVER_ROOM, EAK)).await
}

#[tokio::test]
async fn the_climbs_lead_up_and_the_right_levers_bring_out_the_stone_box() {
    let mut a = vec![];
    for _ in 0..5 {
        a.push(waiting_in(OPQ.lobby, 51, &[]).await);
    }
    party(&mut a).await;
    enter_opq(&mut a).await;

    // The lounge: the right pad leads a row up, then to the top
    // (`success`); a wrong one down to `fail`.
    let p = &mut a[0];
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut p.connection, OPQ_LOUNGE, "", &mut trace).await,
        OPQ_LOUNGE
    );
    assert_eq!(climb(p, OPQ_LOUNGE, "r4rp12").await, 15);
    assert_eq!(climb(p, OPQ_LOUNGE, "r4rp23").await, 1);
    assert_eq!(climb(p, OPQ_LOUNGE, "r4rp21").await, 2);
    assert_eq!(climb(p, OPQ_LOUNGE, "r4rp11").await, 2);

    // The tall room: a row up, a section's landing after every fourth,
    // and a wrong pad back to the start of the section.
    assert_eq!(
        warp(&mut p.connection, OPQ_LEVER_ROOM, "", &mut trace).await,
        OPQ_LEVER_ROOM
    );
    assert_eq!(climb(p, OPQ_LEVER_ROOM, "rp011").await, 28);
    assert_eq!(climb(p, OPQ_LEVER_ROOM, "rp044").await, 1);
    assert_eq!(climb(p, OPQ_LEVER_ROOM, "rp164").await, 4);
    assert_eq!(climb(p, OPQ_LEVER_ROOM, "rp013").await, 5);
    assert_eq!(climb(p, OPQ_LEVER_ROOM, "rp053").await, 1);
    assert_eq!(climb(p, OPQ_LEVER_ROOM, "rp093").await, 2);
    assert_eq!(climb(p, OPQ_LEVER_ROOM, "rp163").await, 3);
    // Up by the levers again.
    let (mut seen, stone6) = go_to(&mut a[0], OPQ_LEVER_ROOM, STONE6).await;
    until(&mut a[0].connection, &mut seen, |t| {
        t.field
            .reactors
            .iter()
            .filter(|r| r.reactor_id == LEVER)
            .count()
            == 5
    })
    .await;
    go_to(&mut a[1], OPQ_LEVER_ROOM, STONE6).await;

    // Until two levers are up Eak only asks for them.
    let trace = ask_eak(&mut a[1]).await;
    assert!(said(&trace, keys::OPQ_EAK_LEVERS_ASK), "{trace:?}");
    assert!(!wrong(&trace), "{trace:?}");
    pull(&mut a[0], &mut seen, 1).await;
    let trace = ask_eak(&mut a[1]).await;
    assert!(said(&trace, keys::OPQ_EAK_LEVERS_ASK), "{trace:?}");
    // The first and second: one of them is his.
    pull(&mut a[0], &mut seen, 2).await;
    let trace = ask_eak(&mut a[1]).await;
    assert!(said(&trace, keys::OPQ_EAK_LEVERS_ONE_RIGHT), "{trace:?}");
    assert!(wrong(&trace), "{trace:?}");
    // The second and third: neither is. A lever pulled again goes down.
    pull(&mut a[0], &mut seen, 1).await;
    pull(&mut a[0], &mut seen, 3).await;
    let trace = ask_eak(&mut a[1]).await;
    assert!(said(&trace, keys::OPQ_EAK_LEVERS_NONE_RIGHT), "{trace:?}");
    assert!(wrong(&trace), "{trace:?}");
    // The first and fourth.
    for name in [2, 3, 1, 4] {
        pull(&mut a[0], &mut seen, name).await;
    }
    let up: Vec<u8> = (1..=5)
        .map(|name| {
            let oid = lever(&seen, name).object_id;
            let hits = seen.field.reactor_hits.iter().filter(|&&(o, _)| o == oid);
            hits.last().map_or(0, |&(_, state)| state)
        })
        .collect();
    assert_eq!(up, [1, 0, 0, 1, 0]);
    let mut trace = ask_eak(&mut a[1]).await;
    assert!(said(&trace, keys::OPQ_EAK_LEVERS_RIGHT), "{trace:?}");
    until(&mut a[1].connection, &mut trace, cleared).await;
    until(&mut a[0].connection, &mut seen, |t| {
        t.field.reactor_hits.contains(&(stone6.object_id, 1)) && cleared(t)
    })
    .await;
    let trace = ask_eak(&mut a[1]).await;
    assert!(said(&trace, keys::OPQ_EAK_LEVERS_DONE), "{trace:?}");
    assert!(!cleared(&trace), "{trace:?}");
    break_open(&mut a[0], &mut seen, stone6, 4001049).await;
}
