//! Trainer Bartos's Robo pet lessons: Educating Smart Pets (4660, recall;
//! 4661, auto speaking) need the first pet out to lack the skill they teach
//! (`petRecallLimit`, `petAutoSpeakingLimit`), and teach it with `petskill`.
//! The pet's item is re-sent with its new skills.

use integration_harness::database::{grant_pet, grant_stack, pet_skills, set_job};
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

const PET_PARK: i32 = 100_000_202;
const BARTOS: i32 = 1_012_006;
const RECALL: i16 = 4660;
const AUTO_SPEAKING: i16 = 4661;
const ROBO: i32 = 5_000_048;
const JUICE: i32 = 4_031_993;
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
async fn bartos_teaches_a_robo_each_skill_once() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 10, 50).await.unwrap();
    let serial = 9_210_460_000 + i64::from(id);
    grant_pet(id, 1, ROBO, serial, "Robo").await.unwrap();
    grant_stack(id, 4, 1, JUICE, 2).await.unwrap();

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, PET_PARK, "", &mut Trace::default()).await, PET_PARK);
    let field = maps().field(PET_PARK).unwrap();
    let bartos = field.map_npcs.iter().find(|n| n.npc_id == BARTOS).unwrap();
    step(c, &m, walk_to(bartos.x, bartos.y)).await;

    // No pet out: nothing starts.
    let t = step(c, &m, build_quest_action(START, AUTO_SPEAKING, BARTOS)).await;
    assert!(t.progress.is_empty(), "{t:?}");
    step(c, &m, spawn_pet(1)).await;
    let t = step(c, &m, build_quest_action(START, AUTO_SPEAKING, BARTOS)).await;
    assert_eq!(t.progress, [(AUTO_SPEAKING, 1)]);
    let t = step(c, &m, build_quest_completion(AUTO_SPEAKING, BARTOS, -1)).await;
    assert_eq!(t.progress, [(AUTO_SPEAKING, 2)]);
    assert_eq!(t.items, [(JUICE, -1)]);
    assert!(t.removed.contains(&(CASH, 1)), "{t:?}");
    assert!(t.added.contains(&(CASH, 1, ROBO)), "{t:?}");

    // It speaks now, so 4661 won't start again; recall is still to learn.
    let t = step(c, &m, build_quest_action(START, AUTO_SPEAKING, BARTOS)).await;
    assert!(t.progress.is_empty(), "{t:?}");
    let t = step(c, &m, build_quest_action(START, RECALL, BARTOS)).await;
    assert_eq!(t.progress, [(RECALL, 1)]);
    let t = step(c, &m, build_quest_completion(RECALL, BARTOS, -1)).await;
    assert_eq!(t.progress, [(RECALL, 2)]);
    assert!(t.added.contains(&(CASH, 1, ROBO)), "{t:?}");
    drop(session);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Both skills were saved with the pet.
    assert_eq!(pet_skills(serial).await.unwrap(), (0x180, 0));
}
