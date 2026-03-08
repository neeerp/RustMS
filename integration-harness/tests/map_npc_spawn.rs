use game_data::GameData;
use integration_harness::connection::MapleTestConnection;
use integration_harness::login_to_world_session;
use integration_harness::packets::{
    build_change_map, decode_set_field_warp, decode_spawn_npc, opcode_name, SetFieldWarpPacket,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use net::packet::op::SendOpcode;
use std::path::PathBuf;
use tokio::time::{timeout, Duration};

const NPC_TEST_MAP_ID: i32 = 10_000;
const MAP_NPC_OBJECT_ID_BASE: i32 = 1_000_000_000;

#[tokio::test]
async fn direct_warp_replays_static_map_npcs() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config)
        .await
        .expect("login-to-world session flow failed");

    session
        .connection
        .send_packet(
            build_change_map(NPC_TEST_MAP_ID, "").expect("failed to build npc map warp packet"),
            "warp to npc test map",
        )
        .await
        .expect("failed to send npc map warp packet");

    let warp = read_next_set_field_warp(&mut session.connection, "warp to npc test map").await;
    assert_eq!(
        warp.map_id, NPC_TEST_MAP_ID,
        "expected warp to npc test map"
    );
    read_next_stat_change(&mut session.connection, "warp to npc test map").await;

    let expected_npcs = expected_map_npcs(NPC_TEST_MAP_ID);
    assert!(
        !expected_npcs.is_empty(),
        "expected test map {NPC_TEST_MAP_ID} to contain NPCs"
    );

    for (index, expected_npc) in expected_npcs.iter().enumerate() {
        let envelope = timeout(
            Duration::from_secs(5),
            session.connection.read_packet("npc replay after warp"),
        )
        .await
        .expect("timed out waiting for npc spawn packet")
        .expect("failed to read npc spawn packet");

        let spawn = decode_spawn_npc(&envelope.packet).unwrap_or_else(|message| {
            panic!(
                "failed to decode npc spawn packet {} ({}): {}",
                envelope.opcode(),
                opcode_name(envelope.opcode()),
                message
            )
        });

        assert_eq!(spawn.object_id, MAP_NPC_OBJECT_ID_BASE + index as i32);
        assert_eq!(spawn.npc_id, expected_npc.npc_id);
        assert_eq!(spawn.x, expected_npc.x);
        assert_eq!(spawn.y, expected_npc.y);
        assert_eq!(spawn.flip, expected_npc.flip);
        assert_eq!(spawn.foothold, expected_npc.foothold);
        assert_eq!(spawn.rx0, expected_npc.rx0);
        assert_eq!(spawn.rx1, expected_npc.rx1);
    }
}

fn expected_map_npcs(map_id: i32) -> Vec<game_data::MapNpcTemplate> {
    let path = std::env::var("RUSTMS_MAP_NX_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("integration-harness should live under workspace root")
                .join("assets/game-data/Map.nx")
        });
    let game_data = GameData::load_from_nx_map(&path).unwrap_or_else(|error| {
        panic!(
            "failed to load game data from '{}': {error}",
            path.display()
        )
    });

    game_data
        .field(map_id)
        .unwrap_or_else(|| panic!("missing test field {map_id}"))
        .map_npcs
        .clone()
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
