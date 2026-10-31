//! A Noblesse's first job (b310): at level 10 Neinheart's five paths
//! (20020) complete Time to Choose (20100), which frees the Knight
//! Commanders' quests, and a Commander's end script makes a Knight.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{set_job, set_str};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END, SCRIPT_START, START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const CYGNUS: i32 = 0;
const EREVE: i32 = 130_000_000;
const NEINHEART: i32 = 1101002;
const DAWN_WARRIOR_COMMANDER: i32 = 1101003;
const FIVE_PATHS: i16 = 20020;
const TIME_TO_CHOOSE: i16 = 20100;
const PATH_OF_A_DAWN_WARRIOR: i16 = 20101;
const SWORD: i32 = 1302077;
const JOB: i32 = 0x20;

/// Walk to NPC `npc` on Ereve and send a quest action to it.
async fn quest(c: &mut MapleTestConnection, m: &str, npc: i32, action: Packet) -> Trace {
    let npcs = &maps().field(EREVE).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    send(c, walk_to(at.x, at.y)).await;
    barrier(c, m, &mut Trace::default()).await;
    step(c, m, action).await
}

#[tokio::test]
async fn neinheart_and_a_commander_make_a_noblesse_of_10_a_dawn_warrior() {
    let config = load_harness_config_or_fail().await.with_class(CYGNUS);
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 1000, 10, 50).await.expect("level 10");
    set_str(id, 35).await.expect("strong");

    let mut before = relogin(&config).await;
    let c = &mut before.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, EREVE, "", &mut t).await, EREVE);

    // Neinheart's yes completes Time to Choose, and the end the five paths.
    let start = build_quest_action(SCRIPT_START, FIVE_PATHS, NEINHEART);
    let t = quest(c, &m, NEINHEART, start).await;
    assert_eq!(t.prompts, [(NEINHEART, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(NEINHEART, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.states.get(&TIME_TO_CHOOSE), Some(&2));
    assert_eq!(t.prompts, [(NEINHEART, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.states.get(&FIVE_PATHS), Some(&2));
    assert!(t.prompts.is_empty());

    // The Commander's quest starts, and its end makes a Dawn Warrior with
    // a sword and a row more of equipment and etc. slots.
    let commander = DAWN_WARRIOR_COMMANDER;
    let start = build_quest_action(START, PATH_OF_A_DAWN_WARRIOR, commander);
    let t = quest(c, &m, commander, start).await;
    assert_eq!(t.states.get(&PATH_OF_A_DAWN_WARRIOR), Some(&1));
    let end = build_quest_action(SCRIPT_END, PATH_OF_A_DAWN_WARRIOR, commander);
    let t = step(c, &m, end).await;
    assert_eq!(t.prompts, [(commander, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.stat_values(JOB), [1100]);
    assert_eq!(t.states.get(&PATH_OF_A_DAWN_WARRIOR), Some(&2));
    let grown = [1, 4].map(|kind| (kind, before.slot_limits[usize::from(kind) - 1] + 4));
    assert_eq!(t.bag_slots, grown);
    assert_eq!(
        t.added.iter().map(|a| (a.0, a.2)).collect::<Vec<_>>(),
        [(1, SWORD)]
    );
    assert_eq!(t.prompts, [(commander, 0)]);
    for _ in 0..4 {
        step(c, &m, answer(0, 1, None)).await;
    }
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.prompts.is_empty());
    drop(before);

    let after = relogin(&config).await;
    assert_eq!(after.job, 1100);
    assert_eq!(after.items.get(&SWORD), Some(&1));
}
