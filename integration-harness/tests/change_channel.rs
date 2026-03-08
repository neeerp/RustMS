use integration_harness::connection::MapleTestConnection;
use integration_harness::packets::{
    build_change_channel, build_player_logged_in_for_channel, decode_channel_change,
    decode_spawn_player, opcode_name,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::{login_to_world_session, HarnessError};
use net::packet::op::SendOpcode;
use tokio::time::{timeout, Duration};

async fn read_world_entry(
    connection: &mut MapleTestConnection,
    character_id: i32,
    channel_id: u8,
) -> Result<(), HarnessError> {
    connection
        .send_packet(
            build_player_logged_in_for_channel(character_id, channel_id)
                .map_err(|message| HarnessError::fixture("world entry", message))?,
            "world entry",
        )
        .await?;

    let mut saw_keymap = false;
    let mut saw_set_field = false;
    for _ in 0..8 {
        let envelope = connection.read_packet("world entry").await?;
        match envelope.opcode() {
            x if x == SendOpcode::KeyMap as i16 => saw_keymap = true,
            x if x == SendOpcode::SetField as i16 => saw_set_field = true,
            x if x == SendOpcode::SpawnNpc as i16 => {}
            opcode => {
                return Err(HarnessError::protocol(
                    "world entry",
                    connection.endpoint(),
                    format!(
                        "unexpected opcode {} ({}) during world entry",
                        opcode,
                        opcode_name(opcode)
                    ),
                ));
            }
        }

        if saw_keymap && saw_set_field {
            return Ok(());
        }
    }

    Err(HarnessError::protocol(
        "world entry",
        connection.endpoint(),
        "did not receive KeyMap and SetField during world entry".to_string(),
    ))
}

async fn read_until_opcode(
    connection: &mut MapleTestConnection,
    phase: &'static str,
    expected_opcode: i16,
) -> Result<integration_harness::connection::PacketEnvelope, HarnessError> {
    for _ in 0..12 {
        let envelope = connection.read_packet(phase).await?;
        match envelope.opcode() {
            opcode if opcode == expected_opcode => return Ok(envelope),
            x if x == SendOpcode::SpawnNpc as i16 => {}
            x if x == SendOpcode::SpawnPlayer as i16 => {}
            x if x == SendOpcode::RemovePlayerFromMap as i16 => {}
            opcode => {
                return Err(HarnessError::protocol(
                    phase,
                    connection.endpoint(),
                    format!(
                        "unexpected opcode {} ({}) while waiting for {} ({})",
                        opcode,
                        opcode_name(opcode),
                        expected_opcode,
                        opcode_name(expected_opcode)
                    ),
                ));
            }
        }
    }

    Err(HarnessError::protocol(
        phase,
        connection.endpoint(),
        format!(
            "did not receive expected opcode {} ({})",
            expected_opcode,
            opcode_name(expected_opcode)
        ),
    ))
}

#[tokio::test]
async fn change_channel_redirects_and_updates_presence() {
    let mut old_channel_observer = load_harness_config_or_fail().await.with_channel(0, 0);
    let mut migrant = load_harness_config_or_fail().await.with_channel(0, 0);
    let mut new_channel_observer = load_harness_config_or_fail().await.with_channel(0, 1);

    old_channel_observer.expected_redirect_addr = old_channel_observer.world_addr;
    migrant.expected_redirect_addr = migrant.world_addr;
    new_channel_observer.expected_redirect_addr = new_channel_observer.world_addr;

    let mut old_channel_observer = login_to_world_session(&old_channel_observer)
        .await
        .expect("old-channel observer failed to login");
    let migrant = login_to_world_session(&migrant)
        .await
        .expect("migrant failed to login");
    let mut new_channel_observer = login_to_world_session(&new_channel_observer)
        .await
        .expect("new-channel observer failed to login");

    let old_presence = timeout(
        Duration::from_secs(5),
        old_channel_observer
            .connection
            .read_packet("old-channel observer initial presence"),
    )
    .await
    .expect("timed out waiting for initial same-channel presence")
    .expect("failed to read initial same-channel presence");
    let old_spawn = decode_spawn_player(&old_presence.packet)
        .expect("failed to decode initial same-channel presence");
    assert_eq!(old_spawn.character_id, migrant.character_id);

    let mut migrant_connection = migrant.connection;
    migrant_connection
        .send_packet(
            build_change_channel(1).expect("build change-channel request"),
            "change channel",
        )
        .await
        .expect("failed to send change-channel request");

    let redirect_packet = timeout(
        Duration::from_secs(5),
        read_until_opcode(
            &mut migrant_connection,
            "change channel redirect",
            SendOpcode::ChangeChannel as i16,
        ),
    )
    .await
    .expect("timed out waiting for change-channel redirect")
    .expect("failed to read change-channel redirect");
    let redirect = decode_channel_change(&redirect_packet.packet)
        .expect("failed to decode change-channel redirect");

    let leave_packet = timeout(
        Duration::from_secs(5),
        read_until_opcode(
            &mut old_channel_observer.connection,
            "old-channel leave after migration",
            SendOpcode::RemovePlayerFromMap as i16,
        ),
    )
    .await
    .expect("timed out waiting for old-channel leave")
    .expect("failed to read old-channel leave");
    assert_eq!(leave_packet.opcode(), SendOpcode::RemovePlayerFromMap as i16);

    let mut reconnected = MapleTestConnection::connect(
        format!("{}:{}", redirect.ip, redirect.port)
            .parse()
            .expect("valid redirect socket address"),
        "change-channel reconnect handshake",
    )
    .await
    .expect("failed to reconnect to redirected world address");
    read_world_entry(&mut reconnected, migrant.character_id, 1)
        .await
        .expect("failed to reattach migrated player");

    let new_presence = timeout(
        Duration::from_secs(5),
        read_until_opcode(
            &mut new_channel_observer.connection,
            "new-channel join after migration",
            SendOpcode::SpawnPlayer as i16,
        ),
    )
    .await
    .expect("timed out waiting for new-channel join")
    .expect("failed to read new-channel join");
    let new_spawn = decode_spawn_player(&new_presence.packet)
        .expect("failed to decode new-channel join");
    assert_eq!(new_spawn.character_id, migrant.character_id);
}
