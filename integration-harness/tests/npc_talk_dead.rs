//! A dead player's click on an NPC: the v83 client sends the talk (its
//! click path doesn't check and the death window isn't modal), and the
//! server only unlocks the client, as Cosmic's NPCTalkHandler does.

use game_data::GameData;
use integration_harness::database::set_hp;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{stat, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::collections::BTreeMap;
use std::path::Path;
use tokio::time::Duration;

const SOUTHPERRY: i32 = 60_000;
const SHANKS: i32 = 22000;
const STUMPY: i32 = 3_220_000;

fn talk(map: i32, npc: i32) -> Packet {
    let field = GameData::load_from_nx_map(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/game-data/Map.nx"),
    )
    .unwrap();
    let index = field
        .field(map)
        .unwrap()
        .map_npcs
        .iter()
        .position(|n| n.npc_id == npc)
        .unwrap();
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TalkToNpc as i16).unwrap();
    p.write_int(1_000_000_000 + index as i32).unwrap();
    p
}

/// TAKE_DAMAGE for a mob's touch.
fn touched(damage: i32, mob_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TakeDamage as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(-1i8 as u8).unwrap();
    p.write_byte(0).unwrap();
    p.write_int(damage).unwrap();
    p.write_int(mob_id).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(0).unwrap();
    p
}

#[tokio::test]
async fn a_dead_player_cannot_talk_to_an_npc() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_hp(id, 10, 50).await.expect("low HP");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, SOUTHPERRY, "", &mut t).await, SOUTHPERRY);
    let shanks = talk(SOUTHPERRY, SHANKS);

    // Alive, Shanks talks.
    let t = step(c, &m, shanks.clone()).await;
    assert!(!t.prompts.is_empty(), "Shanks talks to the living");

    let t = step(c, &m, touched(20, STUMPY)).await;
    assert_eq!(t.stat_values(stat::HP), [0]);

    // Dead, he doesn't: the client is only unlocked.
    let t = step(c, &m, shanks).await;
    assert!(t.prompts.is_empty(), "no conversation for the dead");
    assert_eq!(t.stat_updates, [BTreeMap::new()]);
}
