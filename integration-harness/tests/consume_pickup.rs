//! Items with `consumeOnPickup`, such as the reactor fruits, work the
//! moment they're picked up and never reach the bag. The Coliseum's Crazy
//! Skull works on everyone else on the map instead.

use integration_harness::database::grant_stack;
use integration_harness::hunting::pickup;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

const FOREST: i32 = 50_000;
const USE: i16 = 2;
const SLOT: i16 = 5;
/// Red Fruit: weapon attack for a while.
const RED_FRUIT: i32 = 2_022_086;
/// A map no other test here uses.
const ROAD: i32 = 1_010_000;
/// Crazy Skull: Confuse (mob skill 132, level 1) on everyone else.
const CRAZY_SKULL: i32 = 2_022_268;
const CONFUSE: i32 = 132 | 1 << 16;

fn item_move(kind: i8, from: i16, to: i16, quantity: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemMove as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_byte(kind as u8).unwrap();
    p.write_short(from).unwrap();
    p.write_short(to).unwrap();
    p.write_short(quantity).unwrap();
    p
}

#[tokio::test]
async fn a_fruit_picked_up_buffs_and_is_gone() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, USE, SLOT, RED_FRUIT, 1)
        .await
        .expect("grant fruit");
    let mut player = relogin(&config).await;
    let c = &mut player.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, FOREST, "", &mut t).await, FOREST);
    read_for(c, &mut t, 300).await;

    // Drop it from the bag, then pick it back up.
    send(c, item_move(USE as i8, SLOT, 0, 1)).await;
    until(c, &mut t, |t| t.field.item_drop(RED_FRUIT).is_some()).await;
    let loot = t.field.item_drop(RED_FRUIT).unwrap();
    let added = t.added.len();
    send(c, pickup(loot.object_id, loot.x, loot.y)).await;
    until(c, &mut t, |t| {
        t.buffs.iter().any(|&(id, _, _)| id == -RED_FRUIT)
    })
    .await;
    until(c, &mut t, |t| !t.field.removals(loot.object_id).is_empty()).await;
    read_for(c, &mut t, 300).await;
    assert!(t
        .buffs
        .iter()
        .any(|&(id, value, time)| id == -RED_FRUIT && value > 0 && time > 0));
    assert!(t.added[added..]
        .iter()
        .all(|&(_, _, item)| item != RED_FRUIT));
    drop(player);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let player = relogin(&config).await;
    assert!(!player.items.contains_key(&RED_FRUIT));
}

#[tokio::test]
async fn a_crazy_skull_confuses_everyone_else_on_the_map() {
    let configs = [
        load_harness_config_or_fail().await,
        load_harness_config_or_fail().await,
    ];
    let mut ids = Vec::new();
    for config in &configs {
        let id = login_to_world_session(config)
            .await
            .expect("first login failed")
            .character_id;
        drop(relogin(config).await);
        ids.push(id);
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(ids[0], USE, SLOT, CRAZY_SKULL, 1)
        .await
        .expect("grant the skull");
    let mut picker = relogin(&configs[0]).await;
    let mut other = relogin(&configs[1]).await;
    let (p, o) = (&mut picker.connection, &mut other.connection);
    let (mut pt, mut ot) = (Trace::default(), Trace::default());
    assert_eq!(warp(p, ROAD, "", &mut pt).await, ROAD);
    assert_eq!(warp(o, ROAD, "", &mut ot).await, ROAD);
    read_for(p, &mut pt, 300).await;
    read_for(o, &mut ot, 300).await;

    send(p, item_move(USE as i8, SLOT, 0, 1)).await;
    until(p, &mut pt, |t| t.field.item_drop(CRAZY_SKULL).is_some()).await;
    let loot = pt.field.item_drop(CRAZY_SKULL).unwrap();
    let added = pt.added.len();
    send(p, pickup(loot.object_id, loot.x, loot.y)).await;
    until(p, &mut pt, |t| !t.field.removals(loot.object_id).is_empty()).await;
    until(o, &mut ot, |t| {
        t.buffs.iter().any(|&(id, _, _)| id == CONFUSE)
    })
    .await;
    read_for(p, &mut pt, 500).await;
    assert!(ot
        .buffs
        .iter()
        .any(|&(id, value, time)| id == CONFUSE && value > 0 && time > 0));
    assert!(
        pt.buffs.iter().all(|&(id, _, _)| id != CONFUSE),
        "{:?}",
        pt.buffs
    );
    assert!(pt.added[added..]
        .iter()
        .all(|&(_, _, item)| item != CRAZY_SKULL));
}
