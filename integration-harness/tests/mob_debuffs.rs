//! Debuffs on Snail Hunting Ground II (40001): a Fire/Poison Wizard's Slow
//! from USE_SKILL slows the snails it reaches, the map sees it put on
//! (MOB_STAT_SET), and it wears off (MOB_STAT_RESET).

use integration_harness::database::{grant_skill, set_job};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, stat, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::{Duration, Instant};

const HUNTING_GROUND: i32 = 40_001;
const SNAIL: i32 = 100100;
const FP_WIZARD: i16 = 210;
const SLOW: i32 = 2101003;
const SPEED: u128 = 1 << 6;

/// USE_SKILL for Slow at level 1, cast from (x, y) on `mobs`.
fn slow(x: i16, y: i16, mobs: &[i32]) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseSkill as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_int(SLOW).unwrap();
    p.write_byte(1).unwrap();
    p.write_short(x).unwrap();
    p.write_short(y).unwrap();
    p.write_byte(mobs.len() as u8).unwrap();
    for &mob in mobs {
        p.write_int(mob).unwrap();
    }
    p.write_short(0).unwrap();
    p
}

#[tokio::test]
async fn slow_slows_the_snails_it_reaches_until_it_wears_off() {
    let wizard_config = load_harness_config_or_fail().await;
    let watcher_config = load_harness_config_or_fail().await;

    // Make the wizard, then teach it Slow while logged out.
    let wizard_id = login_to_world_session(&wizard_config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&wizard_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(wizard_id, FP_WIZARD, 30, 100)
        .await
        .expect("make a wizard");
    grant_skill(wizard_id, SLOW, 1).await.expect("teach Slow");

    let mut wizard = relogin(&wizard_config).await;
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();

    let mut w = Trace::default();
    assert_eq!(
        warp(&mut wizard.connection, HUNTING_GROUND, "", &mut w).await,
        HUNTING_GROUND
    );
    let mut o = Trace::default();
    assert_eq!(
        warp(&mut watcher.connection, HUNTING_GROUND, "", &mut o).await,
        HUNTING_GROUND
    );
    until(&mut wizard.connection, &mut w, |t| {
        !t.field.alive(SNAIL).is_empty()
    })
    .await;
    let snail = w.field.alive(SNAIL)[0];
    let (x, y) = w.field.mob_positions[&snail];

    // Slow at level 1: 8 MP, the snail's speed -2 for 2 seconds. An object
    // id that isn't a mob is passed over.
    send(&mut wizard.connection, slow(x, y, &[snail, 0, 999_999])).await;
    until(&mut wizard.connection, &mut w, |t| {
        !t.field.stat_sets.is_empty()
    })
    .await;
    let slowed = Instant::now();
    assert_eq!(w.field.stat_sets, [(snail, SPEED, -2, SLOW)]);
    assert_eq!(w.stat(stat::MP), Some(92));
    until(&mut watcher.connection, &mut o, |t| {
        t.field.stat_sets == [(snail, SPEED, -2, SLOW)]
    })
    .await;

    until(&mut wizard.connection, &mut w, |t| {
        !t.field.stat_resets.is_empty()
    })
    .await;
    assert_eq!(w.field.stat_resets, [(snail, SPEED)]);
    assert!(slowed.elapsed() >= Duration::from_millis(1500));
}
