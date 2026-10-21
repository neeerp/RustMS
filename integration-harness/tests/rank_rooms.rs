//! Portals that open with the portal sound and no condition: the job halls'
//! rooms (`rankRoom`), and Kerning City's subway tunnels from the ticketing
//! booth (`subway_in2`) and the construction site (`met_in`).

use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{step, warp, Trace};
use integration_harness::{login_to_world_session, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// Go to `map`, take its portal `portal`, and say where it led.
async fn through(player: &mut WorldSession, map: i32, portal: &str) -> (i32, u8) {
    let mut trace = Trace::default();
    assert_eq!(warp(&mut player.connection, map, "", &mut trace).await, map);
    let name = player.character_name.clone();
    let trace = step(&mut player.connection, &name, enter(portal)).await;
    assert_eq!(trace.portal_sounds, 1, "{trace:?}");
    let [(map, portal, _)] = trace.arrivals[..] else {
        panic!("{:?}", trace.arrivals);
    };
    (map, portal)
}

#[tokio::test]
async fn halls_rooms_and_subway_tunnels_open_to_anyone() {
    let config = load_harness_config_or_fail().await;
    let mut player = login_to_world_session(&config).await.unwrap();

    assert_eq!(
        through(&mut player, 100000201, "rank00").await,
        (100000204, 2)
    );
    assert_eq!(
        through(&mut player, 130000000, "west00").await,
        (130000100, 5)
    );
    assert_eq!(
        through(&mut player, 103000100, "in00").await,
        (103000101, 3)
    );
    assert_eq!(
        through(&mut player, 102040000, "subway00").await,
        (103000103, 1)
    );
}
