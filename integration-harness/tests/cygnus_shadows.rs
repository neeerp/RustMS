//! The Cygnus Knights' black shadows: the one in the Pig Farm (20706),
//! found among its decoys, and the one in the hidden Magic Library (20718),
//! which a knight on the quest reaches through Ellinia's library door and
//! which brings out Blue Mushrooms that must fall before the way out
//! opens. Each sets the record its quest's completion reads.

use integration_harness::connection::MapleTestConnection;
use integration_harness::database::{replace_item, set_job, set_quest, set_str, set_weapon_attack};
use integration_harness::events::{maps, talk};
use integration_harness::hunting::{attack, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{answer, build_quest_action, build_quest_completion, START};
use integration_harness::trace::{barrier, read_for, send, step, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const HENESYS: i32 = 100000000;
const NEINHEART: i32 = 1103001;
const PIG_FARM: i32 = 100020000;
const PIG_FARM_SHADOW: i32 = 1012115;
const PIG_FARM_DECOY: i32 = 1012116;
const BLACK_SHADOW_IN_THE_PIG_FARM: i16 = 20706;
const PIG_FARM_RECORD: i16 = 20731;

const ELLINIA: i32 = 101000000;
const CYGNUS_KNIGHT_IN_ELLINIA: i32 = 1103003;
const MAGIC_LIBRARY: i32 = 101000003;
/// Ellinia's library door, `jobin00`.
const LIBRARY_DOOR: (i16, i16) = (334, -3987);
const HIDDEN_MAGIC_LIBRARY: i32 = 910110000;
const LIBRARY_SHADOW: i32 = 1032109;
const MAYBE_ITS_GRENDEL: i16 = 20718;
const LIBRARY_RECORD: i16 = 20732;
const BLUE_MUSHROOM: i32 = 2220100;

/// A walk to NPC `npc` on `map`.
fn to(map: i32, npc: i32) -> Packet {
    let npcs = &maps().field(map).unwrap().map_npcs;
    let at = npcs.iter().find(|n| n.npc_id == npc).unwrap();
    walk_to(at.x, at.y)
}

/// Walk to NPC `npc` on `map` and talk to it.
async fn touch(c: &mut MapleTestConnection, m: &str, map: i32, npc: i32) -> Trace {
    send(c, to(map, npc)).await;
    barrier(c, m, &mut Trace::default()).await;
    step(c, m, talk(map, npc)).await
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

#[tokio::test]
async fn the_pig_farms_shadow_is_found_among_its_decoys() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 1100, 30, 50).await.unwrap();
    set_quest(id, 20705, 2, "").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(warp(c, HENESYS, "", &mut trace).await, HENESYS);
    send(c, to(HENESYS, NEINHEART)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_action(START, BLACK_SHADOW_IN_THE_PIG_FARM, NEINHEART),
    )
    .await;
    assert_eq!(t.states.get(&BLACK_SHADOW_IN_THE_PIG_FARM), Some(&1));

    assert_eq!(warp(c, PIG_FARM, "", &mut trace).await, PIG_FARM);
    let t = touch(c, &m, PIG_FARM, PIG_FARM_DECOY).await;
    assert_eq!(t.prompts, [(PIG_FARM_DECOY, 0)]);
    assert!(t.kills.is_empty(), "{t:?}");
    send(c, answer(0, -1, None)).await;
    let t = touch(c, &m, PIG_FARM, PIG_FARM_SHADOW).await;
    assert_eq!(t.kills[&PIG_FARM_RECORD], "1");
    send(c, answer(0, -1, None)).await;

    assert_eq!(warp(c, HENESYS, "", &mut trace).await, HENESYS);
    send(c, to(HENESYS, NEINHEART)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_completion(BLACK_SHADOW_IN_THE_PIG_FARM, NEINHEART, -1),
    )
    .await;
    assert_eq!(t.states.get(&BLACK_SHADOW_IN_THE_PIG_FARM), Some(&2));
    assert_eq!(t.gains, [1000]);
}

#[tokio::test]
async fn the_hidden_library_lets_its_knight_out_once_the_mushrooms_fall() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config).await.unwrap().character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 1100, 30, 50).await.unwrap();
    replace_item(id, -1, -11, 1302000).await.unwrap();
    set_weapon_attack(id, 500).await.unwrap();
    set_str(id, 999).await.unwrap();
    set_quest(id, 20717, 2, "").await.unwrap();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut trace = Trace::default();

    assert_eq!(warp(c, ELLINIA, "", &mut trace).await, ELLINIA);
    send(c, to(ELLINIA, CYGNUS_KNIGHT_IN_ELLINIA)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_action(START, MAYBE_ITS_GRENDEL, CYGNUS_KNIGHT_IN_ELLINIA),
    )
    .await;
    assert_eq!(t.states.get(&MAYBE_ITS_GRENDEL), Some(&1));

    // On the quest, the library door leads to the hidden library.
    step(c, &m, walk_to(LIBRARY_DOOR.0, LIBRARY_DOOR.1)).await;
    send(c, enter("jobin00")).await;
    let mut trace = Trace::default();
    until(c, &mut trace, |t| t.warps == [HIDDEN_MAGIC_LIBRARY]).await;

    // The shadow rises, and the mushrooms spill out after the last page.
    let t = touch(c, &m, HIDDEN_MAGIC_LIBRARY, LIBRARY_SHADOW).await;
    assert_eq!(t.prompts, [(LIBRARY_SHADOW, 0)]);
    send(c, answer(0, 1, None)).await;
    let mut trace = Trace::default();
    until(c, &mut trace, |t| {
        t.kills.get(&LIBRARY_RECORD).map(String::as_str) == Some("1")
            && t.field.alive(BLUE_MUSHROOM).len() == 30
    })
    .await;
    // Talked to again, it has nothing more.
    let t = touch(c, &m, HIDDEN_MAGIC_LIBRARY, LIBRARY_SHADOW).await;
    assert!(t.kills.is_empty(), "{t:?}");
    send(c, answer(0, -1, None)).await;

    // The way out stays shut while any mushroom stands.
    let out = maps()
        .field(HIDDEN_MAGIC_LIBRARY)
        .unwrap()
        .portal_by_name("out00")
        .map(|p| (p.x as i16, p.y as i16))
        .unwrap();
    step(c, &m, walk_to(out.0, out.1)).await;
    let t = step(c, &m, enter("out00")).await;
    assert!(t.warps.is_empty(), "{t:?}");
    assert_eq!(t.broadcasts.len(), 1, "told in pink: {:?}", t.broadcasts);

    for mushroom in trace.field.alive(BLUE_MUSHROOM) {
        for _ in 0..20 {
            send(c, attack(mushroom, 30_000)).await;
            read_for(c, &mut trace, 100).await;
            if trace.field.killed.contains(&mushroom) {
                break;
            }
        }
    }
    assert!(trace.field.alive(BLUE_MUSHROOM).is_empty());
    send(c, enter("out00")).await;
    let mut trace = Trace::default();
    until(c, &mut trace, |t| t.warps == [ELLINIA]).await;

    // Found, the door leads to Grendel's house again.
    step(c, &m, walk_to(LIBRARY_DOOR.0, LIBRARY_DOOR.1)).await;
    send(c, enter("jobin00")).await;
    let mut trace = Trace::default();
    until(c, &mut trace, |t| t.warps == [MAGIC_LIBRARY]).await;

    assert_eq!(warp(c, ELLINIA, "", &mut trace).await, ELLINIA);
    send(c, to(ELLINIA, CYGNUS_KNIGHT_IN_ELLINIA)).await;
    barrier(c, &m, &mut trace).await;
    let t = step(
        c,
        &m,
        build_quest_completion(MAYBE_ITS_GRENDEL, CYGNUS_KNIGHT_IN_ELLINIA, -1),
    )
    .await;
    assert_eq!(t.states.get(&MAYBE_ITS_GRENDEL), Some(&2));
    assert_eq!(t.gains, [4000]);
}
