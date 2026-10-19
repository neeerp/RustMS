//! Map traps: objs marked `obstacle`, such as the Forest of Patience's
//! thorns (`damage` 1). A v83 client tests them itself and reports a hit as
//! TAKE_DAMAGE from -3; the server takes the HP and shows it to the map, and
//! refuses more than the map's traps can do.

use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{barrier, send, stat, until, warp, Trace};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

/// The Forest of Patience <Step 1>.
const PATIENCE: i32 = 101_000_100;

/// TAKE_DAMAGE as a v83 client sends it from a trap (0x9593f7 with no mob):
/// source -3, no element, then the disease it names.
fn trap_damage(damage: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TakeDamage as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(-3i8 as u8).unwrap();
    p.write_byte(0).unwrap();
    p.write_int(damage).unwrap();
    p.write_short(0).unwrap();
    p.write_byte(0).unwrap();
    p
}

#[tokio::test(flavor = "multi_thread")]
async fn a_trap_hurts_the_player_who_touches_it() {
    let config = load_harness_config_or_fail().await;
    let watcher_config = load_harness_config_or_fail().await;
    let mut player = login_to_world_session(&config).await.unwrap();
    let id = player.character_id;
    let mut watcher = login_to_world_session(&watcher_config).await.unwrap();
    let (c, w) = (&mut player.connection, &mut watcher.connection);
    let (mut t, mut wt) = (Trace::default(), Trace::default());
    assert_eq!(warp(w, PATIENCE, "", &mut wt).await, PATIENCE);
    assert_eq!(warp(c, PATIENCE, "", &mut t).await, PATIENCE);

    // A thorn takes 1 HP, and the others see it.
    send(c, trap_damage(1)).await;
    until(c, &mut t, |t| t.stat(stat::HP).is_some()).await;
    let hp = t.stat(stat::HP).unwrap();
    until(w, &mut wt, |t| t.field.players_hurt.contains(&(id, -3, 1))).await;
    send(c, trap_damage(1)).await;
    until(c, &mut t, |t| t.stat(stat::HP) == Some(hp - 1)).await;

    // More than the map's traps do is refused.
    let updates = t.stat_updates.len();
    send(c, trap_damage(2)).await;
    barrier(c, "refused", &mut t).await;
    assert_eq!(t.stat_updates.len(), updates);
}
