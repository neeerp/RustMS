//! A Knight's second job (b310): at level 30 Neinheart's The End of
//! Knight-in-Training (20200) frees the Knighthood Exams, the way into a
//! 2nd Drill Hall takes the exam's tokens held so the count starts over,
//! its gatekeeper lets the player out, and 30 tokens brought to the Chief
//! Knight make an official Knight.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_stack, set_job, set_sp};
use integration_harness::events::{maps, talk};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END, SCRIPT_START, START};
use integration_harness::trace::{barrier, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const CYGNUS: i32 = 0;
const EREVE: i32 = 130_000_000;
const DRILL_HALL_ENTRANCE: i32 = 130_020_000;
const SECOND_DRILL_HALLS: [i32; 3] = [108_000_600, 108_000_601, 108_000_602];
const KIRIKO: i32 = 1102001;
const NEINHEART: i32 = 1101002;
const WIND_ARCHER_CHIEF: i32 = 1101005;
const END_OF_KNIGHT_IN_TRAINING: i16 = 20200;
const WIND_ARCHER_EXAM: i16 = 20203;
const TOKEN: i32 = 4032098;
/// Where the Etc tokens are put.
const SLOT: i16 = 20;
const JOB: i32 = 0x20;

/// Walk to NPC `npc` on Ereve and send a quest action to it.
async fn quest(c: &mut MapleTestConnection, m: &str, npc: i32, action: Packet) -> Trace {
    let npcs = &maps().field(EREVE).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    send(c, walk_to(at.x, at.y)).await;
    barrier(c, m, &mut Trace::default()).await;
    step(c, m, action).await
}

/// Enter the portal `name`'s script where the player stands.
fn enter(name: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(name).unwrap();
    p.write_short(0).unwrap();
    p
}

#[tokio::test]
async fn thirty_drill_hall_tokens_make_a_wind_archer_an_official_knight() {
    let config = load_harness_config_or_fail().await.with_class(CYGNUS);
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 1300, 31, 50)
        .await
        .expect("a level 31 Wind Archer");
    set_sp(id, 3).await.expect("3 SP left");
    grant_stack(id, 4, SLOT, TOKEN, 5)
        .await
        .expect("old tokens");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, EREVE, "", &mut t).await, EREVE);

    // Neinheart's yes completes The End of Knight-in-Training, which frees
    // the exam.
    let start = build_quest_action(SCRIPT_START, END_OF_KNIGHT_IN_TRAINING, NEINHEART);
    let t = quest(c, &m, NEINHEART, start).await;
    assert_eq!(t.prompts, [(NEINHEART, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.states.get(&END_OF_KNIGHT_IN_TRAINING), Some(&2));
    step(c, &m, answer(0, 1, None)).await;
    let start = build_quest_action(START, WIND_ARCHER_EXAM, WIND_ARCHER_CHIEF);
    let t = quest(c, &m, WIND_ARCHER_CHIEF, start).await;
    assert_eq!(t.states.get(&WIND_ARCHER_EXAM), Some(&1));

    // The way in takes the old tokens, and Kiriko lets the player out.
    let mut t = Trace::default();
    assert_eq!(
        warp(c, DRILL_HALL_ENTRANCE, "", &mut t).await,
        DRILL_HALL_ENTRANCE
    );
    send(c, walk_to(551, -330)).await;
    barrier(c, &m, &mut Trace::default()).await;
    let mut t = Trace::default();
    send(c, enter("in01")).await;
    until(c, &mut t, |t| !t.warps.is_empty()).await;
    assert!(SECOND_DRILL_HALLS.contains(&t.warps[0]), "{:?}", t.warps);
    assert!(t.removed.contains(&(4, SLOT)), "{:?}", t.removed);
    let hall = t.warps[0];
    let t = step(c, &m, talk(hall, KIRIKO)).await;
    assert_eq!(t.prompts, [(KIRIKO, 1)]);
    let mut t = Trace::default();
    send(c, answer(1, 1, None)).await;
    until(c, &mut t, |t| t.warps.contains(&DRILL_HALL_ENTRANCE)).await;
    drop(session);

    // 30 tokens to the Chief Knight make an official Wind Archer.
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, 4, SLOT, TOKEN, 30)
        .await
        .expect("30 tokens");
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, EREVE, "", &mut t).await, EREVE);
    let end = build_quest_action(SCRIPT_END, WIND_ARCHER_EXAM, WIND_ARCHER_CHIEF);
    let t = quest(c, &m, WIND_ARCHER_CHIEF, end).await;
    assert_eq!(t.prompts, [(WIND_ARCHER_CHIEF, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.stat_values(JOB), [1310]);
    assert_eq!(t.states.get(&WIND_ARCHER_EXAM), Some(&2));
    assert!(t.removed.contains(&(4, SLOT)), "{:?}", t.removed);
    assert_eq!(t.prompts, [(WIND_ARCHER_CHIEF, 0)]);
    for _ in 0..2 {
        step(c, &m, answer(0, 1, None)).await;
    }
    drop(session);

    let after = relogin(&config).await;
    assert_eq!(after.job, 1310);
    assert_eq!(after.items.get(&TOKEN), None);
}
