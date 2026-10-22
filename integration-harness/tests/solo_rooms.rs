//! The fourth job's solo rooms: El Nath's ways into the Fire Hawk's and Ice
//! Eagle's nests and Ludibrium's into the Tornado room open to a player on
//! their quest, one player at a time; closed, the nests' ways say so in
//! pink and the Tornado room's says nothing.

use integration_harness::database::set_quest;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{step, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const PINK: u8 = 5;
const NEST_WAY: i32 = 211042200;
const FIREHAWK_NEST: i32 = 921100200;
const CLOUDY_BALCONY: i32 = 220010001;

fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// Go to `map` and try its portal `portal`; returns what came back.
async fn try_portal(player: &mut WorldSession, map: i32, portal: &str) -> Trace {
    let mut trace = Trace::default();
    assert_eq!(warp(&mut player.connection, map, "", &mut trace).await, map);
    let name = player.character_name.clone();
    step(&mut player.connection, &name, enter(portal)).await
}

fn arrived(trace: &Trace) -> (i32, u8) {
    assert_eq!(trace.portal_sounds, 1, "{trace:?}");
    let [(map, portal, _)] = trace.arrivals[..] else {
        panic!("{:?}", trace.arrivals);
    };
    (map, portal)
}

fn pink(trace: &Trace) -> bool {
    matches!(trace.broadcasts.as_slice(), [(PINK, text)] if !text.is_empty())
}

#[tokio::test]
async fn the_fire_hawks_nest_takes_one_player_on_the_quest_at_a_time() {
    let config = load_harness_config_or_fail().await;
    let mut player = login_to_world_session(&config).await.unwrap();
    let id = player.character_id;

    // Closed to a new character: the nest in pink, the Tornado room silent.
    let trace = try_portal(&mut player, NEST_WAY, "in00").await;
    assert!(trace.warps.is_empty(), "{:?}", trace.warps);
    assert!(pink(&trace), "{:?}", trace.broadcasts);
    let trace = try_portal(&mut player, CLOUDY_BALCONY, "pt00").await;
    assert!(trace.warps.is_empty(), "{:?}", trace.warps);
    assert!(trace.broadcasts.is_empty(), "{:?}", trace.broadcasts);
    drop(player);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // On the Fire Hawk's quest, but someone is in the nest already.
    set_quest(id, 6240, 1, "").await.unwrap();
    let mut player = relogin(&config).await;
    let mut other = login_to_world_session(&load_harness_config_or_fail().await)
        .await
        .unwrap();
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut other.connection, FIREHAWK_NEST, "", &mut trace).await,
        FIREHAWK_NEST
    );
    let trace = try_portal(&mut player, NEST_WAY, "in00").await;
    assert!(trace.warps.is_empty(), "{:?}", trace.warps);
    assert!(pink(&trace), "{:?}", trace.broadcasts);

    // Once they leave, it's the player's turn, and `out00` leads back.
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut other.connection, NEST_WAY, "", &mut trace).await,
        NEST_WAY
    );
    let trace = try_portal(&mut player, NEST_WAY, "in00").await;
    assert_eq!(arrived(&trace), (FIREHAWK_NEST, 0));
    assert!(trace.broadcasts.is_empty(), "{:?}", trace.broadcasts);
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut player.connection, -1, "out00", &mut trace).await,
        NEST_WAY
    );
}
