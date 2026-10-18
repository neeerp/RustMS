//! Pet loot: a pet wearing a Meso Magnet picks up its owner's mesos, which
//! fly to the pet, and without an Item Pouch it leaves items where they lie.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_pet, wear_cash_equip};
use integration_harness::hunting::attack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, read_for, send, stat, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;

const FOREST: i32 = 50_000;
const SNAIL: i32 = 100100;
const BABY_TIGER: i32 = 5_000_066;
const MESO_MAGNET: i32 = 1_812_000;
/// Where the first pet out wears its Meso Magnet.
const FIRST_PET_MAGNET: i16 = -123;

/// SPAWN_PET: the pet in Cash slot `slot` comes out.
fn spawn_pet(slot: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::SpawnPet as i16).unwrap();
    p.write_int(0).unwrap(); // time
    p.write_short(slot).unwrap();
    p.write_byte(0).unwrap(); // not leading
    p
}

/// PET_LOOT: the pet `serial`, standing at (x, y), reaches for a drop.
fn pet_loot(serial: i64, object_id: i32, x: i16, y: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::PetLoot as i16).unwrap();
    p.write_long(serial).unwrap();
    p.write_byte(0).unwrap(); // field key
    p.write_int(0).unwrap(); // time
    p.write_short(x).unwrap();
    p.write_short(y).unwrap();
    p.write_int(object_id).unwrap();
    p.write_int(0).unwrap(); // checksum
    p.write_bytes(&[0, 0, 0]).unwrap(); // others' drops, sweep, long range
    p
}

/// Read until the drop `object_id` leaves the map, and return the body.
async fn removal(c: &mut MapleTestConnection, t: &mut Trace, object_id: i32) -> Vec<u8> {
    loop {
        let p = read(c, t).await;
        if p.opcode() == SendOpcode::RemoveItemFromMap as i16
            && p.bytes[3..7] == object_id.to_le_bytes()
        {
            return p.bytes[2..].to_vec();
        }
    }
}

#[tokio::test]
async fn a_pet_with_a_meso_magnet_takes_mesos_but_not_items() {
    let config = load_harness_config_or_fail().await;
    let session = login_to_world_session(&config).await.unwrap();
    let owner_id = session.character_id;
    drop(session);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let serial = 9_159_000_000 + 2 * i64::from(owner_id);
    grant_pet(owner_id, 1, BABY_TIGER, serial, "Baby Tiger")
        .await
        .expect("grant a pet");
    wear_cash_equip(owner_id, FIRST_PET_MAGNET, MESO_MAGNET, serial + 1)
        .await
        .expect("put the magnet on the pet");
    let mut owner = relogin(&config).await;
    let (o, mut t) = (&mut owner.connection, Trace::default());
    assert_eq!(warp(o, FOREST, "", &mut t).await, FOREST);
    until(o, &mut t, |t| t.field.mobs.len() >= 17).await;
    send(o, spawn_pet(1)).await;
    until(o, &mut t, |t| !t.pets.is_empty()).await;
    read_for(o, &mut t, 300).await;
    // Earlier tests may have left drops on this map.
    let old_drops = t.field.drops.len();

    // Kill snails until mesos drop; every kill drops a Monster Book card
    // as well (tests/monster_book.rs).
    for oid in t.field.alive(SNAIL) {
        send(o, attack(oid, 8)).await;
        until(o, &mut t, |t| t.field.killed.contains(&oid)).await;
        read_for(o, &mut t, 200).await;
        if t.field.drops[old_drops..].iter().any(|d| d.meso) {
            break;
        }
    }
    let ours = &t.field.drops[old_drops..];
    let mesos = *ours
        .iter()
        .find(|d| d.meso)
        .expect("seventeen snails dropped no mesos");
    let card = *ours
        .iter()
        .find(|d| !d.meso && d.value / 10_000 == 238)
        .expect("a card dropped");

    // Without an Item Pouch the pet leaves the card.
    send(o, pet_loot(serial, card.object_id, card.x, card.y)).await;
    read_for(o, &mut t, 400).await;
    assert!(t.field.removals(card.object_id).is_empty());

    // The mesos fly to the first pet, and are its owner's.
    send(o, pet_loot(serial, mesos.object_id, mesos.x, mesos.y)).await;
    let mut taken = vec![5];
    taken.extend(mesos.object_id.to_le_bytes());
    taken.extend(owner_id.to_le_bytes());
    taken.push(0);
    assert_eq!(removal(o, &mut t, mesos.object_id).await, taken);
    until(o, &mut t, |t| t.stat(stat::MESO).is_some()).await;
    assert_eq!(t.stat(stat::MESO), Some(i64::from(mesos.value)));
}
