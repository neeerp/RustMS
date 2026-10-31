//! A Knight's fourth job (b310): at level 120, after Chasing the Knight's
//! Target, Cygnus thanks an Advanced Knight (20408), and the yes makes a
//! Chief Knight and completes the quest.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{complete_quests, set_job, set_sp};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const CYGNUS: i32 = 0;
const EREVE: i32 = 130_000_000;
const CYGNUS_NPC: i32 = 1101000;
const BLACK_WITCH: i16 = 20407;
const CHIEF_KNIGHT: i16 = 20408;
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
async fn cygnus_makes_a_thunder_breaker_of_120_a_chief_knight() {
    let config = load_harness_config_or_fail().await.with_class(CYGNUS);
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 1511, 120, 50)
        .await
        .expect("a level 120 Thunder Breaker");
    set_sp(id, 0).await.expect("no SP left");
    complete_quests(id, BLACK_WITCH, BLACK_WITCH)
        .await
        .expect("the witch beaten");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, EREVE, "", &mut t).await, EREVE);

    let chief = CYGNUS_NPC;
    let start = build_quest_action(SCRIPT_START, CHIEF_KNIGHT, chief);
    let t = quest(c, &m, chief, start).await;
    assert_eq!(t.prompts, [(chief, 0)]);
    step(c, &m, answer(0, 1, None)).await;
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(chief, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.stat_values(JOB), [1512]);
    assert_eq!(t.states.get(&CHIEF_KNIGHT), Some(&2));
    assert_eq!(t.prompts, [(chief, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.prompts.is_empty());
    drop(session);

    let after = relogin(&config).await;
    assert_eq!(after.job, 1512);
}
