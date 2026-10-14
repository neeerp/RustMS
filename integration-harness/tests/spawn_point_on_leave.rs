//! Leaving the game saves the spawn point nearest where the player stood, so
//! a channel change or relogin puts them back there (issue #217).

use integration_harness::connection::MapleTestConnection;
use integration_harness::hunting::walk_to;
use integration_harness::packets::{
    build_change_channel, build_player_logged_in, decode_channel_change, decode_set_field,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{barrier, read, send, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::SendOpcode;

/// Amherst: several start points, entered through portal 0 at (-288, 27).
const AMHERST: i32 = 1_010_000;
/// The start point under Bari, at (257, 132).
const UNDER_BARI: u8 = 3;
/// The start point at the top left, at (-349, -45).
const TOP_LEFT: u8 = 2;

#[tokio::test]
async fn leaving_the_game_saves_the_nearest_spawn_point() {
    let config = load_harness_config_or_fail().await;
    let session = login_to_world_session(&config).await.unwrap();
    let character_id = session.character_id;
    let mut c = session.connection;
    let mut trace = Trace::default();
    assert_eq!(warp(&mut c, AMHERST, "", &mut trace).await, AMHERST);

    send(&mut c, walk_to(250, 132)).await;
    barrier(&mut c, "under bari", &mut trace).await;
    send(&mut c, build_change_channel(1).unwrap()).await;
    let redirect = loop {
        let packet = read(&mut c, &mut trace).await;
        if packet.opcode() == SendOpcode::ChangeChannel as i16 {
            break decode_channel_change(&packet).unwrap();
        }
    };

    let mut c = MapleTestConnection::connect(
        format!("{}:{}", redirect.ip, redirect.port)
            .parse()
            .unwrap(),
        "change-channel reconnect",
    )
    .await
    .unwrap();
    send(&mut c, build_player_logged_in(character_id).unwrap()).await;
    let set_field = loop {
        let packet = read(&mut c, &mut trace).await;
        if packet.opcode() == SendOpcode::SetField as i16 {
            break decode_set_field(&packet).unwrap();
        }
    };
    assert_eq!(set_field.map_id, AMHERST);
    assert_eq!(set_field.spawn_point, UNDER_BARI);

    send(&mut c, walk_to(-340, -45)).await;
    barrier(&mut c, "top left", &mut trace).await;
    drop(c);

    let session = relogin(&config).await;
    assert_eq!(session.map_id, AMHERST);
    assert_eq!(session.spawn_point, TOP_LEFT);
}
