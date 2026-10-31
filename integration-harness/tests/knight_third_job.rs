//! A Knight's third job (b310): at level 70, after The Master of
//! Disguise, an official Knight's Chief Knight tells of Shinsoo's Teardrop
//! (20311-20315), and the yes makes an advanced Knight and completes the
//! quest.

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
const NIGHT_WALKER_CHIEF: i32 = 1101006;
const MASTER_OF_DISGUISE: i16 = 20304;
const TEARDROP: i16 = 20314;
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
async fn shinsoos_teardrop_makes_a_night_walker_of_70_an_advanced_knight() {
    let config = load_harness_config_or_fail().await.with_class(CYGNUS);
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 1410, 70, 50)
        .await
        .expect("a level 70 Night Walker");
    set_sp(id, 0).await.expect("no SP left");
    complete_quests(id, MASTER_OF_DISGUISE, MASTER_OF_DISGUISE)
        .await
        .expect("an official Knight");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, EREVE, "", &mut t).await, EREVE);

    let chief = NIGHT_WALKER_CHIEF;
    let start = build_quest_action(SCRIPT_START, TEARDROP, chief);
    let t = quest(c, &m, chief, start).await;
    assert_eq!(t.prompts, [(chief, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert_eq!(t.prompts, [(chief, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.stat_values(JOB), [1411]);
    assert_eq!(t.states.get(&TEARDROP), Some(&2));
    assert_eq!(t.prompts, [(chief, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.prompts.is_empty());
    drop(session);

    let after = relogin(&config).await;
    assert_eq!(after.job, 1411);
}
