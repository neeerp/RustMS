//! Pet auto potions: the potion set beside the HP pouch is kept with the
//! character, and the first pet out gives it from the USE bag. Another pet
//! may not.

use integration_harness::database::{grant_pet, grant_stack, set_hp, wear_cash_equip};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{send, stat, step, until, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

const BABY_TIGER: i32 = 5_000_066;
const HP_POUCH: i32 = 1_812_002;
/// Where the first pet out wears its HP pouch.
const FIRST_PET_HP_POUCH: i16 = -124;
const RED_POTION: i32 = 2_000_000;
const USE: i16 = 2;

/// SPAWN_PET: the pet in Cash slot `slot` comes out.
fn spawn_pet(slot: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::SpawnPet as i16).unwrap();
    p.write_int(0).unwrap(); // time
    p.write_short(slot).unwrap();
    p.write_byte(0).unwrap(); // not leading
    p
}

/// CHANGE_KEYMAP mode 1 (HP) or 2 (MP): the potion beside that pouch.
fn set_potion(mode: i32, item_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeKeybinds as i16).unwrap();
    p.write_int(mode).unwrap();
    p.write_int(item_id).unwrap();
    p
}

/// PET_AUTO_POT: the pet `serial` gives the item in USE slot `slot`.
fn pet_potion(serial: i64, slot: i16, item_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::PetAutoPot as i16).unwrap();
    p.write_long(serial).unwrap();
    p.write_byte(0).unwrap();
    p.write_int(0).unwrap(); // time
    p.write_short(slot).unwrap();
    p.write_int(item_id).unwrap();
    p
}

#[tokio::test]
async fn the_first_pet_gives_the_potion_set_beside_its_pouch() {
    let config = load_harness_config_or_fail().await;
    let session = login_to_world_session(&config).await.unwrap();
    let id = session.character_id;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let serial = 9_238_000_000 + 2 * i64::from(id);
    grant_pet(id, 1, BABY_TIGER, serial, "Baby Tiger")
        .await
        .expect("grant a pet");
    wear_cash_equip(id, FIRST_PET_HP_POUCH, HP_POUCH, serial + 1)
        .await
        .expect("put the pouch on the pet");
    grant_stack(id, USE, 1, RED_POTION, 20)
        .await
        .expect("grant potions");
    set_hp(id, 10, 500).await.expect("hp");

    let mut session = relogin(&config).await;
    assert_eq!(session.auto_potions, (0, 0));
    let m = session.character_name.clone();
    let c = &mut session.connection;
    let mut t = Trace::default();
    send(c, spawn_pet(1)).await;
    // The pet comes out through the field; the stat change answers apart.
    until(c, &mut t, |t| {
        !t.pets.is_empty() && !t.stat_updates.is_empty()
    })
    .await;

    // Set before it is given: nothing answers. A Red Potion restores no
    // MP, so the MP pouch's slot leaves it.
    let t = step(c, &format!("{m} set"), set_potion(1, RED_POTION)).await;
    assert!(t.stat_updates.is_empty());
    step(c, &format!("{m} set mp"), set_potion(2, RED_POTION)).await;

    // The pet gives one: 50 HP.
    let t = step(c, &format!("{m} give"), pet_potion(serial, 1, RED_POTION)).await;
    assert_eq!(t.quantities, [(USE as u8, 1, 19)]);
    assert_eq!(t.stat(stat::HP), Some(60));

    // A pet that is not out is refused, and the client is unlocked.
    let t = step(
        c,
        &format!("{m} other"),
        pet_potion(serial + 7, 1, RED_POTION),
    )
    .await;
    assert!(t.quantities.is_empty());
    assert_eq!(t.stat_updates.len(), 1);
    assert!(t.stat_updates[0].is_empty());
    drop(session);

    // The potion is kept with the character, and cleared with 0.
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut session = relogin(&config).await;
    assert_eq!(session.auto_potions, (RED_POTION, 0));
    step(
        &mut session.connection,
        &format!("{m} clear"),
        set_potion(1, 0),
    )
    .await;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let session = relogin(&config).await;
    assert_eq!(session.auto_potions, (0, 0));
}
