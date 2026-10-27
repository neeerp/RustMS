//! An Aran's wolf (21600-21618): Pucci sends the Aran to Nanuke, Scadur
//! makes a saddle, the Field of Wolves is run to its end for Scadur's
//! Advice, the pack's wolf tests the Aran against its wolves (and lets it
//! try again), and Maha wakes the wolf's power.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{
    grant_item, replace_item, set_job, set_quest, set_str, set_weapon_attack,
};
use integration_harness::events::{maps, talk};
use integration_harness::hunting::{attack, walk_to};
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

const PUCCIS_MAP: i32 = 140020110;
const PUCCI: i32 = 1202007;
const EL_NATH: i32 = 211000000;
const SCADUR: i32 = 2020007;
const WOLF_SHAPED_ROCK: i32 = 211050000;
const FIELD_OF_WOLVES: i32 = 921110000;
const WOLVES_DEN: i32 = 140010210;
const PACK_WOLF: i32 = 1202008;
const TEST_ROOM: i32 = 914030000;
const TEST_WOLF: i32 = 9300354;
const RIEN: i32 = 140000000;
const MAHA: i32 = 1201002;
const PUCCIS_REQUEST: i16 = 21600;
const WOLF_MOUNT: i16 = 21604;
const SCADURS_ADVICE: i16 = 21610;
const SCADURS_ADVICE_RECORD: i16 = 21619;
const THE_WOLVES_TEST: i16 = 21613;
const WOLVES_TEST_RECORD: i16 = 21620;
const THE_WOLFS_EVOLUTION: i16 = 21618;
const WOLF: i32 = 1902015;
const WOLF_MOUNT_ITEM: i32 = 1902017;
const EVOLVED_WOLF_MOUNT: i32 = 1902018;

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

/// Walk to NPC `npc` on `map` and send `action` to it.
async fn quest(c: &mut MapleTestConnection, m: &str, map: i32, npc: i32, action: Packet) -> Trace {
    send(c, to(map, npc)).await;
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

/// Going through a scripted portal.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// Whether `trace` has a pink line.
fn pink(trace: &Trace) -> bool {
    trace.broadcasts.iter().any(|(kind, _)| *kind == 5)
}

/// Into the test room, until its 30 wolves are out.
async fn into_the_test_room(c: &mut MapleTestConnection) -> Trace {
    let mut trace = Trace::default();
    until(c, &mut trace, |t| {
        t.warps == [TEST_ROOM] && t.field.alive(TEST_WOLF).len() == 30
    })
    .await;
    trace
}

#[tokio::test]
async fn an_aran_rides_and_raises_a_wolf() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 2112, 200, 50).await.unwrap();
    replace_item(id, -1, -11, 1442000).await.unwrap();
    set_weapon_attack(id, 500).await.unwrap();
    set_str(id, 999).await.unwrap();
    grant_item(id, 1, 1, WOLF).await.unwrap();
    grant_item(id, 1, 2, WOLF_MOUNT_ITEM).await.unwrap();
    for quest_id in [21500, 21612, 21614] {
        set_quest(id, quest_id, 2, "").await.unwrap();
    }
    set_quest(id, 21607, 1, "").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    // Pucci sends the Aran to Nanuke.
    assert_eq!(warp(c, PUCCIS_MAP, "", &mut trace).await, PUCCIS_MAP);
    let t = quest(
        c,
        &m,
        PUCCIS_MAP,
        PUCCI,
        build_quest_action(SCRIPT_START, PUCCIS_REQUEST, PUCCI),
    )
    .await;
    assert_eq!(t.prompts, [(PUCCI, 0)]);
    let t = next(c, &m, 1).await;
    assert_eq!(t.prompts, [(PUCCI, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.states.get(&PUCCIS_REQUEST), Some(&1));
    next(c, &m, 1).await;

    // Scadur will make a saddle.
    assert_eq!(warp(c, EL_NATH, "", &mut trace).await, EL_NATH);
    let t = quest(
        c,
        &m,
        EL_NATH,
        SCADUR,
        build_quest_action(SCRIPT_START, WOLF_MOUNT, SCADUR),
    )
    .await;
    assert_eq!(t.prompts, [(SCADUR, 0)]);
    let t = next(c, &m, 2).await;
    assert_eq!(t.states.get(&WOLF_MOUNT), Some(&1));

    // Scadur's Advice: through the Field of Wolves to its end.
    let t = step(c, &m, build_quest_action(START, SCADURS_ADVICE, SCADUR)).await;
    assert_eq!(t.states.get(&SCADURS_ADVICE), Some(&1));
    assert_eq!(
        warp(c, WOLF_SHAPED_ROCK, "", &mut trace).await,
        WOLF_SHAPED_ROCK
    );
    step(c, &m, walk_to(984, -640)).await;
    let t = step(c, &m, enter("in00")).await;
    assert_eq!(t.warps, [FIELD_OF_WOLVES]);
    step(c, &m, walk_to(6875, -71)).await;
    let t = step(c, &m, enter("clear00")).await;
    assert_eq!(t.kills[&SCADURS_ADVICE_RECORD], "0");
    assert!(pink(&t), "{t:?}");
    // Run once, the field stays shut.
    assert_eq!(
        warp(c, WOLF_SHAPED_ROCK, "", &mut trace).await,
        WOLF_SHAPED_ROCK
    );
    step(c, &m, walk_to(984, -640)).await;
    let t = step(c, &m, enter("in00")).await;
    assert!(t.warps.is_empty() && pink(&t), "{t:?}");
    assert_eq!(warp(c, EL_NATH, "", &mut trace).await, EL_NATH);
    let t = quest(
        c,
        &m,
        EL_NATH,
        SCADUR,
        build_quest_completion(SCADURS_ADVICE, SCADUR, -1),
    )
    .await;
    assert_eq!(t.states.get(&SCADURS_ADVICE), Some(&2));

    // The Wolves' Test: the wolf sends the Aran in among its wolves.
    assert_eq!(warp(c, WOLVES_DEN, "", &mut trace).await, WOLVES_DEN);
    let t = quest(
        c,
        &m,
        WOLVES_DEN,
        PACK_WOLF,
        build_quest_action(SCRIPT_START, THE_WOLVES_TEST, PACK_WOLF),
    )
    .await;
    assert_eq!(t.prompts, [(PACK_WOLF, 0)]);
    let t = next(c, &m, 2).await;
    assert_eq!(t.prompts, [(PACK_WOLF, 1)]);
    send(c, answer(1, 1, None)).await;
    let t = into_the_test_room(c).await;
    assert_eq!(t.states.get(&THE_WOLVES_TEST), Some(&1));
    let t = step(c, &m, enter("out00")).await;
    assert!(t.warps.is_empty() && pink(&t), "{t:?}");

    // Out without passing, the wolf lets the Aran try again, and the
    // room has its wolves back.
    assert_eq!(warp(c, WOLVES_DEN, "", &mut trace).await, WOLVES_DEN);
    let t = quest(c, &m, WOLVES_DEN, PACK_WOLF, talk(WOLVES_DEN, PACK_WOLF)).await;
    assert_eq!(t.prompts, [(PACK_WOLF, 1)]);
    send(c, answer(1, 1, None)).await;
    let mut trace = into_the_test_room(c).await;
    for wolf in trace.field.alive(TEST_WOLF) {
        for _ in 0..20 {
            send(c, attack(wolf, 30_000)).await;
            read_for(c, &mut trace, 100).await;
            if trace.field.killed.contains(&wolf) {
                break;
            }
        }
    }
    assert_eq!(
        trace.field.alive(TEST_WOLF),
        Vec::<i32>::new(),
        "wolves left"
    );
    let t = step(c, &m, enter("out00")).await;
    assert_eq!(t.warps, [WOLVES_DEN]);
    assert_eq!(t.kills[&WOLVES_TEST_RECORD], "0");
    let t = quest(
        c,
        &m,
        WOLVES_DEN,
        PACK_WOLF,
        build_quest_completion(THE_WOLVES_TEST, PACK_WOLF, -1),
    )
    .await;
    assert_eq!(t.states.get(&THE_WOLVES_TEST), Some(&2));

    // Maha wakes the wolf's power.
    let mut trace = Trace::default();
    assert_eq!(warp(c, RIEN, "", &mut trace).await, RIEN);
    let t = quest(
        c,
        &m,
        RIEN,
        MAHA,
        build_quest_action(SCRIPT_START, THE_WOLFS_EVOLUTION, MAHA),
    )
    .await;
    assert_eq!(t.prompts, [(MAHA, 0)]);
    let t = next(c, &m, 2).await;
    assert_eq!(t.prompts, [(MAHA, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.states.get(&THE_WOLFS_EVOLUTION), Some(&1));
    let t = step(
        c,
        &m,
        build_quest_action(SCRIPT_END, THE_WOLFS_EVOLUTION, MAHA),
    )
    .await;
    assert_eq!(t.prompts, [(MAHA, 0)]);
    let t = next(c, &m, 1).await;
    assert_eq!(t.items, [(WOLF_MOUNT_ITEM, -1), (EVOLVED_WOLF_MOUNT, 1)]);
    assert_eq!(t.states.get(&THE_WOLFS_EVOLUTION), Some(&2));
}
