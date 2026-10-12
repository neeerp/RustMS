//! Mobs on Dangerous Forest (50000): spawning, control, movement, melee
//! attacks, kills and EXP, touch damage, respawning, and levelling up.

use integration_harness::hunting::attack;
use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, stat, until, warp, Trace};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const FOREST: i32 = 50_000;
const SNAIL: i32 = 100100;

fn move_mob(object_id: i32, x: i16, y: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::MoveLife as i16).unwrap();
    p.write_int(object_id).unwrap();
    p.write_short(1).unwrap();
    p.write_bytes(&[0; 19]).unwrap();
    p.write_short(x).unwrap();
    p.write_short(y).unwrap();
    // One absolute movement to (x + 10, y).
    p.write_byte(1).unwrap();
    p.write_byte(0).unwrap();
    p.write_short(x + 10).unwrap();
    p.write_short(y).unwrap();
    p.write_bytes(&[0; 6]).unwrap();
    p.write_byte(2).unwrap();
    p.write_short(100).unwrap();
    p
}

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
async fn snails_spawn_move_fight_back_die_and_respawn() {
    let hunter_config = load_harness_config_or_fail().await;
    let watcher_config = load_harness_config_or_fail().await;
    let mut hunter = login_to_world_session(&hunter_config).await.unwrap();
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();

    // The first player in gets every snail to control. Earlier tests may
    // have hunted here, so allow for a respawn.
    let mut h = Trace::default();
    assert_eq!(
        warp(&mut hunter.connection, FOREST, "", &mut h).await,
        FOREST
    );
    until(&mut hunter.connection, &mut h, |t| {
        t.field.controlled.len() >= 17
    })
    .await;
    let snails = h.field.alive(SNAIL);
    assert_eq!(snails.len(), 17);

    // The second sees them all but controls none.
    let mut w = Trace::default();
    assert_eq!(
        warp(&mut watcher.connection, FOREST, "", &mut w).await,
        FOREST
    );
    until(&mut watcher.connection, &mut w, |t| {
        t.field.mobs.len() >= 17
    })
    .await;
    read_for(&mut watcher.connection, &mut w, 300).await;
    assert!(w.field.controlled.is_empty());

    // Only the controller's movement is relayed.
    let snail = snails[0];
    send(&mut watcher.connection, move_mob(snail, 147, -25)).await;
    send(&mut hunter.connection, move_mob(snail, 147, -25)).await;
    until(&mut hunter.connection, &mut h, |t| {
        t.field.move_responses.contains(&snail)
    })
    .await;
    until(&mut watcher.connection, &mut w, |t| {
        t.field.moved.contains(&snail)
    })
    .await;
    assert_eq!(w.field.moved, [snail]);

    // Two hits of 4 kill an 8 HP snail; the watcher sees the swing and the death.
    send(&mut hunter.connection, attack(snail, 4)).await;
    until(&mut hunter.connection, &mut h, |t| {
        !t.field.hp_bars.is_empty()
    })
    .await;
    assert_eq!(h.field.hp_bars, [(snail, 50)]);
    send(&mut hunter.connection, attack(snail, 4)).await;
    until(&mut hunter.connection, &mut h, |t| {
        t.field.killed.contains(&snail) && !t.gains.is_empty()
    })
    .await;
    assert_eq!(h.gains, [3]);
    until(&mut watcher.connection, &mut w, |t| {
        t.field.killed.contains(&snail) && t.field.attacks_by.len() == 2
    })
    .await;
    assert_eq!(w.field.attacks_by, [hunter.character_id; 2]);

    // Hitting harder than the character can is refused.
    send(&mut hunter.connection, attack(snails[1], 500)).await;
    read_for(&mut hunter.connection, &mut h, 400).await;
    assert!(!h.field.hp_bars.iter().any(|(oid, _)| *oid == snails[1]));

    // Touching a snail hurts up to its attack (12), no more.
    send(&mut hunter.connection, touched(12, SNAIL)).await;
    until(&mut hunter.connection, &mut h, |t| {
        t.stat(stat::HP).is_some()
    })
    .await;
    let after_touch = h.stat_values(stat::HP);
    send(&mut hunter.connection, touched(13, SNAIL)).await;
    read_for(&mut hunter.connection, &mut h, 400).await;
    assert_eq!(h.stat_values(stat::HP), after_touch);

    // Four more snails (15 EXP total) reach level 2, which others see.
    for &snail in &snails[1..5] {
        send(&mut hunter.connection, attack(snail, 8)).await;
    }
    until(&mut hunter.connection, &mut h, |t| {
        t.stat(stat::LEVEL).is_some()
    })
    .await;
    assert_eq!(h.stat(stat::LEVEL), Some(2));
    assert!(h.stat(stat::MAX_HP).unwrap() > 50);
    until(&mut watcher.connection, &mut w, |t| {
        t.field.effects.contains(&(hunter.character_id, 0))
    })
    .await;

    // The first snail comes back after the respawn delay.
    let before = h.field.mobs.len();
    until(&mut hunter.connection, &mut h, |t| {
        t.field.mobs.len() > before
    })
    .await;
}
