use game_data::GameData;
use integration_harness::connection::MapleTestConnection;
use integration_harness::login_two_players_to_world;
use integration_harness::packets::{
    build_change_map, decode_set_field_warp, decode_spawn_npc, decode_spawn_player, opcode_name,
    SetFieldWarpPacket,
};
use integration_harness::preconditions::load_multi_harness_config_or_fail;
use net::packet::op::SendOpcode;
use std::path::PathBuf;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn portal_transfer_spawns_at_destination_portal_for_observers() {
    let config = load_multi_harness_config_or_fail().await;
    let (mut mover, mut observer) = login_two_players_to_world(&config)
        .await
        .expect("two-player login-to-world flow failed");

    let _ = timeout(
        Duration::from_secs(5),
        read_spawn_player(&mut mover.connection, "mover initial presence"),
    )
    .await
    .expect("timed out waiting for mover initial presence");
    let _ = timeout(
        Duration::from_secs(5),
        read_spawn_player(&mut observer.connection, "observer initial presence"),
    )
    .await
    .expect("timed out waiting for observer initial presence");

    mover
        .connection
        .send_packet(
            build_change_map(100000000, "").expect("failed to build mover normalize warp"),
            "mover normalize to Henesys",
        )
        .await
        .expect("failed to send mover normalize warp");
    let mover_normalized =
        read_next_set_field_warp(&mut mover.connection, "mover normalize to Henesys").await;
    assert_eq!(
        mover_normalized.map_id, 100000000,
        "expected mover to normalize to map 100000000"
    );
    read_next_stat_change(&mut mover.connection, "mover normalize to Henesys").await;
    let mover_leave = timeout(
        Duration::from_secs(5),
        observer.connection.read_packet("observer sees mover leave initial map"),
    )
    .await
    .expect("timed out waiting for mover leave packet")
    .expect("failed to read mover leave packet");
    assert_eq!(
        mover_leave.opcode(),
        SendOpcode::RemovePlayerFromMap as i16,
        "expected observer to see mover leave the initial map"
    );

    observer
        .connection
        .send_packet(
            build_change_map(100000001, "").expect("failed to build observer direct warp"),
            "observer direct warp to destination map",
        )
        .await
        .expect("failed to send observer direct warp");
    let observer_warp = read_next_set_field_warp(
        &mut observer.connection,
        "observer direct warp to destination map",
    )
    .await;
    assert_eq!(
        observer_warp.map_id, 100000001,
        "expected observer to warp to map 100000001"
    );
    read_next_stat_change(&mut observer.connection, "observer direct warp to destination map").await;

    let destination_portal = destination_portal(100000000, "in02");

    mover
        .connection
        .send_packet(
            build_change_map(-1, "in02").expect("failed to build mover portal transfer"),
            "mover portal transfer in02",
        )
        .await
        .expect("failed to send mover portal transfer");

    let mover_warp = read_next_set_field_warp(&mut mover.connection, "mover portal transfer in02").await;
    assert_eq!(
        mover_warp.map_id, 100000001,
        "expected mover to warp to map 100000001"
    );
    assert_eq!(
        mover_warp.spawn_point,
        destination_portal.0,
        "expected mover warp to use destination portal id"
    );

    let observer_spawn = timeout(
        Duration::from_secs(5),
        read_spawn_player(&mut observer.connection, "observer sees mover portal spawn"),
    )
    .await
    .expect("timed out waiting for observer portal spawn");
    assert_eq!(observer_spawn.character_id, mover.character_id);
    assert_eq!(observer_spawn.character_name, mover.character_name);
    assert_eq!(observer_spawn.x, destination_portal.1);
    assert_eq!(observer_spawn.y, destination_portal.2);
    assert_eq!(observer_spawn.stance, 2);
}

fn destination_portal(source_map_id: i32, source_portal_name: &str) -> (u8, i16, i16) {
    let path = std::env::var("RUSTMS_MAP_NX_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("integration-harness should live under workspace root")
                .join("assets/game-data/Map.nx")
        });
    let game_data = GameData::load_from_nx_map(&path)
        .unwrap_or_else(|error| panic!("failed to load game data from '{}': {error}", path.display()));
    let source_field = game_data
        .field(source_map_id)
        .unwrap_or_else(|| panic!("missing source field {source_map_id}"));
    let source_portal = source_field
        .portal_by_name(source_portal_name)
        .unwrap_or_else(|| panic!("missing source portal {source_portal_name}"));
    let destination_map_id = source_portal
        .to_map
        .unwrap_or_else(|| panic!("portal {source_portal_name} has no destination map"));
    let destination_field = game_data
        .field(destination_map_id)
        .unwrap_or_else(|| panic!("missing destination field {destination_map_id}"));
    let destination_portal = destination_field
        .resolve_spawn_portal(source_portal.to_name.as_str())
        .unwrap_or_else(|| panic!("missing destination portal {}", source_portal.to_name));

    (
        u8::try_from(destination_portal.id).expect("destination portal id out of range"),
        i16::try_from(destination_portal.x).expect("destination portal x out of range"),
        i16::try_from(destination_portal.y).expect("destination portal y out of range"),
    )
}

async fn read_next_set_field_warp(
    connection: &mut MapleTestConnection,
    phase: &'static str,
) -> SetFieldWarpPacket {
    for _ in 0..32 {
        let envelope = timeout(Duration::from_secs(5), connection.read_packet(phase))
            .await
            .expect("timed out waiting for map-transfer packet")
            .expect("failed to read map-transfer packet");

        match envelope.opcode() {
            x if x == SendOpcode::SetField as i16 => {
                return decode_set_field_warp(&envelope.packet)
                    .expect("failed to decode set-field warp packet");
            }
            x if x == SendOpcode::StatChange as i16 => continue,
            x if x == SendOpcode::SpawnNpc as i16 => continue,
            opcode => {
                panic!(
                    "unexpected opcode {} ({}) during map transfer",
                    opcode,
                    opcode_name(opcode)
                );
            }
        }
    }

    panic!("did not receive SetField packet during map transfer");
}

async fn read_next_stat_change(connection: &mut MapleTestConnection, phase: &'static str) {
    let envelope = timeout(Duration::from_secs(5), connection.read_packet(phase))
        .await
        .expect("timed out waiting for stat-change packet")
        .expect("failed to read stat-change packet");
    assert_eq!(
        envelope.opcode(),
        SendOpcode::StatChange as i16,
        "expected stat-change packet after map transfer"
    );
}

async fn read_spawn_player(
    connection: &mut MapleTestConnection,
    phase: &'static str,
) -> integration_harness::packets::SpawnPlayerPacket {
    loop {
        let envelope = timeout(Duration::from_secs(5), connection.read_packet(phase))
            .await
            .expect("timed out waiting for spawn-player packet")
            .expect("failed to read spawn-player packet");

        if let Ok(spawn) = decode_spawn_player(&envelope.packet) {
            return spawn;
        }

        let _ = decode_spawn_npc(&envelope.packet);
    }
}
