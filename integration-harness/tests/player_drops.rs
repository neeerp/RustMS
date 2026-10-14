//! Players dropping items (ITEM_MOVE to slot 0) and mesos (MESO_DROP): the
//! drop is anyone's at once, and the dropper's loss is saved.

use integration_harness::hunting::{attack, pickup};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, stat, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const FOREST: i32 = 50_000;
const SNAIL: i32 = 100100;
const WEAPON: i16 = -11;
const EQUIP: i8 = 1;
/// HeavenClient sends the EQUIPPED type for worn items.
const EQUIPPED: i8 = -1;

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

fn meso_drop(amount: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::MesoDrop as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_int(amount).unwrap();
    p
}

#[tokio::test]
async fn a_dropped_weapon_is_anyones_and_gone_from_the_dropper() {
    let dropper_config = load_harness_config_or_fail().await;
    let finder_config = load_harness_config_or_fail().await;
    let mut dropper = login_to_world_session(&dropper_config).await.unwrap();
    let mut finder = login_to_world_session(&finder_config).await.unwrap();
    let weapon = dropper.equipped[&WEAPON.abs()];
    let (mut d, mut f) = (Trace::default(), Trace::default());
    assert_eq!(
        warp(&mut dropper.connection, FOREST, "", &mut d).await,
        FOREST
    );
    assert_eq!(
        warp(&mut finder.connection, FOREST, "", &mut f).await,
        FOREST
    );
    read_for(&mut dropper.connection, &mut d, 300).await;
    read_for(&mut finder.connection, &mut f, 300).await;

    // Take the weapon off, then drop it from the bag.
    send(&mut dropper.connection, item_move(EQUIPPED, WEAPON, 1, 1)).await;
    read_for(&mut dropper.connection, &mut d, 300).await;
    send(&mut dropper.connection, item_move(EQUIP, 1, 0, 1)).await;
    until(&mut finder.connection, &mut f, |t| {
        t.field.item_drop(weapon).is_some()
    })
    .await;
    let loot = f.field.item_drop(weapon).unwrap();
    until(&mut dropper.connection, &mut d, |t| {
        t.field.item_drop(weapon).is_some()
    })
    .await;

    // Someone else may pick it up straight away.
    send(
        &mut finder.connection,
        pickup(loot.object_id, loot.x, loot.y),
    )
    .await;
    until(&mut finder.connection, &mut f, |t| {
        t.added.iter().any(|a| a.2 == weapon)
    })
    .await;
    until(&mut dropper.connection, &mut d, |t| {
        !t.field.removals(loot.object_id).is_empty()
    })
    .await;
    assert_eq!(
        d.field.removals(loot.object_id),
        [Some(finder.character_id)]
    );
    drop(dropper);
    drop(finder);

    let dropper = relogin(&dropper_config).await;
    assert!(!dropper.equipped.contains_key(&WEAPON.abs()));
    assert!(!dropper.items.contains_key(&weapon));
    let finder = relogin(&finder_config).await;
    assert_eq!(finder.items.get(&weapon), Some(&1));
}

#[tokio::test]
async fn mesos_can_be_dropped_once_there_are_ten() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config).await.unwrap();
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut session.connection, FOREST, "", &mut seen).await,
        FOREST
    );
    until(&mut session.connection, &mut seen, |t| {
        t.field.alive(SNAIL).len() >= 10
    })
    .await;

    // A new character has no mesos to drop.
    send(&mut session.connection, meso_drop(10)).await;
    read_for(&mut session.connection, &mut seen, 400).await;
    assert!(seen.field.drops.iter().all(|d| !d.meso || d.value != 10));

    // Hunt snails (5 mesos 65% of the time) until there are ten.
    let mut meso = 0;
    for oid in seen.field.alive(SNAIL) {
        let old_drops = seen.field.drops.len();
        send(&mut session.connection, attack(oid, 8)).await;
        until(&mut session.connection, &mut seen, |t| {
            t.field.killed.contains(&oid)
        })
        .await;
        read_for(&mut session.connection, &mut seen, 200).await;
        let bags: Vec<_> = seen.field.drops[old_drops..]
            .iter()
            .filter(|d| d.meso)
            .copied()
            .collect();
        for bag in bags {
            send(&mut session.connection, pickup(bag.object_id, bag.x, bag.y)).await;
            meso += bag.value;
            until(&mut session.connection, &mut seen, |t| {
                t.stat(stat::MESO) == Some(i64::from(meso))
            })
            .await;
        }
        if meso >= 10 {
            break;
        }
    }
    assert!(meso >= 10, "the snails dropped {meso} mesos");

    // Asking for more than there are is refused; ten is fine.
    let old_drops = seen.field.drops.len();
    send(&mut session.connection, meso_drop(meso + 1)).await;
    send(&mut session.connection, meso_drop(10)).await;
    until(&mut session.connection, &mut seen, |t| {
        t.stat(stat::MESO) == Some(i64::from(meso - 10))
    })
    .await;
    until(&mut session.connection, &mut seen, |t| {
        t.field.drops.len() > old_drops
    })
    .await;
    let bag = seen.field.drops[old_drops];
    assert!(bag.meso);
    assert_eq!(bag.value, 10);
    drop(session);

    assert_eq!(relogin(&config).await.meso, meso - 10);
}
