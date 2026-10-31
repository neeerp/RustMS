//! An Aran's second job (row 191): at level 30 Lilin hears the Polearm
//! calling (21200), Maha wants to be remembered (21201), and the Mirror of
//! Desire shows the past, where Sir Blacksmith asks for 30 Signs of
//! Acceptance (21202); told of it, Maha makes the Aran a 2110 with the Aran
//! in Memory medal. The mirror leads into the first pair of rooms nobody is
//! in.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{grant_stack, set_job};
use integration_harness::events::{maps, waiting_as};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, SCRIPT_END, SCRIPT_START, START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const RIEN: i32 = 140000000;
const MIRROR_CAVE: i32 = 140030000;
const MIRROR_ROOM: i32 = 108000700;
const LILIN: i32 = 1201000;
const MAHA: i32 = 1201002;
const SIR_BLACKSMITH: i32 = 1203000;
const POLEARM_CALLING: i16 = 21200;
const MIRROR_OF_DESIRE: i16 = 21201;
const ULTIMATE_WEAPON: i16 = 21202;
const MIRROR_RECORD: i16 = 21203;
const SIGN_OF_ACCEPTANCE: i32 = 4032311;
const ARAN_2_MEDAL: i32 = 1142130;
const JOB: i32 = 0x20;

/// Walk to NPC `npc` on `map`, or a copy of it, and send `action`.
async fn quest(c: &mut MapleTestConnection, m: &str, map: i32, npc: i32, action: Packet) -> Trace {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    send(c, walk_to(at.x, at.y)).await;
    barrier(c, m, &mut Trace::default()).await;
    step(c, m, action).await
}

/// Click through `n` say pages.
async fn next(c: &mut MapleTestConnection, m: &str, n: usize) -> Trace {
    let mut t = Trace::default();
    for _ in 0..n {
        t = step(c, m, answer(0, 1, None)).await;
    }
    t
}

/// Going through a scripted portal standing at `at`.
async fn enter(c: &mut MapleTestConnection, m: &str, portal: &str, at: (i16, i16)) -> Trace {
    step(c, m, walk_to(at.0, at.1)).await;
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    step(c, m, p).await
}

#[tokio::test]
async fn maha_remembered_through_the_mirror_makes_a_second_job_aran() {
    // Someone is already in the first room of the past.
    let _occupant = waiting_as(MIRROR_ROOM, 30, 2100).await;

    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 2100, 30, 50).await.expect("an Aran of 30");
    grant_stack(id, 4, 1, SIGN_OF_ACCEPTANCE, 30)
        .await
        .expect("the Signs");

    let mut before = relogin(&config).await;
    let c = &mut before.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, RIEN, "", &mut t).await, RIEN);

    // Lilin hears the Polearm calling; Maha ends it.
    let start = build_quest_action(SCRIPT_START, POLEARM_CALLING, LILIN);
    let t = quest(c, &m, RIEN, LILIN, start).await;
    assert_eq!(t.prompts, [(LILIN, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.states.get(&POLEARM_CALLING), Some(&1));
    assert!(next(c, &m, 1).await.prompts.is_empty());
    let end = build_quest_action(SCRIPT_END, POLEARM_CALLING, MAHA);
    let t = quest(c, &m, RIEN, MAHA, end).await;
    assert_eq!(t.prompts, [(MAHA, 0)]);
    let t = next(c, &m, 6).await;
    assert_eq!(t.prompts, [(MAHA, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.states.get(&POLEARM_CALLING), Some(&2));
    assert!(next(c, &m, 1).await.prompts.is_empty());

    // Lilin starts The Mirror of Desire from the data.
    let start = build_quest_action(START, MIRROR_OF_DESIRE, LILIN);
    let t = quest(c, &m, RIEN, LILIN, start).await;
    assert_eq!(t.states.get(&MIRROR_OF_DESIRE), Some(&1));

    // The mirror passes over the room taken and sets the record.
    let mut t = Trace::default();
    assert_eq!(warp(c, MIRROR_CAVE, "", &mut t).await, MIRROR_CAVE);
    let t = enter(c, &m, "in00", (-3, 122)).await;
    assert_eq!(t.warps, [MIRROR_ROOM + 1]);
    assert_eq!(t.kills[&MIRROR_RECORD], "0");

    // Sir Blacksmith asks for the Signs and takes them all.
    let start = build_quest_action(SCRIPT_START, ULTIMATE_WEAPON, SIR_BLACKSMITH);
    let t = quest(c, &m, MIRROR_ROOM, SIR_BLACKSMITH, start).await;
    assert_eq!(t.prompts, [(SIR_BLACKSMITH, 0)]);
    let t = next(c, &m, 4).await;
    assert_eq!(t.prompts, [(SIR_BLACKSMITH, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.states.get(&ULTIMATE_WEAPON), Some(&1));
    assert!(next(c, &m, 1).await.prompts.is_empty());
    let end = build_quest_action(SCRIPT_END, ULTIMATE_WEAPON, SIR_BLACKSMITH);
    let t = step(c, &m, end).await;
    assert_eq!(t.prompts, [(SIR_BLACKSMITH, 0)]);
    let t = next(c, &m, 2).await;
    assert_eq!(t.prompts, [(SIR_BLACKSMITH, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.states.get(&ULTIMATE_WEAPON), Some(&2));
    assert_eq!(t.removed, [(4, 1)]);

    // The room's west way leads to its own bears' room, and back.
    let t = enter(c, &m, "west00", (-1301, 358)).await;
    assert_eq!(t.warps, [MIRROR_ROOM + 11]);
    let t = enter(c, &m, "east00", (852, 359)).await;
    assert_eq!(t.warps, [MIRROR_ROOM + 1]);

    // Told of it, Maha makes a second-job Aran.
    let mut t = Trace::default();
    assert_eq!(warp(c, RIEN, "", &mut t).await, RIEN);
    let end = build_quest_action(SCRIPT_END, MIRROR_OF_DESIRE, MAHA);
    let t = quest(c, &m, RIEN, MAHA, end).await;
    assert_eq!(t.prompts, [(MAHA, 0)]);
    let t = next(c, &m, 4).await;
    assert_eq!(t.prompts, [(MAHA, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.stat_values(JOB), [2110]);
    assert_eq!(t.states.get(&MIRROR_OF_DESIRE), Some(&2));
    assert_eq!(
        t.added.iter().map(|a| (a.0, a.2)).collect::<Vec<_>>(),
        [(1, ARAN_2_MEDAL)]
    );
    assert!(next(c, &m, 1).await.prompts.is_empty());
    drop(before);

    let after = relogin(&config).await;
    assert_eq!(after.job, 2110);
    assert_eq!(after.items.get(&ARAN_2_MEDAL), Some(&1));
    assert_eq!(after.items.get(&SIGN_OF_ACCEPTANCE), None);
}
