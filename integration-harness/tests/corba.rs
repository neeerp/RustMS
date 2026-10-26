//! Corba (report a014): at Leafre's station he turns the player into a
//! little dragon (2210016's morph) and flies them toward the Temple of
//! Time. Touching the ground below the flight ends the morph and drops
//! them back at the station; the temple's gate ends it too, and leaving
//! the temple's road puts it back on for the way home.

use integration_harness::events::talk;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const CORBA: i32 = 2082003;
const MINI_DRACO: i32 = 2210016;
const LEAFRE_STATION: i32 = 240000110;
const FLIGHT_TO_TEMPLE: i32 = 200090500;
const FLIGHT_TO_MINAR: i32 = 200090510;
const TEMPLE_ROAD: i32 = 270000100;
/// Morph's flag (bit 33), in the second half of a buff mask.
const MORPH: i64 = 2;

/// CHANGE_MAP_SPECIAL: walk into the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

fn morphed(trace: &Trace) -> bool {
    trace.buffs.iter().any(|&(item, _, _)| item == -MINI_DRACO)
}

fn arrived(trace: &Trace) -> (i32, u8) {
    let [(map, portal, _)] = trace.arrivals[..] else {
        panic!("{:?}", trace.arrivals);
    };
    (map, portal)
}

#[tokio::test]
async fn corba_flies_a_dragon_and_the_ground_ends_it() {
    let config = load_harness_config_or_fail().await;
    login_to_world_session(&config)
        .await
        .expect("first login failed");
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut player = relogin(&config).await;
    let name = player.character_name.clone();
    let c = &mut player.connection;

    // Corba's one choice: become a dragon and fly off.
    let mut t = Trace::default();
    assert_eq!(warp(c, LEAFRE_STATION, "", &mut t).await, LEAFRE_STATION);
    step(c, &name, talk(LEAFRE_STATION, CORBA)).await;
    let t = step(c, &name, answer(4, 1, Some(0))).await;
    assert!(morphed(&t), "{:?}", t.buffs);
    assert_eq!(arrived(&t), (FLIGHT_TO_TEMPLE, 0));

    // The ground below: back to the station, a dragon no more.
    let t = step(c, &name, enter("down05")).await;
    assert_eq!(t.cancelled_buffs, [MORPH]);
    assert_eq!(arrived(&t), (LEAFRE_STATION, 2), "at po00");

    // Leaving the temple's road makes a dragon for the flight home ...
    let mut t = Trace::default();
    assert_eq!(warp(c, TEMPLE_ROAD, "", &mut t).await, TEMPLE_ROAD);
    let t = step(c, &name, enter("out00")).await;
    assert!(morphed(&t), "{:?}", t.buffs);
    assert_eq!(arrived(&t), (FLIGHT_TO_MINAR, 0));

    // ... and the temple's gate takes it off.
    let t = step(c, &name, enter("in00")).await;
    assert_eq!(t.cancelled_buffs, [MORPH]);
    assert_eq!(arrived(&t), (TEMPLE_ROAD, 2), "at out00");
}
