//! Pet label rings: a pet out shows its kind's name tag while a label ring
//! is worn in its part, in SPAWN_PET and in the PET_NAMECHANGE sent when the
//! ring comes off or goes back on.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_pet, wear_cash_equip};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;

const BABY_TIGER: i32 = 5_000_066;
const LABEL_RING: i32 = 1_822_000;
/// Where the first pet out wears its label ring.
const FIRST_PET_LABEL: i16 = -121;
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

#[tokio::test]
async fn a_label_ring_shows_its_pets_name_tag() {
    let config = load_harness_config_or_fail().await;
    let session = login_to_world_session(&config).await.unwrap();
    let owner_id = session.character_id;
    drop(session);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let serial = 9_182_000_000 + 2 * i64::from(owner_id);
    grant_pet(owner_id, 1, BABY_TIGER, serial, "Baby Tiger")
        .await
        .expect("grant a pet");
    wear_cash_equip(owner_id, FIRST_PET_LABEL, LABEL_RING, serial + 1)
        .await
        .expect("put the label ring on the pet");
    let mut owner = relogin(&config).await;
    let (o, mut t) = (&mut owner.connection, Trace::default());

    // It comes out with its name tag, and no chat balloon.
    send(o, spawn_pet(1)).await;
    let spawned = next(o, &mut t, SendOpcode::SpawnPet).await;
    assert_eq!(spawned[spawned.len() - 2..], [1, 0], "name tag, no balloon");

    let renamed = |tagged: u8| {
        let mut body = owner_id.to_le_bytes().to_vec();
        body.extend([0, 10, 0]); // pet 0, then the name's length
        body.extend(b"Baby Tiger");
        body.push(tagged);
        body
    };
    // Taking the ring off takes the tag away; putting it back shows it.
    send(o, item_move(-1, FIRST_PET_LABEL, BAG_SLOT)).await;
    assert_eq!(next(o, &mut t, SendOpcode::PetNameChange).await, renamed(0));
    send(o, item_move(1, BAG_SLOT, FIRST_PET_LABEL)).await;
    assert_eq!(next(o, &mut t, SendOpcode::PetNameChange).await, renamed(1));
}
