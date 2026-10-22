//! Super Dragon Roar (9001001 level 1: ten times a blow, up to 15 mobs):
//! a GM who has it kills a group of snails with one swing, and a player
//! who is not a GM can't use it even with the skill.

use integration_harness::database::{grant_skill, set_gm_level};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

/// A snail map no other test hunts on.
const FIELD: i32 = 1_000_003;
const SNAIL: i32 = 100_100;
const SUPER_DRAGON_ROAR: i32 = 9_001_001;

/// CLOSE_RANGE_ATTACK with Super Dragon Roar, hitting each mob once for
/// `damage`.
fn roar(mobs: &[i32], damage: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::CloseRangeAttack as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_byte((mobs.len() as u8) << 4 | 1).unwrap();
    p.write_int(SUPER_DRAGON_ROAR).unwrap();
    p.write_bytes(&[0; 8]).unwrap();
    p.write_byte(0).unwrap();
    p.write_short(0x8005u16 as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_byte(4).unwrap();
    p.write_bytes(&[0; 4]).unwrap();
    for &mob in mobs {
        p.write_int(mob).unwrap();
        p.write_bytes(&[0; 14]).unwrap();
        p.write_int(damage).unwrap();
        p.write_bytes(&[0; 4]).unwrap();
    }
    p
}

#[tokio::test]
async fn a_gms_super_dragon_roar_kills_a_group_and_no_one_else_uses_it() {
    let gm_config = load_harness_config_or_fail().await;
    let gm_id = login_to_world_session(&gm_config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&gm_config).await);
    let other_config = load_harness_config_or_fail().await;
    let other_id = login_to_world_session(&other_config)
        .await
        .expect("second login failed")
        .character_id;
    drop(relogin(&other_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_gm_level(&gm_config.username, 1)
        .await
        .expect("make a GM");
    for id in [gm_id, other_id] {
        grant_skill(id, SUPER_DRAGON_ROAR, 1)
            .await
            .expect("teach Super Dragon Roar");
    }

    let mut other = relogin(&other_config).await;
    let o = &mut other.connection;
    let mut ot = Trace::default();
    assert_eq!(warp(o, FIELD, "", &mut ot).await, FIELD);
    until(o, &mut ot, |t| t.field.alive(SNAIL).len() >= 3).await;
    let snails = ot.field.alive(SNAIL)[..3].to_vec();

    // Not a GM: the swing is dropped and every snail lives.
    send(o, roar(&snails, 8)).await;
    read_for(o, &mut ot, 500).await;
    assert!(ot.field.killed.is_empty(), "a player's roar killed");
    assert!(ot.field.hp_bars.is_empty(), "a player's roar hurt");

    // The GM's one swing kills all three, which the other sees.
    let mut gm = relogin(&gm_config).await;
    let g = &mut gm.connection;
    let mut gt = Trace::default();
    assert_eq!(warp(g, FIELD, "", &mut gt).await, FIELD);
    send(g, roar(&snails, 8)).await;
    until(g, &mut gt, |t| {
        snails.iter().all(|s| t.field.killed.contains(s))
    })
    .await;
    until(o, &mut ot, |t| {
        snails.iter().all(|s| t.field.killed.contains(s))
    })
    .await;
}
