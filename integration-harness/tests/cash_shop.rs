use integration_harness::connection::{MapleTestConnection, PacketEnvelope};
use integration_harness::packets::{
    build_all_chat, build_enter_cash_shop, build_leave_cash_shop, build_player_logged_in,
    decode_channel_change, decode_set_field, decode_spawn_player, opcode_name,
};
use integration_harness::preconditions::load_multi_harness_config_or_fail;
use integration_harness::{login_two_players_to_world, HarnessError};
use net::packet::op::SendOpcode;
use packet::io::read::PktRead;
use std::io::Cursor;
use tokio::time::{timeout, Duration};

/// Read until `expected`, skipping the packets of an ordinary map.
async fn read_until(
    connection: &mut MapleTestConnection,
    phase: &'static str,
    expected: SendOpcode,
) -> Result<PacketEnvelope, HarnessError> {
    let expected = expected as i16;
    for _ in 0..24 {
        let envelope = timeout(Duration::from_secs(5), connection.read_packet(phase))
            .await
            .map_err(|_| {
                HarnessError::protocol(phase, connection.endpoint(), "timed out".to_string())
            })??;
        if envelope.opcode() == expected {
            return Ok(envelope);
        }
        let skippable = [
            SendOpcode::SpawnNpc as i16,
            SendOpcode::SpawnNpcRequestController as i16,
            SendOpcode::FieldEffect as i16,
            SendOpcode::SpawnPlayer as i16,
            SendOpcode::RemovePlayerFromMap as i16,
            SendOpcode::SpawnMonster as i16,
            SendOpcode::SpawnMonsterControl as i16,
            SendOpcode::MoveMonster as i16,
            SendOpcode::StatChange as i16,
            SendOpcode::KeyMap as i16,
            SendOpcode::MacroSysDataInit as i16,
            SendOpcode::AutoHpPot as i16,
            SendOpcode::AutoMpPot as i16,
        ];
        if !skippable.contains(&envelope.opcode()) {
            return Err(HarnessError::protocol(
                phase,
                connection.endpoint(),
                format!(
                    "unexpected opcode {} ({}) while waiting for {} ({})",
                    envelope.opcode(),
                    opcode_name(envelope.opcode()),
                    expected,
                    opcode_name(expected)
                ),
            ));
        }
    }
    Err(HarnessError::protocol(
        phase,
        connection.endpoint(),
        format!("never received {} ({})", expected, opcode_name(expected)),
    ))
}

fn removed_character(envelope: &PacketEnvelope) -> i32 {
    let mut reader = Cursor::new(&envelope.packet.bytes);
    reader.read_short().expect("opcode");
    reader.read_int().expect("removed character id")
}

#[tokio::test]
async fn the_cash_shop_takes_the_player_off_the_map_and_back() {
    let config = load_multi_harness_config_or_fail().await;
    let (shopper, mut observer) = login_two_players_to_world(&config)
        .await
        .expect("two-player login failed");
    let mut shopper_connection = shopper.connection;
    let spawn = read_until(
        &mut observer.connection,
        "shopper on the map",
        SendOpcode::SpawnPlayer,
    )
    .await
    .expect("observer never saw the shopper");
    assert_eq!(
        decode_spawn_player(&spawn.packet).unwrap().character_id,
        shopper.character_id
    );

    shopper_connection
        .send_packet(build_enter_cash_shop(), "enter cash shop")
        .await
        .expect("send ENTER_CASHSHOP");
    read_until(
        &mut shopper_connection,
        "enter cash shop",
        SendOpcode::SetCashShop,
    )
    .await
    .expect("no SET_CASH_SHOP");
    for _ in 0..3 {
        read_until(
            &mut shopper_connection,
            "cash shop lists",
            SendOpcode::CashShopOperation,
        )
        .await
        .expect("no cash inventory, gifts or wish list");
    }
    read_until(
        &mut shopper_connection,
        "cash balance",
        SendOpcode::QueryCashResult,
    )
    .await
    .expect("no NX balance");

    let removed = read_until(
        &mut observer.connection,
        "shopper leaves the map",
        SendOpcode::RemovePlayerFromMap,
    )
    .await
    .expect("observer never saw the shopper leave");
    assert_eq!(removed_character(&removed), shopper.character_id);

    // Off the map, the shopper's chat reaches no one.
    shopper_connection
        .send_packet(build_all_chat("from the shop", 1).unwrap(), "chat")
        .await
        .expect("send chat");
    let quiet_until = tokio::time::Instant::now() + Duration::from_millis(500);
    while let Ok(packet) = tokio::time::timeout_at(
        quiet_until,
        observer.connection.read_packet("no chat from the shop"),
    )
    .await
    {
        let packet = packet.expect("observer connection");
        assert_ne!(
            packet.opcode(),
            SendOpcode::ChatText as i16,
            "the observer heard from a player in the Cash Shop"
        );
    }

    shopper_connection
        .send_packet(build_leave_cash_shop(), "leave cash shop")
        .await
        .expect("send empty CHANGE_MAP");
    let redirect = read_until(
        &mut shopper_connection,
        "leave cash shop",
        SendOpcode::ChangeChannel,
    )
    .await
    .expect("no redirect out of the Cash Shop");
    let redirect = decode_channel_change(&redirect.packet).unwrap();
    let world_port: u16 = shopper
        .world_addr
        .rsplit(':')
        .next()
        .and_then(|port| port.parse().ok())
        .expect("world port");
    assert_eq!(
        redirect.port, world_port,
        "left the Cash Shop to another channel"
    );

    let mut returned = MapleTestConnection::connect(
        format!("{}:{}", redirect.ip, redirect.port)
            .parse()
            .expect("redirect address"),
        "return from the cash shop",
    )
    .await
    .expect("reconnect after the Cash Shop");
    returned
        .send_packet(
            build_player_logged_in(shopper.character_id).unwrap(),
            "re-enter the world",
        )
        .await
        .expect("send PLAYER_LOGGEDIN");
    let set_field = read_until(&mut returned, "re-enter the world", SendOpcode::SetField)
        .await
        .expect("no SET_FIELD after the Cash Shop");
    assert_eq!(
        decode_set_field(&set_field.packet).unwrap().map_id,
        shopper.map_id
    );

    let back = read_until(
        &mut observer.connection,
        "shopper back on the map",
        SendOpcode::SpawnPlayer,
    )
    .await
    .expect("observer never saw the shopper return");
    assert_eq!(
        decode_spawn_player(&back.packet).unwrap().character_id,
        shopper.character_id
    );
}
