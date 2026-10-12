use integration_harness::connection::MapleTestConnection;
use integration_harness::login_to_world_session;
use integration_harness::packets::{
    build_change_map, decode_set_field_warp, opcode_name, SetFieldWarpPacket,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use net::packet::op::SendOpcode;
use tokio::time::{timeout_at, Duration, Instant};

#[tokio::test]
async fn portal_map_transfer_round_trip() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config)
        .await
        .expect("login-to-world session flow failed");

    // Normalize to Henesys so this test can be run repeatedly.
    session
        .connection
        .send_packet(
            build_change_map(100000000, "").expect("failed to build direct change-map packet"),
            "normalize map before portal transfer",
        )
        .await
        .expect("failed to send normalize change-map packet");

    let normalized = read_next_set_field_warp(
        &mut session.connection,
        "normalize map before portal transfer",
    )
    .await;
    assert_eq!(
        normalized.map_id, 100000000,
        "expected normalization warp to map 100000000"
    );

    // in02 (Henesys) -> out02 (Henesys Hair Salon) according to NX/Cosmic data.
    session
        .connection
        .send_packet(
            build_change_map(-1, "in02").expect("failed to build portal change-map packet"),
            "portal transfer in02",
        )
        .await
        .expect("failed to send portal change-map packet");

    let to_salon = read_next_set_field_warp(&mut session.connection, "portal transfer in02").await;
    assert_eq!(
        to_salon.map_id, 100000001,
        "expected in02 to transfer into map 100000001"
    );

    // out02 (Henesys Hair Salon) -> in02 (Henesys).
    session
        .connection
        .send_packet(
            build_change_map(-1, "out02").expect("failed to build portal return change-map packet"),
            "portal transfer out02",
        )
        .await
        .expect("failed to send portal return change-map packet");

    let back_to_henesys =
        read_next_set_field_warp(&mut session.connection, "portal transfer out02").await;
    assert_eq!(
        back_to_henesys.map_id, 100000000,
        "expected out02 to transfer back to map 100000000"
    );
}

#[tokio::test]
async fn beginner_route_and_ninas_house_follow_asset_links() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config).await.unwrap();
    let maps = game_data::GameData::load_from_nx_map(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx"),
    )
    .unwrap();
    session
        .connection
        .send_packet(build_change_map(10_000, "").unwrap(), "beginner route")
        .await
        .unwrap();
    assert_eq!(
        read_next_set_field_warp(&mut session.connection, "beginner route")
            .await
            .map_id,
        10_000
    );
    let mut source = 10_000;
    // Tutorial exits are intentionally one-way. The legacy side maps in the
    // archive are not a reason to invent reverse links or insert extra maps.
    for (name, destination) in [
        ("out00", 20_000),
        ("out00", 30_000),
        ("in01", 30_001),
        ("out00", 30_000),
        ("out00", 40_000),
        ("out00", 50_000),
        ("east00", 1_000_000),
        ("east00", 1_010_000),
        ("east00", 1_020_000),
        ("east00", 2_000_000),
        ("west00", 1_020_000),
        ("west00", 1_010_000),
        ("west00", 1_000_000),
        ("west00", 50_000),
    ] {
        let link = maps.field(source).unwrap().portal_by_name(name).unwrap();
        assert_eq!(link.to_map, Some(destination));
        let arrival = maps
            .field(destination)
            .unwrap()
            .portal_by_name(&link.to_name)
            .unwrap();
        session
            .connection
            .send_packet(build_change_map(-1, name).unwrap(), "beginner route portal")
            .await
            .unwrap();
        let warp = read_next_set_field_warp(&mut session.connection, "beginner route portal").await;
        assert_eq!(
            (warp.map_id, u32::from(warp.spawn_point)),
            (destination, arrival.id)
        );
        source = destination;
    }
}

async fn read_next_set_field_warp(
    connection: &mut MapleTestConnection,
    phase: &'static str,
) -> SetFieldWarpPacket {
    // Bound the wait by time, not packet count: entering a busy map (Henesys
    // has 30 NPCs) leaves enough spawn packets queued to fill any small count
    // before the next warp arrives.
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        let envelope = timeout_at(deadline, connection.read_packet(phase))
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
            x if x == SendOpcode::SpawnPlayer as i16
                || x == SendOpcode::RemovePlayerFromMap as i16 =>
            {
                continue
            }
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
