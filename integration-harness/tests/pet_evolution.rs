//! The Rock of Evolution: Garnox evolves the first pet out, once it is close
//! enough, for a rock and 10,000 mesos. "Baby Dragon's Evolution" (8185)
//! turns a Baby Dragon into one of the four dragons in place; the pet keeps
//! its serial, comes back out as its new kind, and stays that kind.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{
    grant_pet, grant_stack, set_meso, set_pet_closeness, set_quest,
};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END, START};
use integration_harness::trace::{stat, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

/// New Leaf City, where Garnox is.
const NLC: i32 = 600_000_000;
const GARNOX: i32 = 9_102_001;
const BABY_DRAGON_QUEST: i16 = 8185;
/// "Baby Dragon's Growth", which 8185 follows.
const GROWTH: i16 = 8184;
const BABY_DRAGON: i32 = 5_000_029;
const DRAGONS: std::ops::RangeInclusive<i32> = 5_000_030..=5_000_033;
const ROCK: i32 = 5_380_000;
const CASH: u8 = 5;

/// Walk up to Garnox.
async fn walk_up(c: &mut MapleTestConnection, m: &str) {
    let field = maps().field(NLC).unwrap();
    let spot = field.map_npcs.iter().find(|n| n.npc_id == GARNOX).unwrap();
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
async fn garnox_evolves_a_baby_dragon() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let serial = 9_380_000_000 + i64::from(id);
    grant_pet(id, 1, BABY_DRAGON, serial, "Baby Dragon")
        .await
        .expect("a Baby Dragon");
    set_pet_closeness(serial, 1641).await.expect("closeness");
    set_quest(id, GROWTH, 2, "").await.expect("8184 done");
    grant_stack(id, i16::from(CASH), 2, ROCK, 1)
        .await
        .expect("a rock");
    set_meso(id, 10_000).await.expect("the fee");

    // One point short of the closeness the quest wants, it doesn't start.
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, NLC, "", &mut Trace::default()).await, NLC);
    walk_up(c, &m).await;
    step(c, &m, spawn_pet(1)).await;
    let t = step(c, &m, build_quest_action(START, BABY_DRAGON_QUEST, GARNOX)).await;
    assert!(t.progress.is_empty(), "not close enough");
    drop(session);

    tokio::time::sleep(Duration::from_millis(500)).await;
    set_pet_closeness(serial, 1642).await.expect("closeness");
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    if session.map_id != NLC {
        assert_eq!(warp(c, NLC, "", &mut Trace::default()).await, NLC);
    }
    walk_up(c, &m).await;
    let t = step(c, &m, build_quest_action(START, BABY_DRAGON_QUEST, GARNOX)).await;
    assert_eq!(t.progress, [(BABY_DRAGON_QUEST, 1)]);

    // Garnox asks, then takes the rock and the fee and evolves the pet in
    // its slot; it comes back out as a dragon.
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_END, BABY_DRAGON_QUEST, GARNOX),
    )
    .await;
    assert_eq!(t.prompts, [(GARNOX, 1)], "asks first");
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.items, [(ROCK, -1)]);
    assert_eq!(t.stat(stat::MESO), Some(0));
    let (_, _, into) = *t.added.iter().find(|a| a.1 == 1).expect("evolved");
    assert!(DRAGONS.contains(&into), "{into} is a dragon");
    assert!(t.removed.contains(&(CASH, 1)));
    assert_eq!(t.pets_gone.len(), 1);
    let (owner, index, item_id, ref name, out) = t.pets[0];
    assert_eq!((owner, index, item_id, out), (id, 0, into, serial));
    assert_ne!(name, "Baby Dragon", "the name follows the kind");
    assert_eq!(t.progress, [(BABY_DRAGON_QUEST, 2)]);
    assert_eq!(t.prompts, [(GARNOX, 0)], "says it worked");
    step(c, &m, answer(0, 1, None)).await;
    drop(session);

    // It is that dragon for good.
    tokio::time::sleep(Duration::from_millis(500)).await;
    let restored = relogin(&config).await;
    assert_eq!(restored.items.get(&into), Some(&1));
    assert_eq!(restored.items.get(&BABY_DRAGON), None);
    assert_eq!(restored.items.get(&ROCK), None);
    assert_eq!(restored.quests.get(&BABY_DRAGON_QUEST), Some(&2));
}
