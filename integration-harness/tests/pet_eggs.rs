//! Pet eggs: summoning a Dragon Egg or a Robo Egg hatches it into its baby
//! in place, keeping the pet's serial and any name its owner gave it. The
//! baby comes out on the next summon, and is still a baby after a relog.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::grant_pet;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, until, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::{RecvOpcode, SendOpcode};
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

const DRAGON_EGG: i32 = 5_000_028;
const BABY_DRAGON: i32 = 5_000_029;
const ROBO_EGG: i32 = 5_000_047;
const BABY_ROBO: i32 = 5_000_048;
const CASH: u8 = 5;

/// SPAWN_PET: the pet in Cash slot `slot` comes out or goes back.
fn spawn_pet(slot: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::SpawnPet as i16).unwrap();
    p.write_int(0).unwrap(); // time
    p.write_short(slot).unwrap();
    p.write_byte(0).unwrap(); // not leading
    p
}

/// Read until the empty stat change that unlocks the client.
async fn unlocked(c: &mut MapleTestConnection, t: &mut Trace) {
    loop {
        let p = read(c, t).await;
        if p.opcode() == SendOpcode::StatChange as i16 && p.bytes[2..] == [1, 0, 0, 0, 0] {
            return;
        }
    }
}

#[tokio::test]
async fn an_egg_hatches_when_summoned() {
    let config = load_harness_config_or_fail().await;
    let session = login_to_world_session(&config).await.unwrap();
    let owner_id = session.character_id;
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let dragon = 9_300_000_000 + i64::from(owner_id);
    let robo = dragon + 1;
    grant_pet(owner_id, 1, DRAGON_EGG, dragon, "Dragon")
        .await
        .expect("grant a Dragon Egg");
    grant_pet(owner_id, 2, ROBO_EGG, robo, "Bolt")
        .await
        .expect("grant a Robo Egg");
    let mut owner = relogin(&config).await;
    let (o, mut t) = (&mut owner.connection, Trace::default());

    // The egg hatches in its slot and nothing comes out.
    send(o, spawn_pet(1)).await;
    unlocked(o, &mut t).await;
    assert_eq!(t.removed, [(CASH, 1)]);
    assert_eq!(t.added, [(CASH, 1, BABY_DRAGON)]);
    assert!(t.pets.is_empty(), "the egg doesn't come out");

    send(o, spawn_pet(2)).await;
    unlocked(o, &mut t).await;
    assert_eq!(t.added[1], (CASH, 2, BABY_ROBO));

    // The babies come out, named for their kind unless named already (the
    // second in place of the first, without Follow the Lead).
    send(o, spawn_pet(1)).await;
    until(o, &mut t, |t| !t.pets.is_empty()).await;
    assert_eq!(
        t.pets[0],
        (owner_id, 0, BABY_DRAGON, "Baby Dragon".to_string(), dragon)
    );
    send(o, spawn_pet(2)).await;
    until(o, &mut t, |t| t.pets.len() == 2).await;
    let (_, _, item_id, ref name, serial) = t.pets[1];
    assert_eq!((item_id, name.as_str(), serial), (BABY_ROBO, "Bolt", robo));

    // They are babies for good.
    drop(owner);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut owner = relogin(&config).await;
    let (o, mut t) = (&mut owner.connection, Trace::default());
    until(o, &mut t, |t| !t.pets.is_empty()).await;
    assert_eq!(t.pets[0].2, BABY_ROBO);
    send(o, spawn_pet(1)).await;
    until(o, &mut t, |t| t.pets.len() == 2).await;
    assert_eq!(t.pets[1].2, BABY_DRAGON);
}
