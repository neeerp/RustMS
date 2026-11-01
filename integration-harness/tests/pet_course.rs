//! Ludibrium's Pet Walkway: Weaver at its foot gives a letter, and Neru
//! at its top takes it and brings the pet out 4 points closer, which levels
//! a new pet up.

use integration_harness::database::grant_pet;
use integration_harness::events::{talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::npc_scripts::{LUDIBRIUM_PET_COURSE, NERU, WEAVER};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const PET_WALKWAY: i32 = 220000006;
const PUPPY: i32 = 5_000_001;
const CASH: u8 = 5;

/// SPAWN_PET: the pet in Cash slot `slot` comes out.
fn spawn_pet(slot: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::SpawnPet as i16).unwrap();
    p.write_int(0).unwrap(); // time
    p.write_short(slot).unwrap();
    p.write_byte(0).unwrap(); // not leading
    p
}

#[tokio::test]
async fn neru_takes_weavers_letter_and_brings_the_pet_closer() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let serial = 9_220_000_000 + i64::from(id);
    grant_pet(id, 1, PUPPY, serial, "Puppy").await.unwrap();
    let letter = LUDIBRIUM_PET_COURSE.letter;

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(
        warp(c, PET_WALKWAY, "", &mut Trace::default()).await,
        PET_WALKWAY
    );
    step(c, &m, spawn_pet(1)).await;

    // Weaver gives the letter.
    let asked = told(&mut session, talk(PET_WALKWAY, WEAVER)).await;
    assert_eq!(asked.prompts, [(WEAVER, 1)], "{asked:?}");
    let mut seen = told(&mut session, answer(1, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.items.contains(&(letter, 1))).await;

    // Neru takes it, and the pet comes closer.
    let thanks = told(&mut session, talk(PET_WALKWAY, NERU)).await;
    assert_eq!(thanks.prompts, [(NERU, 0)], "{thanks:?}");
    let mut seen = told(&mut session, answer(0, 1, None)).await;
    let c = &mut session.connection;
    until(c, &mut seen, |t| t.items.contains(&(letter, -1))).await;
    until(c, &mut seen, |t| t.pet_levels == [0]).await;
    assert!(seen.added.contains(&(CASH, 1, PUPPY)), "{:?}", seen.added);
}
