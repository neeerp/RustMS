//! The Ariant Coliseum's Bomb (2100067), the first of its pieces: it opens
//! only in the Coliseum's three battle maps (`reqMap`), where it brings out
//! a bomb (9300166) whose hits knock Spirit Jewels (4031868) out of a
//! player's bag, five rolls at 40 percent each (`loseItem`); they fall
//! to either side of the player. The Barrier (2022269), which works when
//! picked up, keeps them in.

use integration_harness::database::{grant_stack, set_hp};
use integration_harness::hunting::pickup;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

const HENESYS: i32 = 100_000_000;
const BATTLE_MAP: i32 = 980_010_101;
const BOMB: i32 = 2_100_067;
const BOMB_MOB: i32 = 9_300_166;
const BARRIER: i32 = 2_022_269;
const JEWEL: i32 = 4_031_868;
const JEWELS: i16 = 30;
const USE: u8 = 2;
const ETC: u8 = 4;

/// USE_SUMMON_BAG of the sack in USE `slot`.
fn open_sack(slot: i16, item_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseSummonBag as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(item_id).unwrap();
    p
}

/// TAKE_DAMAGE of `damage` from `mob_id`'s first attack.
fn hurt(damage: i32, mob_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::TakeDamage as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(0).unwrap();
    p.write_byte(0).unwrap();
    p.write_int(damage).unwrap();
    p.write_int(mob_id).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(0).unwrap();
    p
}

/// Throw the USE item in `slot` on the ground.
fn throw_use(slot: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemMove as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(USE).unwrap();
    p.write_short(slot).unwrap();
    p.write_short(0).unwrap();
    p.write_short(1).unwrap();
    p
}

/// How many Spirit Jewels `t` last saw in the bag, or `JEWELS`.
fn jewels(t: &Trace) -> i16 {
    if t.removed.contains(&(ETC, 1)) {
        return 0;
    }
    (t.quantities.iter().rev())
        .find(|&&(kind, slot, _)| (kind, slot) == (ETC, 1))
        .map_or(JEWELS, |&(_, _, quantity)| quantity)
}

/// Where the Spirit Jewels on the ground in `t` lie.
fn jewel_drops(t: &Trace) -> Vec<i16> {
    (t.field.drops.iter())
        .filter(|d| !d.meso && d.value == JEWEL)
        .map(|d| d.x)
        .collect()
}

#[tokio::test]
async fn the_bomb_knocks_spirit_jewels_out_unless_a_barrier_keeps_them_in() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_hp(id, 5_000, 5_000).await.unwrap();
    grant_stack(id, USE.into(), 1, BOMB, 2).await.unwrap();
    grant_stack(id, USE.into(), 2, BARRIER, 1).await.unwrap();
    grant_stack(id, ETC.into(), 1, JEWEL, JEWELS).await.unwrap();
    let mut player = relogin(&config).await;
    let c = &mut player.connection;

    // Outside the Coliseum the Bomb stays shut.
    let mut t = Trace::default();
    assert_eq!(warp(c, HENESYS, "", &mut t).await, HENESYS);
    send(c, open_sack(1, BOMB)).await;
    read_for(c, &mut t, 500).await;
    assert!(
        t.quantities.is_empty() && t.field.alive(BOMB_MOB).is_empty(),
        "{t:?}"
    );

    // In a battle map it brings out the bomb, whose hits knock jewels out
    // to either side of the player, each its own spot.
    let mut t = Trace::default();
    assert_eq!(warp(c, BATTLE_MAP, "", &mut t).await, BATTLE_MAP);
    send(c, open_sack(1, BOMB)).await;
    until(c, &mut t, |t| !t.field.alive(BOMB_MOB).is_empty()).await;
    for _ in 0..20 {
        if jewels(&t) < JEWELS {
            break;
        }
        send(c, hurt(1, BOMB_MOB)).await;
        read_for(c, &mut t, 200).await;
    }
    let lost = JEWELS - jewels(&t);
    assert!((1..=5).contains(&lost), "{t:?}");
    until(c, &mut t, |t| jewel_drops(t).len() == lost as usize).await;
    let mut xs = jewel_drops(&t);
    xs.sort();
    xs.dedup();
    assert_eq!(xs.len(), lost as usize, "{t:?}");

    // The Barrier, picked up, keeps them in.
    send(c, throw_use(2)).await;
    until(c, &mut t, |t| t.field.item_drop(BARRIER).is_some()).await;
    let barrier = t.field.item_drop(BARRIER).unwrap();
    send(c, pickup(barrier.object_id, barrier.x, barrier.y)).await;
    until(c, &mut t, |t| {
        t.buffs.iter().any(|&(id, _, _)| id == -BARRIER)
    })
    .await;
    let kept = jewels(&t);
    for _ in 0..10 {
        send(c, hurt(1, BOMB_MOB)).await;
    }
    read_for(c, &mut t, 1_000).await;
    assert_eq!(jewels(&t), kept, "{t:?}");
    assert_eq!(jewel_drops(&t).len(), lost as usize, "{t:?}");
}
