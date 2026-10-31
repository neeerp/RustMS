//! Matthias's mission (b310): a Knight of 16 takes Hidden Inside the Trash
//! Can (20710) in Kerning City, finds the clue in the back alley's trash
//! can, and brings it back.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{complete_quests, set_job};
use integration_harness::events::{maps, talk};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, COMPLETE, SCRIPT_START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use packet::Packet;
use tokio::time::Duration;

const CYGNUS: i32 = 0;
const KERNING: i32 = 103_000_000;
const BACK_ALLEY: i32 = 103_000_101;
const MATTHIAS: i32 = 1103002;
const TRASH_CAN: i32 = 1052111;
const BEFORE_THE_MISSION: i16 = 20709;
const HIDDEN_INSIDE_THE_TRASH_CAN: i16 = 20710;
const CLUE: i32 = 4032136;

/// Walk to NPC `npc` on `map` and send `action` to it.
async fn at(c: &mut MapleTestConnection, m: &str, map: i32, npc: i32, action: Packet) -> Trace {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    send(c, walk_to(at.x, at.y)).await;
    barrier(c, m, &mut Trace::default()).await;
    step(c, m, action).await
}

#[tokio::test]
async fn a_knight_finds_matthiass_clue_in_kerning_citys_trash_can() {
    let config = load_harness_config_or_fail().await.with_class(CYGNUS);
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 1100, 16, 50)
        .await
        .expect("a level 16 Dawn Warrior");
    complete_quests(id, BEFORE_THE_MISSION, BEFORE_THE_MISSION)
        .await
        .expect("the quest before");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;

    // Before the mission the trash can holds nothing.
    let mut t = Trace::default();
    assert_eq!(warp(c, BACK_ALLEY, "", &mut t).await, BACK_ALLEY);
    let t = at(c, &m, BACK_ALLEY, TRASH_CAN, talk(BACK_ALLEY, TRASH_CAN)).await;
    assert_eq!(t.prompts, [(TRASH_CAN, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    assert!(t.added.is_empty(), "{:?}", t.added);

    let mut t = Trace::default();
    assert_eq!(warp(c, KERNING, "", &mut t).await, KERNING);
    let start = build_quest_action(SCRIPT_START, HIDDEN_INSIDE_THE_TRASH_CAN, MATTHIAS);
    let t = at(c, &m, KERNING, MATTHIAS, start).await;
    assert_eq!(t.prompts, [(MATTHIAS, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.states.get(&HIDDEN_INSIDE_THE_TRASH_CAN), Some(&1));

    let mut t = Trace::default();
    assert_eq!(warp(c, BACK_ALLEY, "", &mut t).await, BACK_ALLEY);
    let t = at(c, &m, BACK_ALLEY, TRASH_CAN, talk(BACK_ALLEY, TRASH_CAN)).await;
    assert_eq!(t.prompts, [(TRASH_CAN, 0)]);
    let t = step(c, &m, answer(0, 1, None)).await;
    let found: Vec<_> = t.added.iter().map(|a| a.2).collect();
    assert_eq!(found, [CLUE]);

    let mut t = Trace::default();
    assert_eq!(warp(c, KERNING, "", &mut t).await, KERNING);
    let end = build_quest_action(COMPLETE, HIDDEN_INSIDE_THE_TRASH_CAN, MATTHIAS);
    let t = at(c, &m, KERNING, MATTHIAS, end).await;
    assert_eq!(t.states.get(&HIDDEN_INSIDE_THE_TRASH_CAN), Some(&2));
    assert_eq!(t.removed.len(), 1, "{:?}", t.removed);
    drop(session);

    let after = relogin(&config).await;
    assert_eq!(after.items.get(&CLUE), None);
}
