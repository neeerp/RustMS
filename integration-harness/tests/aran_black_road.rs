//! An Aran's escape down the Black Road: the ark's map marks it reached,
//! and its way out stays shut until Athena Pierce's Find the Missing Kid 1
//! (21000) is taken up. Each stretch of the road on teaches one of the
//! tutorial's skills. The Lost Kid ends 21000 and starts 21001, coming
//! along as the Lost Child; the way back leads toward the ark, where Athena
//! takes the child. The Aran stays behind, loses the weapon, and wakes in
//! Snow Island's Ice Cave.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{replace_item, set_job};
use integration_harness::events::maps;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{
    answer, build_quest_action, build_quest_completion, SCRIPT_END, SCRIPT_START,
};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const BLACK_ROAD: i32 = 914000000;
const ARK: i32 = 914000100;
const STRETCHES: [i32; 3] = [914000200, 914000210, 914000220];
const LOST_KIDS_FOREST: i32 = 914000300;
const WAY_BACK: i32 = 914000400;
const GANGWAY: i32 = 914000500;
const ICE_CAVE: i32 = 140090000;
const ATHENA: i32 = 1209000;
const LOST_KID: i32 = 1209006;
const ATHENA_AT_THE_GANGWAY: i32 = 1209007;
const MISSING_KID_1: i16 = 21000;
const MISSING_KID_2: i16 = 21001;
const ARK_REACHED: i16 = 21002;
const LOST_CHILD: i32 = 4001271;
const EQUIP: u8 = 1;
const PINK: u8 = 5;

/// Walk to NPC `npc` on `map` and send `action`.
async fn quest(c: &mut MapleTestConnection, m: &str, map: i32, npc: i32, action: Packet) -> Trace {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    send(c, walk_to(at.x, at.y)).await;
    barrier(c, m, &mut Trace::default()).await;
    step(c, m, action).await
}

/// Going through the scripted portal `portal` of the current map.
async fn enter(c: &mut MapleTestConnection, m: &str, map: i32, portal: &str) -> Trace {
    let field = maps().field(map).unwrap();
    let at = field.portals().find(|p| p.name == portal).unwrap();
    step(c, m, walk_to(at.x as i16, at.y as i16)).await;
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    step(c, m, p).await
}

#[tokio::test]
async fn the_black_road_leads_an_aran_from_the_ark_to_the_ice_cave() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 2000, 1, 50).await.expect("an Aran beginner");
    replace_item(id, -1, -11, 1302000).await.unwrap();

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, BLACK_ROAD, "", &mut t).await, BLACK_ROAD);

    // To the ark, which marks it reached.
    let t = enter(c, &m, BLACK_ROAD, "out00").await;
    assert_eq!(t.warps, [ARK]);
    assert_eq!(
        t.kills.get(&ARK_REACHED).map(String::as_str),
        Some("1"),
        "{t:?}"
    );

    // The way on is shut before Athena's request.
    let t = enter(c, &m, ARK, "out00").await;
    assert!(t.warps.is_empty(), "{t:?}");
    assert!(t.broadcasts.iter().any(|b| b.0 == PINK), "{t:?}");
    let start = build_quest_action(SCRIPT_START, MISSING_KID_1, ATHENA);
    let t = quest(c, &m, ARK, ATHENA, start).await;
    assert_eq!(t.prompts, [(ATHENA, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.states.get(&MISSING_KID_1), Some(&1));
    for _ in 0..3 {
        step(c, &m, answer(0, 1, None)).await;
    }

    // Each stretch teaches its skills.
    let mut from = ARK;
    let stretches: [(&str, i32, &[i32]); 3] = [
        ("out00", STRETCHES[0], &[20000017, 20000018]),
        ("west00", STRETCHES[1], &[20000014, 20000015]),
        ("west00", STRETCHES[2], &[20000016]),
    ];
    for (portal, map, skills) in stretches {
        let t = enter(c, &m, from, portal).await;
        assert_eq!(t.warps, [map], "{t:?}");
        let taught: Vec<i32> = t.skills.iter().map(|s| s.0).collect();
        assert_eq!(taught, skills, "{t:?}");
        from = map;
    }

    // The Lost Kid ends the search and comes along.
    let mut t = Trace::default();
    assert_eq!(
        warp(c, LOST_KIDS_FOREST, "", &mut t).await,
        LOST_KIDS_FOREST
    );
    let done = build_quest_completion(MISSING_KID_1, LOST_KID, -1);
    let t = quest(c, &m, LOST_KIDS_FOREST, LOST_KID, done).await;
    assert_eq!(t.states.get(&MISSING_KID_1), Some(&2));
    let start = build_quest_action(SCRIPT_START, MISSING_KID_2, LOST_KID);
    let t = step(c, &m, start).await;
    assert_eq!(t.prompts, [(LOST_KID, 1)]);
    let t = step(c, &m, answer(1, 1, None)).await;
    assert_eq!(t.states.get(&MISSING_KID_2), Some(&1));
    assert_eq!(t.items, [(LOST_CHILD, 1)]);
    assert_eq!(t.warps, [LOST_KIDS_FOREST]);

    // Then the way out leads back toward the ark.
    let t = enter(c, &m, LOST_KIDS_FOREST, "east00").await;
    assert_eq!(t.warps, [WAY_BACK]);

    // Athena takes the child; the Aran stays, loses the weapon, and
    // wakes in the Ice Cave.
    let mut t = Trace::default();
    assert_eq!(warp(c, GANGWAY, "", &mut t).await, GANGWAY);
    let end = build_quest_action(SCRIPT_END, MISSING_KID_2, ATHENA_AT_THE_GANGWAY);
    let t = quest(c, &m, GANGWAY, ATHENA_AT_THE_GANGWAY, end).await;
    assert_eq!(t.prompts, [(ATHENA_AT_THE_GANGWAY, 1)]);
    step(c, &m, answer(1, 1, None)).await;
    let mut t = Trace::default();
    for _ in 0..7 {
        t = step(c, &m, answer(0, 1, None)).await;
    }
    assert_eq!(t.states.get(&MISSING_KID_2), Some(&2), "{t:?}");
    assert_eq!(t.warps, [ICE_CAVE], "{t:?}");
    assert!(t.removed.contains(&(EQUIP, -11)), "{t:?}");
    assert!(t.items.contains(&(LOST_CHILD, -1)), "{t:?}");
}
