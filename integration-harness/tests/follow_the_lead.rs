//! Bartos's pet instructor test in Henesys: "Pet Instructor Test" (4646)
//! starts and ends only with a pet out, and needs the note hidden on the
//! Pet-Walking Road; "Bartos's Secret" (4647) then takes a Pet Snack and
//! teaches Follow the Lead.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_pet, grant_stack};
use integration_harness::events::{maps, talk};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_END, START,
};
use integration_harness::trace::{step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

/// The Pet-Walking Road, where Bartos and the hidden note are.
const ROAD: i32 = 100_000_202;
const BARTOS: i32 = 1_012_006;
const NOTE_SPOT: i32 = 9_102_100;
const TEST: i16 = 4646;
const SECRET: i16 = 4647;
const NOTE: i32 = 4_031_921;
const SNACK: i32 = 5_460_000;
const BROWN_KITTY: i32 = 5_000_000;
/// Follow the Lead for explorers.
const FOLLOW_THE_LEAD: i32 = 8;
const CASH: i16 = 5;

/// Walk up to `npc` on the road.
async fn walk_up(c: &mut MapleTestConnection, m: &str, npc: i32) {
    let field = maps().field(ROAD).unwrap();
    let spot = field.map_npcs.iter().find(|n| n.npc_id == npc).unwrap();
    step(c, m, walk_to(spot.x, spot.y)).await;
}

/// SPAWN_PET: the pet in Cash slot `slot` comes out, or goes back.
fn spawn_pet(slot: i16) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::SpawnPet as i16).unwrap();
    p.write_int(0).unwrap(); // time
    p.write_short(slot).unwrap();
    p.write_byte(0).unwrap(); // not leading
    p
}

#[tokio::test]
async fn bartos_teaches_follow_the_lead() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_pet(id, 1, BROWN_KITTY, 9_182_100_000 + i64::from(id), "Kitty")
        .await
        .expect("a pet");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, ROAD, "", &mut Trace::default()).await, ROAD);
    walk_up(c, &m, BARTOS).await;

    // The test starts only with a pet out.
    let t = step(c, &m, build_quest_action(START, TEST, BARTOS)).await;
    assert!(t.progress.is_empty(), "no pet out");
    step(c, &m, spawn_pet(1)).await;
    let t = step(c, &m, build_quest_action(START, TEST, BARTOS)).await;
    assert_eq!(t.progress, [(TEST, 1)]);
    // Nor does it end before the note is found.
    let t = step(c, &m, build_quest_completion(TEST, BARTOS, -1)).await;
    assert!(t.progress.is_empty(), "no note");

    // The note is found once.
    walk_up(c, &m, NOTE_SPOT).await;
    let t = step(c, &m, talk(ROAD, NOTE_SPOT)).await;
    assert_eq!(t.prompts, [(NOTE_SPOT, 1)], "asks to take the note");
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.items, [(NOTE, 1)]);
    let t = step(c, &m, talk(ROAD, NOTE_SPOT)).await;
    assert_eq!(t.prompts, [(NOTE_SPOT, 0)], "already held");
    step(c, &m, answer(0, 1, None)).await;

    // Without the pet out it still does not end; with it, Bartos takes the
    // note.
    walk_up(c, &m, BARTOS).await;
    step(c, &m, spawn_pet(1)).await;
    let t = step(c, &m, build_quest_completion(TEST, BARTOS, -1)).await;
    assert!(t.progress.is_empty(), "the pet went back");
    step(c, &m, spawn_pet(1)).await;
    let t = step(c, &m, build_quest_completion(TEST, BARTOS, -1)).await;
    assert_eq!(t.progress, [(TEST, 2)]);
    assert_eq!(t.items, [(NOTE, -1)]);

    // The secret wants a Pet Snack.
    let t = step(c, &m, build_quest_action(START, SECRET, BARTOS)).await;
    assert_eq!(t.progress, [(SECRET, 1)]);
    let t = step(c, &m, build_quest_action(SCRIPT_END, SECRET, BARTOS)).await;
    assert_eq!(t.prompts, [(BARTOS, 0)], "asks for a snack");
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.progress.is_empty() && t.skills.is_empty());
    drop(session);

    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, CASH, 2, SNACK, 1).await.expect("a snack");
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    walk_up(c, &m, BARTOS).await;
    let t = step(c, &m, build_quest_action(SCRIPT_END, SECRET, BARTOS)).await;
    assert_eq!(t.prompts, [(BARTOS, 0)], "tells the secret");
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.progress, [(SECRET, 2)]);
    assert_eq!(t.skills, [(FOLLOW_THE_LEAD, 1)]);
    assert_eq!(t.items, [(SNACK, -1)]);
    drop(session);

    let restored = relogin(&config).await;
    assert_eq!(restored.skills.get(&FOLLOW_THE_LEAD), Some(&1));
    for quest in [TEST, SECRET] {
        assert_eq!(restored.quests.get(&quest), Some(&2), "quest {quest}");
    }
}
