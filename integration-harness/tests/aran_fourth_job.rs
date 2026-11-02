//! An Aran's fourth job (row 191): at level 120 Lilin sends the Aran back
//! to Maha (21400), who asks to be stopped from going berserk (21401) and
//! sends the Aran into a battle with an enraged Maha, one player at a time.
//! Once it is beaten, Maha makes the Aran a 2112 with the Aran in Hope medal
//! and a Maple Hero book.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{replace_item, set_job, set_str, set_weapon_attack};
use integration_harness::events::{maps, talk, waiting_as};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_END, SCRIPT_START,
};
use integration_harness::trace::{barrier, read_for, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const RIEN: i32 = 140000000;
const MAHAS_BATTLE: i32 = 914020000;
const LILIN: i32 = 1201000;
const MAHA: i32 = 1201002;
const ENRAGED_MAHA: i32 = 9001014;
const POLEARM_STARTS_A_FIGHT: i16 = 21400;
const TAMING_THE_POLEARM: i16 = 21401;
const ARAN_4_MEDAL: i32 = 1142132;
const MAPLE_HERO: i32 = 2280003;
const JOB: i32 = 0x20;

/// Walk to NPC `npc` on `map` and send `action`.
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
async fn maha_tamed_makes_a_fourth_job_aran() {
    // Someone is already in Maha's battle.
    let occupant = waiting_as(MAHAS_BATTLE, 120, 2111).await;

    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 2111, 120, 50).await.expect("an Aran of 120");
    replace_item(id, -1, -11, 1442000).await.unwrap();
    set_weapon_attack(id, 1_999).await.unwrap();
    set_str(id, 32_000).await.unwrap();

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, RIEN, "", &mut t).await, RIEN);

    // Lilin sends the Aran to Maha, who ends it from the data.
    let start = build_quest_action(SCRIPT_START, POLEARM_STARTS_A_FIGHT, LILIN);
    let t = quest(c, &m, RIEN, LILIN, start).await;
    assert_eq!(t.prompts, [(LILIN, 12)]);
    let t = step(c, &m, answer(12, 1, None)).await;
    assert_eq!(t.states.get(&POLEARM_STARTS_A_FIGHT), Some(&1));
    assert!(next(c, &m, 1).await.prompts.is_empty());
    let done = build_quest_completion(POLEARM_STARTS_A_FIGHT, MAHA, -1);
    let t = quest(c, &m, RIEN, MAHA, done).await;
    assert_eq!(t.states.get(&POLEARM_STARTS_A_FIGHT), Some(&2));

    // Maha asks to be stopped, but someone is in the battle.
    let start = build_quest_action(SCRIPT_START, TAMING_THE_POLEARM, MAHA);
    let t = step(c, &m, start).await;
    assert_eq!(t.prompts, [(MAHA, 0)]);
    let t = next(c, &m, 3).await;
    assert_eq!(t.prompts, [(MAHA, 12)]);
    let t = step(c, &m, answer(12, 1, None)).await;
    assert!(t.warps.is_empty());
    assert_eq!(t.states.get(&TAMING_THE_POLEARM), None);
    next(c, &m, 1).await;
    drop(occupant);
    tokio::time::sleep(Duration::from_millis(1500)).await;

    // Asked again with the room empty, Maha sends the Aran in.
    let start = build_quest_action(SCRIPT_START, TAMING_THE_POLEARM, MAHA);
    step(c, &m, start).await;
    next(c, &m, 3).await;
    let t = step(c, &m, answer(12, 1, None)).await;
    assert_eq!(t.states.get(&TAMING_THE_POLEARM), Some(&1));
    assert_eq!(t.warps, [MAHAS_BATTLE]);
    assert_eq!(t.countdowns, [600]);

    // Leaving before it is beaten, talking to Maha leads back in.
    let t = enter(c, &m, "out00", (-19, 89)).await;
    assert_eq!(t.warps, [RIEN]);
    let t = quest(c, &m, RIEN, MAHA, talk(RIEN, MAHA)).await;
    assert_eq!(t.prompts, [(MAHA, 1)]);
    let mut t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.warps, [MAHAS_BATTLE]);

    // The enraged Maha falls.
    until(c, &mut t, |t| t.field.alive(ENRAGED_MAHA).len() == 1).await;
    let maha = t.field.alive(ENRAGED_MAHA)[0];
    for _ in 0..100 {
        if t.field.killed.contains(&maha) {
            break;
        }
        send(c, attack(maha, 999_999)).await;
        read_for(c, &mut t, 50).await;
    }
    until(c, &mut t, |t| t.field.killed.contains(&maha)).await;
    assert_eq!(t.kills[&TAMING_THE_POLEARM], "001");

    // The way out leads back to Rien.
    let t = enter(c, &m, "out00", (-19, 89)).await;
    assert_eq!(t.warps, [RIEN]);

    // Calmed, Maha makes a fourth-job Aran.
    let end = build_quest_action(SCRIPT_END, TAMING_THE_POLEARM, MAHA);
    let t = quest(c, &m, RIEN, MAHA, end).await;
    assert_eq!(t.prompts, [(MAHA, 0)]);
    let t = next(c, &m, 1).await;
    assert_eq!(t.prompts, [(MAHA, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.stat_values(JOB), [2112]);
    assert_eq!(t.states.get(&TAMING_THE_POLEARM), Some(&2));
    assert_eq!(
        t.added.iter().map(|a| (a.0, a.2)).collect::<Vec<_>>(),
        [(1, ARAN_4_MEDAL), (2, MAPLE_HERO)]
    );
    assert!(next(c, &m, 1).await.prompts.is_empty());
    drop(session);

    let after = relogin(&config).await;
    assert_eq!(after.job, 2112);
    assert_eq!(after.items.get(&ARAN_4_MEDAL), Some(&1));
    assert_eq!(after.items.get(&MAPLE_HERO), Some(&1));
}
