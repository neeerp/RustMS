//! The Helios Tower elevator in Ludibrium: the car docks on each floor in
//! turn, its reactor showing the door open or shut, and the floor's `in00`
//! lets players into the waiting room only while it is there; the waiting
//! room rides up or down when it leaves. The harness world runs the route
//! clock 60 times as fast (`RUSTMS_CONTI_SCALE`), so the car docks for a
//! second on each floor every 4 seconds.

use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{step, until, warp, Trace};
use net::conti::{self, CAR_AWAY, CAR_HERE};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const PINK: u8 = 5;
const CAR: i32 = 2208004;

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

#[tokio::test]
async fn the_elevator_takes_riders_up_and_back_down() {
    let (up, down) = (&conti::ELEVATOR_UP, &conti::ELEVATOR_DOWN);
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;

    // The 2nd floor shows the car.
    let mut trace = Trace::default();
    assert_eq!(warp(c, up.platform, "", &mut trace).await, up.platform);
    until(c, &mut trace, |t| {
        t.field.reactors.iter().any(|r| r.reactor_id == CAR)
    })
    .await;
    let car = trace
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == CAR)
        .unwrap();
    assert!([CAR_HERE, CAR_AWAY].contains(&car.state));
    let car = car.object_id;

    // When it leaves, its door shuts and the way in with it.
    until(c, &mut trace, |t| {
        t.field.reactor_hits.contains(&(car, CAR_AWAY))
    })
    .await;
    let t = step(c, &m, enter("in00")).await;
    assert!(t.warps.is_empty(), "{t:?}");
    assert!(
        matches!(t.broadcasts.as_slice(), [(PINK, text)] if !text.is_empty()),
        "{t:?}"
    );

    // When it comes back, the door opens onto the waiting room...
    let mut trace = Trace::default();
    until(c, &mut trace, |t| {
        t.field.reactor_hits.contains(&(car, CAR_HERE))
    })
    .await;
    let t = step(c, &m, enter("in00")).await;
    assert_eq!(t.portal_sounds, 1, "{t:?}");
    assert_eq!(t.warps, [up.waiting_room]);

    // ...which the car takes up to the 99th floor.
    let mut trace = Trace::default();
    until(c, &mut trace, |t| t.warps.contains(&up.deck)).await;
    until(c, &mut trace, |t| t.warps.contains(&up.arrival.0)).await;
    assert_eq!(trace.arrivals.last().unwrap().0, down.platform);
    assert_eq!(trace.arrivals.last().unwrap().1, 0);

    // It docks there as the riders step off, so the way down is open.
    let t = step(c, &m, enter("in00")).await;
    assert_eq!(t.warps, [down.waiting_room], "{t:?}");
    let mut trace = Trace::default();
    until(c, &mut trace, |t| t.warps.contains(&down.deck)).await;
    until(c, &mut trace, |t| t.warps.contains(&down.arrival.0)).await;
    // The 2nd floor's `st00`.
    assert_eq!(*trace.arrivals.last().unwrap(), (up.platform, 4, None));
    // Its door is open, or opens as this floor's field catches up (and the
    // field is done showing it before the player goes on).
    until(c, &mut trace, |t| {
        t.field.reactors.iter().any(|r| {
            r.reactor_id == CAR
                && (r.state == CAR_HERE || t.field.reactor_hits.contains(&(r.object_id, CAR_HERE)))
        })
    })
    .await;

    // The car waits on the 2nd floor now, so the 99th shows its door shut.
    let mut trace = Trace::default();
    assert_eq!(warp(c, down.platform, "", &mut trace).await, down.platform);
    until(c, &mut trace, |t| {
        t.field.reactors.iter().any(|r| r.reactor_id == CAR)
    })
    .await;
    let car = trace
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == CAR)
        .unwrap();
    assert_eq!(car.state, CAR_AWAY);
}
