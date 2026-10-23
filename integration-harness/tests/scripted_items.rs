//! Scripted items (243xxxx, USE_SCRIPTED_ITEM): the Killer Mushroom Spore
//! opens the poisonous mushrooms' portal for good, and the Thorn Remover,
//! which quest 2324 gives, sets that quest's record by the castle wall, so
//! the Secretary of Magic completes it, which opens the way over the wall.
//! Each works only beside its portal, and takes every one held.

use integration_harness::database::{grant_stack, set_job, set_quest};
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{act, build_quest_completion, START};
use integration_harness::trace::{barrier, send, step, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const USE: u8 = 2;
const KILLER_MUSHROOM_SPORE: i32 = 2_430_014;
const THORN_REMOVER: i32 = 2_430_015;
const THORN_REMOVER_QUEST: i16 = 2324;
const BEFORE_IT: i16 = 2323;
/// The Secretary of Magic's map, and where she stands.
const MUSHROOM_FOREST_FIELD: i32 = 106_020_000;
const SECRETARY: i32 = 1_300_004;
const SECRETARY_AT: (i16, i16) = (-45, 27);
/// The poisonous mushrooms' map, and where its `obstacle` portal is.
const DEEP_INSIDE: i32 = 106_020_300;
const OBSTACLE: (i16, i16) = (1313, -11);
/// The castle wall's map, and where its `right00` portal is.
const SPLIT_ROAD: i32 = 106_020_400;
const THORNS: (i16, i16) = (1388, -24);
const CASTLE_WALL_EDGE: i32 = 106_020_501;
const PINK: u8 = 5;
const STARTED: u8 = 1;
const COMPLETED: u8 = 2;

fn use_item(slot: i16, item_id: i32) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::UseScriptedItem as i16).unwrap();
    p.write_int(0).unwrap();
    p.write_short(slot).unwrap();
    p.write_int(item_id).unwrap();
    p
}

/// CHANGE_MAP_SPECIAL: walk into the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

/// Only the unlock came back.
fn nothing(t: &Trace) {
    assert_eq!(t.stat_updates, [Default::default()], "only the unlock");
    assert!(t.removed.is_empty() && t.quantities.is_empty() && t.broadcasts.is_empty());
}

#[tokio::test]
async fn scripted_items_clear_the_way_to_the_mushroom_castle() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 30, 50).await.unwrap();
    set_quest(id, BEFORE_IT, 2, "").await.unwrap();
    grant_stack(id, USE as i16, 1, KILLER_MUSHROOM_SPORE, 2)
        .await
        .unwrap();

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut seen = Trace::default();
    // The Secretary of Magic gives the Thorn Remover with quest 2324.
    assert_eq!(
        warp(c, MUSHROOM_FOREST_FIELD, "", &mut seen).await,
        MUSHROOM_FOREST_FIELD
    );
    send(c, walk_to(SECRETARY_AT.0, SECRETARY_AT.1)).await;
    let mut t = Trace::default();
    act(c, &mut t, "start", START, THORN_REMOVER_QUEST, SECRETARY).await;
    assert_eq!(t.states.get(&THORN_REMOVER_QUEST), Some(&STARTED));
    assert_eq!(t.added, [(USE, 2, THORN_REMOVER)]);

    assert_eq!(warp(c, DEEP_INSIDE, "", &mut seen).await, DEEP_INSIDE);
    barrier(c, "deep-inside", &mut seen).await;

    // Far from the mushrooms (at the map's spawn point), nothing happens.
    nothing(&step(c, &m, use_item(1, KILLER_MUSHROOM_SPORE)).await);
    // The Thorn Remover does nothing here either.
    nothing(&step(c, &m, use_item(2, THORN_REMOVER)).await);

    // Beside them, both spores go and the way opens.
    send(c, walk_to(OBSTACLE.0 - 100, OBSTACLE.1)).await;
    let t = step(c, "spore", use_item(1, KILLER_MUSHROOM_SPORE)).await;
    assert_eq!(t.removed, [(USE, 1)]);
    assert!(
        matches!(t.broadcasts.as_slice(), [(PINK, text)] if !text.is_empty()),
        "{:?}",
        t.broadcasts
    );
    assert_eq!(
        t.stat_updates.last(),
        Some(&Default::default()),
        "the unlock"
    );
    let t = step(c, "obstacle", enter("obstacle")).await;
    assert_eq!(t.warps, [SPLIT_ROAD]);
    assert!(t.removed.is_empty());

    // The thorns keep the player from the castle. The Thorn Remover, used
    // beside them, sets 2324's record ("001", at least the "1" its
    // Check/1 infoex asks) but doesn't complete it.
    let t = step(c, "thorns", enter("right00")).await;
    assert!(t.warps.is_empty());
    send(c, walk_to(THORNS.0, THORNS.1)).await;
    let t = step(c, "remover", use_item(2, THORN_REMOVER)).await;
    assert_eq!(t.removed, [(USE, 2)]);
    assert!(t.gains.is_empty());
    assert_eq!(t.states.get(&THORN_REMOVER_QUEST), Some(&STARTED));
    assert_eq!(
        t.kills.get(&THORN_REMOVER_QUEST).map(String::as_str),
        Some("001")
    );
    assert!(
        matches!(t.broadcasts.as_slice(), [(PINK, _)]),
        "{:?}",
        t.broadcasts
    );
    let t = step(c, "still-thorns", enter("right00")).await;
    assert!(t.warps.is_empty());

    // The Secretary of Magic completes it, for its 3300 EXP.
    assert_eq!(
        warp(c, MUSHROOM_FOREST_FIELD, "", &mut seen).await,
        MUSHROOM_FOREST_FIELD
    );
    send(c, walk_to(SECRETARY_AT.0, SECRETARY_AT.1)).await;
    let t = step(
        c,
        "complete",
        build_quest_completion(THORN_REMOVER_QUEST, SECRETARY, -1),
    )
    .await;
    assert_eq!(t.states.get(&THORN_REMOVER_QUEST), Some(&COMPLETED));
    assert_eq!(t.gains, [3300]);

    assert_eq!(warp(c, SPLIT_ROAD, "", &mut seen).await, SPLIT_ROAD);
    send(c, walk_to(THORNS.0, THORNS.1)).await;
    let t = step(c, "castle", enter("right00")).await;
    assert_eq!(t.warps, [CASTLE_WALL_EDGE]);
    drop(session);

    // The mushrooms stay open, without a spore or a Poison Spore.
    let mut session = relogin(&config).await;
    assert_eq!(session.quests.get(&THORN_REMOVER_QUEST), Some(&COMPLETED));
    assert_eq!(session.items.get(&KILLER_MUSHROOM_SPORE), None);
    assert_eq!(session.items.get(&THORN_REMOVER), None);
    let c = &mut session.connection;
    let mut seen = Trace::default();
    assert_eq!(warp(c, DEEP_INSIDE, "", &mut seen).await, DEEP_INSIDE);
    barrier(c, "again", &mut seen).await;
    let t = step(c, "obstacle-again", enter("obstacle")).await;
    assert_eq!(t.warps, [SPLIT_ROAD]);
}
