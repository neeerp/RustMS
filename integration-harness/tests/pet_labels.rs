//! Pet label and quote rings: a pet out shows its kind's name tag while a
//! label ring is worn in its part, in SPAWN_PET and in the PET_NAMECHANGE
//! sent when the ring comes off or goes back on; and speaks in its kind's
//! chat balloon while a quote ring is worn in its part.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_pet, grant_skill, wear_cash_equip};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;

const BABY_TIGER: i32 = 5_000_066;
const LABEL_RING: i32 = 1_822_000;
const QUOTE_RING: i32 = 1_832_000;
/// Where the first pet out wears its quote ring.
const FIRST_PET_QUOTE: i16 = -129;
/// "Cute" (`interact/28`), which a level 1 tiger knows.
const CUTE: u8 = 28;
/// Where the first pet out wears its label ring.
const FIRST_PET_LABEL: i16 = -121;
/// Where the second pet out wears its label ring.
const SECOND_PET_LABEL: i16 = -131;
/// Follow the Lead: more than one pet out.
const FOLLOW_THE_LEAD: i32 = 8;
/// A free Equip slot.
const BAG_SLOT: i16 = 24;

/// The body of the next packet `op`.
async fn next(c: &mut MapleTestConnection, t: &mut Trace, op: SendOpcode) -> Vec<u8> {
    let op = op as i16;
    loop {
        let p = read(c, t).await;
        if p.opcode() == op {
            return p.bytes[2..].to_vec();
        }
    }
}

/// SPAWN_PET: the pet in Cash slot `slot` comes out.
fn spawn_pet(slot: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::SpawnPet as i16).unwrap();
    p.write_int(0).unwrap(); // time
    p.write_short(slot).unwrap();
    p.write_byte(0).unwrap(); // not leading
    p
}

/// PET_COMMAND: tell the pet `serial` to do `command`.
fn pet_command(serial: i64, command: u8) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::PetCommand as i16).unwrap();
    p.write_long(serial).unwrap();
    p.write_byte(1).unwrap(); // began with its name
    p.write_byte(command).unwrap();
    p
}

/// ITEM_MOVE of one item of `kind` from `from` to `to`.
fn item_move(kind: i8, from: i16, to: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ItemMove as i16).unwrap();
    p.write_int(0).unwrap(); // time
    p.write_byte(kind as u8).unwrap();
    p.write_short(from).unwrap();
    p.write_short(to).unwrap();
    p.write_short(1).unwrap();
    p
}

/// The 172 for pet `index`, `name`, with or without its name tag.
fn renamed(owner_id: i32, index: u8, name: &str, tagged: u8) -> Vec<u8> {
    let mut body = owner_id.to_le_bytes().to_vec();
    body.push(index);
    body.extend((name.len() as u16).to_le_bytes());
    body.extend(name.as_bytes());
    body.push(tagged);
    body
}

#[tokio::test]
async fn label_and_quote_rings_style_the_pets_tag_and_balloon() {
    let config = load_harness_config_or_fail().await;
    let session = login_to_world_session(&config).await.unwrap();
    let owner_id = session.character_id;
    drop(session);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let serial = 9_182_000_000 + 3 * i64::from(owner_id);
    grant_skill(owner_id, FOLLOW_THE_LEAD, 1)
        .await
        .expect("grant Follow the Lead");
    grant_pet(owner_id, 1, BABY_TIGER, serial, "Baby Tiger")
        .await
        .expect("grant a pet");
    grant_pet(owner_id, 2, BABY_TIGER, serial + 1, "Tigger")
        .await
        .expect("grant a second pet");
    wear_cash_equip(owner_id, FIRST_PET_LABEL, LABEL_RING, serial + 2)
        .await
        .expect("put the label ring on the pet");
    wear_cash_equip(owner_id, FIRST_PET_QUOTE, QUOTE_RING, serial + 3)
        .await
        .expect("put the quote ring on the pet");
    let mut owner = relogin(&config).await;
    let (o, mut t) = (&mut owner.connection, Trace::default());

    // It comes out with its name tag and its chat balloon.
    send(o, spawn_pet(1)).await;
    let spawned = next(o, &mut t, SendOpcode::SpawnPet).await;
    assert_eq!(spawned[spawned.len() - 2..], [1, 1], "name tag, balloon");

    // Taking the ring off takes the tag away; putting it back shows it.
    send(o, item_move(-1, FIRST_PET_LABEL, BAG_SLOT)).await;
    let tiger = |tagged| renamed(owner_id, 0, "Baby Tiger", tagged);
    assert_eq!(next(o, &mut t, SendOpcode::PetNameChange).await, tiger(0));
    send(o, item_move(1, BAG_SLOT, FIRST_PET_LABEL)).await;
    assert_eq!(next(o, &mut t, SendOpcode::PetNameChange).await, tiger(1));

    // A second pet comes out without a ring, then gets the first pet's.
    send(o, spawn_pet(2)).await;
    let second = next(o, &mut t, SendOpcode::SpawnPet).await;
    assert_eq!(second[second.len() - 2..], [0, 0], "no rings on Tigger");
    // Its answers are plain, while the first pet's take its balloon.
    send(o, pet_command(serial + 1, CUTE)).await;
    let answer = next(o, &mut t, SendOpcode::PetCommand).await;
    assert_eq!(answer[4..6], [1, 0], "Tigger's command");
    assert_eq!(answer[answer.len() - 1], 0, "plain");
    send(o, pet_command(serial, CUTE)).await;
    let answer = next(o, &mut t, SendOpcode::PetCommand).await;
    assert_eq!(answer[4..6], [0, 0], "Baby Tiger's command");
    assert_eq!(answer[answer.len() - 1], 1, "its quote ring");
    send(o, item_move(-1, FIRST_PET_LABEL, BAG_SLOT)).await;
    assert_eq!(next(o, &mut t, SendOpcode::PetNameChange).await, tiger(0));
    send(o, item_move(1, BAG_SLOT, SECOND_PET_LABEL)).await;
    let tigger = |tagged| renamed(owner_id, 1, "Tigger", tagged);
    assert_eq!(next(o, &mut t, SendOpcode::PetNameChange).await, tigger(1));

    // Sending the first pet back moves Tigger to the first label slot, which
    // is empty.
    send(o, spawn_pet(1)).await;
    assert_eq!(
        next(o, &mut t, SendOpcode::PetNameChange).await,
        renamed(owner_id, 0, "Tigger", 0)
    );
    // And its answers now take the first slot's quote ring.
    send(o, pet_command(serial + 1, CUTE)).await;
    let answer = next(o, &mut t, SendOpcode::PetCommand).await;
    assert_eq!(answer[4..6], [0, 0], "Tigger is first");
    assert_eq!(answer[answer.len() - 1], 1, "the quote ring");
}
