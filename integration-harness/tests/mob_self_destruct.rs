//! Mobs that destroy themselves (`selfDestruction`): a Boomer (9400547,
//! 700 HP) in New Leaf City's subway (600010300) blows up once a hit leaves
//! it at 350 HP or less, leaving with its action (5, an explosion), and
//! counts as killed.

use integration_harness::database::{set_str, set_weapon_attack};
use integration_harness::hunting::attack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const MAP: i32 = 600_010_300;
const BOOMER: i32 = 9_400_547;

#[tokio::test]
async fn a_boomer_blows_up_at_half_its_hp() {
    let config = load_harness_config_or_fail().await;
    // A beginner strong enough to take half a Boomer's HP in one hit.
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_str(id, 999).await.expect("make it strong");
    set_weapon_attack(id, 100).await.expect("sharpen its sword");
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, MAP, "", &mut t).await, MAP);
    until(c, &mut t, |t| !t.field.alive(BOOMER).is_empty()).await;
    let boomer = t.field.alive(BOOMER)[0];

    send(c, attack(boomer, 349)).await;
    until(c, &mut t, |t| {
        t.field.hp_bars.iter().any(|&(oid, _)| oid == boomer)
    })
    .await;
    read_for(c, &mut t, 300).await;
    assert!(!t.field.killed.contains(&boomer), "gone above 350 HP");

    send(c, attack(boomer, 1)).await;
    until(c, &mut t, |t| t.field.killed.contains(&boomer)).await;
    assert_eq!(t.field.leave_types[&boomer], 5);
    until(c, &mut t, |t| !t.gains.is_empty()).await;
}
