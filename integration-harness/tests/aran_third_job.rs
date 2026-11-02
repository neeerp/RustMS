//! An Aran's third job (row 191): at level 70 Lilin sends the Aran back to
//! Maha (21300), who wants the Red Jade back from the Thief Crow past
//! Rien's eastern dock (21301). It gets away, so Lilin has one made anew
//! (21302); in the Mirror of Desire's past Yeti has lost one to the same
//! thief (21303), and with the Red Jade Maha makes the Aran a 2111 with the
//! Aran in Misery medal. Each thief's room is for one player at a time.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{
    grant_stack, replace_item, set_job, set_str, set_weapon_attack,
};
use integration_harness::events::{maps, waiting_as};
use integration_harness::hunting::{attack, pickup, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_END, SCRIPT_START, START,
};
use integration_harness::trace::{barrier, read_for, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const RIEN: i32 = 140000000;
const RIEN_EAST: i32 = 140020200;
const MIRROR_CAVE: i32 = 140030000;
const THIEF_CROWS_ROOM: i32 = 108010700;
const YETIS_ROOM: i32 = 108010701;
const YETIS_THIEF_ROOM: i32 = 108010702;
const LILIN: i32 = 1201000;
const MAHA: i32 = 1201002;
const YETI: i32 = 1203001;
const THIEF_CROW: i32 = 9001013;
const WEAPON_NEVER_LEAVES: i16 = 21300;
const CATCH_THAT_THIEF: i16 = 21301;
const MAKING_RED_JADE: i16 = 21302;
const FRIENDSHIP_WITH_YETI: i16 = 21303;
const MIRROR_RECORD: i16 = 21203;
const YETIS_JADE: i32 = 4032339;
const RED_JADE: i32 = 4032312;
const ARAN_3_MEDAL: i32 = 1142131;
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

/// Beat the one Thief Crow on the field, seen since `trace` began.
async fn beat_the_thief(c: &mut MapleTestConnection, trace: &mut Trace) {
    until(c, trace, |t| t.field.alive(THIEF_CROW).len() == 1).await;
    let thief = trace.field.alive(THIEF_CROW)[0];
    for _ in 0..20 {
        send(c, attack(thief, 30_000)).await;
        read_for(c, trace, 100).await;
        if trace.field.killed.contains(&thief) {
            return;
        }
    }
    panic!("the Thief Crow lived");
}

#[tokio::test]
async fn maha_with_the_red_jade_back_makes_a_third_job_aran() {
    // Someone is already in the thief's room in the mirror's past.
    let occupant = waiting_as(YETIS_THIEF_ROOM, 70, 2110).await;

    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 2110, 70, 50).await.expect("an Aran of 70");
    replace_item(id, -1, -11, 1442000).await.unwrap();
    set_weapon_attack(id, 500).await.unwrap();
    set_str(id, 999).await.unwrap();

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, RIEN, "", &mut t).await, RIEN);

    // Lilin sends the Aran to Maha, who starts Catch that Thief!.
    let start = build_quest_action(SCRIPT_START, WEAPON_NEVER_LEAVES, LILIN);
    let t = quest(c, &m, RIEN, LILIN, start).await;
    assert_eq!(t.prompts, [(LILIN, 0)]);
    let t = next(c, &m, 1).await;
    assert_eq!(t.prompts, [(LILIN, 12)]);
    let t = step(c, &m, answer(12, 1, None)).await;
    assert_eq!(t.states.get(&WEAPON_NEVER_LEAVES), Some(&1));
    assert!(next(c, &m, 1).await.prompts.is_empty());
    let done = build_quest_completion(WEAPON_NEVER_LEAVES, MAHA, -1);
    let t = quest(c, &m, RIEN, MAHA, done).await;
    assert_eq!(t.states.get(&WEAPON_NEVER_LEAVES), Some(&2));
    let start = build_quest_action(START, CATCH_THAT_THIEF, MAHA);
    let t = step(c, &m, start).await;
    assert_eq!(t.states.get(&CATCH_THAT_THIEF), Some(&1));

    // Past the dock's east end waits the thief.
    let mut t = Trace::default();
    assert_eq!(warp(c, RIEN_EAST, "", &mut t).await, RIEN_EAST);
    let mut t = enter(c, &m, "east00", (4769, 84)).await;
    assert_eq!(t.warps, [THIEF_CROWS_ROOM]);
    beat_the_thief(c, &mut t).await;
    assert_eq!(t.kills[&CATCH_THAT_THIEF], "001");

    // It got away with the jade; Maha despairs.
    let mut t = Trace::default();
    assert_eq!(warp(c, RIEN, "", &mut t).await, RIEN);
    let end = build_quest_action(SCRIPT_END, CATCH_THAT_THIEF, MAHA);
    let t = quest(c, &m, RIEN, MAHA, end).await;
    assert_eq!(t.prompts, [(MAHA, 0)]);
    let t = next(c, &m, 7).await;
    assert_eq!(t.states.get(&CATCH_THAT_THIEF), Some(&2));
    let start = build_quest_action(START, MAKING_RED_JADE, LILIN);
    let t = quest(c, &m, RIEN, LILIN, start).await;
    assert_eq!(t.states.get(&MAKING_RED_JADE), Some(&1));

    // The mirror lets one in at a time.
    let mut t = Trace::default();
    assert_eq!(warp(c, MIRROR_CAVE, "", &mut t).await, MIRROR_CAVE);
    let t = enter(c, &m, "in00", (-3, 122)).await;
    assert!(t.warps.is_empty());
    drop(occupant);
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let t = enter(c, &m, "in00", (-3, 122)).await;
    assert_eq!(t.warps, [YETIS_ROOM]);
    assert_eq!(t.kills[&MIRROR_RECORD], "1");

    // Yeti's jade was taken by the thief in the next room.
    let start = build_quest_action(SCRIPT_START, FRIENDSHIP_WITH_YETI, YETI);
    let t = quest(c, &m, YETIS_ROOM, YETI, start).await;
    assert_eq!(t.prompts, [(YETI, 0)]);
    let t = next(c, &m, 4).await;
    assert_eq!(t.states.get(&FRIENDSHIP_WITH_YETI), Some(&1));
    step(c, &m, walk_to(-210, 455)).await;
    let mut t = Trace::default();
    assert_eq!(warp(c, -1, "in00", &mut t).await, YETIS_THIEF_ROOM);
    beat_the_thief(c, &mut t).await;
    assert_eq!(t.kills[&FRIENDSHIP_WITH_YETI], "001");
    until(c, &mut t, |t| t.field.item_drop(YETIS_JADE).is_some()).await;
    let jade = t.field.item_drop(YETIS_JADE).unwrap();
    send(c, pickup(jade.object_id, jade.x, jade.y)).await;
    until(c, &mut t, |t| t.added.iter().any(|a| a.2 == YETIS_JADE)).await;
    step(c, &m, walk_to(476, 450)).await;
    let mut t = Trace::default();
    assert_eq!(warp(c, -1, "out00", &mut t).await, YETIS_ROOM);
    let done = build_quest_completion(FRIENDSHIP_WITH_YETI, YETI, -1);
    let t = quest(c, &m, YETIS_ROOM, YETI, done).await;
    assert_eq!(t.states.get(&FRIENDSHIP_WITH_YETI), Some(&2));
    drop(session);

    // A Red Jade made at the Maker, Maha makes a third-job Aran.
    tokio::time::sleep(Duration::from_millis(500)).await;
    grant_stack(id, 4, 1, RED_JADE, 1)
        .await
        .expect("the Red Jade");
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, RIEN, "", &mut t).await, RIEN);
    let end = build_quest_action(SCRIPT_END, MAKING_RED_JADE, MAHA);
    let t = quest(c, &m, RIEN, MAHA, end).await;
    assert_eq!(t.prompts, [(MAHA, 0)]);
    let t = next(c, &m, 2).await;
    assert_eq!(t.stat_values(JOB), [2111]);
    assert_eq!(t.states.get(&MAKING_RED_JADE), Some(&2));
    assert_eq!(t.removed, [(4, 1)]);
    assert_eq!(
        t.added.iter().map(|a| (a.0, a.2)).collect::<Vec<_>>(),
        [(1, ARAN_3_MEDAL)]
    );
    assert!(next(c, &m, 1).await.prompts.is_empty());
    drop(session);

    let after = relogin(&config).await;
    assert_eq!(after.job, 2111);
    assert_eq!(after.items.get(&ARAN_3_MEDAL), Some(&1));
    assert_eq!(after.items.get(&RED_JADE), None);
}
