//! Items marked `expireOnLogout` go when their owner logs out, but not when
//! they change channels.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::grant_item;
use integration_harness::packets::{
    build_change_channel, build_player_logged_in, decode_channel_change, decode_set_field,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::SendOpcode;
use tokio::time::Duration;

const ETC: i16 = 4;
/// Gone at logout, and not one of a kind.
const LOGOUT_ITEM: i32 = 4_001_333;
const KEPT: i32 = 4_000_000;

#[tokio::test]
async fn logout_items_survive_a_channel_change_but_not_a_logout() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_item(id, ETC, 1, LOGOUT_ITEM)
        .await
        .expect("a logout item");
    grant_item(id, ETC, 2, KEPT).await.expect("a snail shell");

    let session = relogin(&config).await;
    assert_eq!(session.items.get(&LOGOUT_ITEM), Some(&1));
    let mut c = session.connection;
    let mut trace = Trace::default();
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
    send(&mut c, build_player_logged_in(id).unwrap()).await;
    let set_field = loop {
        let packet = read(&mut c, &mut trace).await;
        if packet.opcode() == SendOpcode::SetField as i16 {
            break decode_set_field(&packet).unwrap();
        }
    };
    assert_eq!(set_field.items.get(&LOGOUT_ITEM), Some(&1));
    drop(c);

    let session = relogin(&config).await;
    assert!(!session.items.contains_key(&LOGOUT_ITEM));
    assert_eq!(session.items.get(&KEPT), Some(&1));
}
