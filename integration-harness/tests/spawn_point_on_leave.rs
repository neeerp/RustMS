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

/// An unchanged training map with several distinct start points.
const TRAINING_ENTRANCE: i32 = 1_010_100;
/// The leftmost start point, at (213, -763).
const LEFT_SPAWN: u8 = 3;
/// The rightmost start point, at (353, -774).
const RIGHT_SPAWN: u8 = 2;

#[tokio::test]
async fn leaving_the_game_saves_the_nearest_spawn_point() {
    let config = load_harness_config_or_fail().await;
    let session = login_to_world_session(&config).await.unwrap();
    let character_id = session.character_id;
    let mut c = session.connection;
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut c, TRAINING_ENTRANCE, "", &mut trace).await,
        TRAINING_ENTRANCE
    );

    send(&mut c, walk_to(213, -763)).await;
    barrier(&mut c, "left spawn", &mut trace).await;
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
    assert_eq!(set_field.map_id, TRAINING_ENTRANCE);
    assert_eq!(set_field.spawn_point, LEFT_SPAWN);

    send(&mut c, walk_to(353, -774)).await;
    barrier(&mut c, "right spawn", &mut trace).await;
    drop(c);

    let session = relogin(&config).await;
    assert_eq!(session.map_id, TRAINING_ENTRANCE);
    assert_eq!(session.spawn_point, RIGHT_SPAWN);
}
