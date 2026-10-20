//! A pet's exception list: the client sends the whole list on every change,
//! and the server hands it back with the pet on every map and after a relog.
//! A pet wearing an Item Ignore Pendant leaves what the list names.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_pet, wear_cash_equip};
use integration_harness::hunting::attack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::read::PktRead;
use packet::io::write::PktWrite;
use packet::Packet;
use std::io::Cursor;

const BABY_TIGER: i32 = 5_000_066;
/// Ellinia, as tests/pets.rs.
const ELLINIA: i32 = 101_000_000;
const SNAIL_SHELL: i32 = 4_000_000;
const MESOS: i32 = i32::MAX;
const FOREST: i32 = 50_000;
const SNAIL: i32 = 100100;
const MESO_MAGNET: i32 = 1_812_000;
const ITEM_IGNORE_PENDANT: i32 = 1_812_007;
/// Where the first pet out wears its Meso Magnet and Item Ignore Pendant.
const FIRST_PET_MAGNET: i16 = -123;
const FIRST_PET_IGNORE: i16 = -146;

/// SPAWN_PET: the pet in Cash slot `slot` comes out.
fn spawn_pet(slot: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::SpawnPet as i16).unwrap();
    p.write_int(0).unwrap(); // time
    p.write_short(slot).unwrap();
    p.write_byte(0).unwrap(); // not leading
    p
}

/// PET_EXCLUDE_ITEMS: the whole exception list of the pet `serial`.
fn exclude_items(serial: i64, items: &[i32]) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::PetExcludeItems as i16).unwrap();
    p.write_long(serial).unwrap();
    p.write_byte(items.len() as u8).unwrap();
    for &item in items {
        p.write_int(item).unwrap();
    }
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

/// Read until the next PET_EXCEPTION_LIST, and return its pet index,
/// serial and items.
async fn exception_list(
    c: &mut MapleTestConnection,
    t: &mut Trace,
    owner_id: i32,
) -> (u8, i64, Vec<i32>) {
    loop {
        let p = read(c, t).await;
        if p.opcode() != SendOpcode::PetExceptionList as i16 {
            continue;
        }
        let mut r = Cursor::new(&p.bytes[2..]);
        assert_eq!(r.read_int().unwrap(), owner_id);
        let index = r.read_byte().unwrap();
        let serial = r.read_long().unwrap();
        let count = r.read_byte().unwrap();
        let items = (0..count).map(|_| r.read_int().unwrap()).collect();
        return (index, serial, items);
    }
}

#[tokio::test]
async fn a_pets_exception_list_comes_back_with_it() {
    let config = load_harness_config_or_fail().await;
    let session = login_to_world_session(&config).await.unwrap();
    let owner_id = session.character_id;
    drop(session);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let serial = 9_212_000_000 + i64::from(owner_id);
    grant_pet(owner_id, 1, BABY_TIGER, serial, "Baby Tiger")
        .await
        .expect("grant a pet");
    let mut owner = relogin(&config).await;
    let (o, mut t) = (&mut owner.connection, Trace::default());

    // Out for the first time, it leaves nothing.
    send(o, spawn_pet(1)).await;
    assert_eq!(
        exception_list(o, &mut t, owner_id).await,
        (0, serial, vec![])
    );

    // A list with an item that doesn't exist changes nothing; a good one
    // comes back on the next map.
    send(o, exclude_items(serial, &[9_999_999])).await;
    assert_eq!(warp(o, ELLINIA, "", &mut t).await, ELLINIA);
    assert_eq!(
        exception_list(o, &mut t, owner_id).await,
        (0, serial, vec![])
    );
    send(o, exclude_items(serial, &[MESOS, SNAIL_SHELL])).await;
    assert_eq!(warp(o, 100_000_000, "", &mut t).await, 100_000_000);
    let (_, _, mut items) = exception_list(o, &mut t, owner_id).await;
    items.sort();
    assert_eq!(items, [SNAIL_SHELL, MESOS]);

    // It is kept with the pet.
    drop(owner);
    let mut owner = relogin(&config).await;
    let (o, mut t) = (&mut owner.connection, Trace::default());
    let (index, seen, mut items) = exception_list(o, &mut t, owner_id).await;
    items.sort();
    assert_eq!((index, seen, items), (0, serial, vec![SNAIL_SHELL, MESOS]));
}

#[tokio::test]
async fn a_pet_wearing_an_item_ignore_pendant_leaves_what_its_list_names() {
    let config = load_harness_config_or_fail().await;
    let session = login_to_world_session(&config).await.unwrap();
    let owner_id = session.character_id;
    drop(session);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let serial = 9_212_500_000 + 3 * i64::from(owner_id);
    grant_pet(owner_id, 1, BABY_TIGER, serial, "Baby Tiger")
        .await
        .expect("grant a pet");
    wear_cash_equip(owner_id, FIRST_PET_MAGNET, MESO_MAGNET, serial + 1)
        .await
        .expect("put the magnet on the pet");
    wear_cash_equip(owner_id, FIRST_PET_IGNORE, ITEM_IGNORE_PENDANT, serial + 2)
        .await
        .expect("put the pendant on the pet");
    let mut owner = relogin(&config).await;
    let (o, mut t) = (&mut owner.connection, Trace::default());
    assert_eq!(warp(o, FOREST, "", &mut t).await, FOREST);
    until(o, &mut t, |t| t.field.mobs.len() >= 17).await;
    send(o, spawn_pet(1)).await;
    until(o, &mut t, |t| !t.pets.is_empty()).await;
    send(o, exclude_items(serial, &[MESOS])).await;
    read_for(o, &mut t, 300).await;
    let old_drops = t.field.drops.len();

    for oid in t.field.alive(SNAIL) {
        send(o, attack(oid, 8)).await;
        until(o, &mut t, |t| t.field.killed.contains(&oid)).await;
        read_for(o, &mut t, 200).await;
        if t.field.drops[old_drops..].iter().any(|d| d.meso) {
            break;
        }
    }
    let mesos = *t.field.drops[old_drops..]
        .iter()
        .find(|d| d.meso)
        .expect("seventeen snails dropped no mesos");

    // The list names mesos, so the pet leaves them.
    send(o, pet_loot(serial, mesos.object_id, mesos.x, mesos.y)).await;
    read_for(o, &mut t, 400).await;
    assert!(t.field.removals(mesos.object_id).is_empty());

    // With the list emptied it takes them.
    send(o, exclude_items(serial, &[])).await;
    send(o, pet_loot(serial, mesos.object_id, mesos.x, mesos.y)).await;
    until(o, &mut t, |t| !t.field.removals(mesos.object_id).is_empty()).await;
}
