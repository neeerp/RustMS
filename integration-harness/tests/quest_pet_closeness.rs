//! A quest's `pettameness`: "Fanzy's Red Furball" (2104), finished with
//! Fanzy in the Ellinia forest, makes the first pet out 15 points closer.
//! The pet's item is re-sent with its new closeness, and a pet that new
//! levels up on the spot.

use integration_harness::database::{grant_pet, grant_stack, set_quest};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{build_quest_action, build_quest_completion, START};
use integration_harness::trace::{step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

const FOREST: i32 = 101_020_000;
const FANZY: i32 = 1_040_002;
const FURBALL: i16 = 2104;
/// "Nella's Dream", which 2104 follows.
const BEFORE: i16 = 2103;
const PUPPY: i32 = 5_000_001;
/// The red furball, which Fanzy takes back.
const FURBALL_ITEM: i32 = 4_031_273;
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
async fn finishing_fanzys_quest_brings_the_pet_closer() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let serial = 9_210_400_000 + i64::from(id);
    grant_pet(id, 1, PUPPY, serial, "Puppy")
        .await
        .expect("a pet");
    set_quest(id, BEFORE, 2, "").await.expect("2103 done");
    grant_stack(id, 4, 1, FURBALL_ITEM, 1)
        .await
        .expect("the proof");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, FOREST, "", &mut Trace::default()).await, FOREST);
    let field = maps().field(FOREST).unwrap();
    let fanzy = field.map_npcs.iter().find(|n| n.npc_id == FANZY).unwrap();
    step(c, &m, walk_to(fanzy.x, fanzy.y)).await;
    step(c, &m, spawn_pet(1)).await;
    let t = step(c, &m, build_quest_action(START, FURBALL, FANZY)).await;
    assert_eq!(t.progress, [(FURBALL, 1)]);

    let t = step(c, &m, build_quest_completion(FURBALL, FANZY, -1)).await;
    assert_eq!(t.progress, [(FURBALL, 2)]);
    assert!(t.items.contains(&(FURBALL_ITEM, -1)), "{:?}", t.items);
    assert!(t.removed.contains(&(CASH, 1)), "{:?}", t.removed);
    assert!(t.added.contains(&(CASH, 1, PUPPY)), "{:?}", t.added);
    assert_eq!(t.pet_levels, [0], "the new pet levels up");
    assert_eq!(t.cleared, 1);
}
