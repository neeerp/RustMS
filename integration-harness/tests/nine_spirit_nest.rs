//! The Nine Spirit's Nest (240040611), below Leafre's Dangerous Nest. Its
//! `dragonNest` takes one with the Nine Spirit's Egg (4001094) in alone,
//! onto its `out00`, for 5 minutes (5 s, as the harness world runs events
//! 60 times as fast) before sending them back below. The touch portal on
//! the nest places the egg; a hit hatches it, and the Baby Dragon
//! (2081008) comes out, where Dragon Squad's Mission (3706) ends. One who
//! placed an egg finds the Baby Dragon there on coming back without one.

use game_data::server_content::keys;
use integration_harness::database::{grant_stack, set_job, set_quest};
use integration_harness::hunting::{hit_reactor, walk_to};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{build_quest_action, COMPLETE};
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::WorldSession;
use integration_harness::{login_to_world_session, relogin};
use net::events::{
    BELOW_THE_NEST, NINE_SPIRIT_BABY, NINE_SPIRIT_EGG_PLACED, NINE_SPIRIT_NEST,
    NINE_SPIRIT_NEST_ARRIVAL,
};
use net::npc_scripts::NINE_SPIRIT_EGG;
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;
use std::time::Duration;

const MISSION: i16 = 3706;
const EGG_REACTOR: i32 = 2_406_000;
const DRAGON_STONE: i32 = 2_041_200;
const ETC: u8 = 4;

/// CHANGE_MAP_SPECIAL: the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

fn pinked(seen: &Trace, key: &str) -> bool {
    let text = net::content::text().get(key, &[]);
    seen.broadcasts.iter().any(|b| b.0 == 5 && b.1 == text)
}

/// A level-120 fighter on Dragon Squad's Mission, with `quests` besides
/// and `eggs` eggs, below the nest.
async fn below_the_nest(quests: &[(i16, i16)], eggs: i16) -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 110, 120, 100).await.unwrap();
    set_quest(id, MISSION, 1, "").await.unwrap();
    for &(quest, state) in quests {
        set_quest(id, quest, state, "").await.unwrap();
    }
    if eggs > 0 {
        grant_stack(id, ETC as i16, 1, NINE_SPIRIT_EGG, eggs)
            .await
            .unwrap();
    }
    let mut player = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut player.connection;
    assert_eq!(warp(c, BELOW_THE_NEST, "", &mut seen).await, BELOW_THE_NEST);
    player
}

/// Through `dragonNest` into a copy of the nest, on its `out00`.
async fn into_the_nest(player: &mut WorldSession) -> Trace {
    let mut seen = Trace::default();
    send(&mut player.connection, enter("in00")).await;
    until(&mut player.connection, &mut seen, |t| {
        t.arrivals.iter().any(|a| a.0 == NINE_SPIRIT_NEST)
            && !t.countdowns.is_empty()
            && t.field.reactors.iter().any(|r| r.reactor_id == EGG_REACTOR)
    })
    .await;
    let arrival = seen.arrivals.iter().find(|a| a.0 == NINE_SPIRIT_NEST);
    assert_eq!(arrival.map(|a| a.1), Some(NINE_SPIRIT_NEST_ARRIVAL));
    assert!(seen.countdowns[0] <= 5, "{:?}", seen.countdowns);
    seen
}

#[tokio::test]
async fn the_egg_placed_and_hit_hatches_the_baby_dragon_for_the_mission() {
    let mut player = below_the_nest(&[], 1).await;
    let mut seen = into_the_nest(&mut player).await;
    let egg = *seen
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == EGG_REACTOR)
        .unwrap();
    assert_eq!(egg.state, 0);
    assert!(seen.field.npcs.is_empty(), "{:?}", seen.field.npcs);

    // Placed on the nest, it is taken.
    let c = &mut player.connection;
    send(c, enter("act01")).await;
    until(c, &mut seen, |t| {
        !t.removed.is_empty() && t.field.reactor_hits.contains(&(egg.object_id, 1))
    })
    .await;
    assert_eq!(seen.removed, [(ETC, 1)]);

    // A hit hatches it, and out comes the Baby Dragon.
    send(c, walk_to(egg.x, egg.y)).await;
    send(c, hit_reactor(egg.object_id)).await;
    until(c, &mut seen, |t| {
        t.field.npcs.iter().any(|n| n.1 == NINE_SPIRIT_BABY)
    })
    .await;
    assert!(pinked(&seen, keys::NINE_SPIRIT_EGG_HATCHES), "{seen:?}");

    // The mission ends there.
    send(c, build_quest_action(COMPLETE, MISSION, NINE_SPIRIT_BABY)).await;
    until(c, &mut seen, |t| t.states.get(&MISSION) == Some(&2)).await;
    until(c, &mut seen, |t| {
        t.gains.contains(&90_000) && t.added.iter().any(|a| a.2 == DRAGON_STONE)
    })
    .await;

    // The time running out sends one back below.
    until(c, &mut seen, |t| {
        t.arrivals.iter().any(|a| a.0 == BELOW_THE_NEST)
    })
    .await;

    // Now `dragonNest` leads to the hatched dragon's nest.
    let mut seen = Trace::default();
    send(c, enter("in00")).await;
    until(c, &mut seen, |t| {
        t.arrivals.iter().any(|a| a.0 == 240_040_612)
    })
    .await;
}

#[tokio::test]
async fn back_without_an_egg_after_placing_one_the_baby_dragon_waits() {
    let mut player = below_the_nest(&[(NINE_SPIRIT_EGG_PLACED, 1)], 0).await;
    let mut seen = into_the_nest(&mut player).await;
    until(&mut player.connection, &mut seen, |t| {
        t.field.npcs.iter().any(|n| n.1 == NINE_SPIRIT_BABY)
    })
    .await;
    let egg = seen
        .field
        .reactors
        .iter()
        .find(|r| r.reactor_id == EGG_REACTOR);
    assert_eq!(egg.map(|r| r.state), Some(0));
}

#[tokio::test]
async fn without_the_egg_the_nest_stays_shut() {
    let mut player = below_the_nest(&[], 0).await;
    let mut seen = Trace::default();
    send(&mut player.connection, enter("in00")).await;
    until(&mut player.connection, &mut seen, |t| {
        pinked(t, keys::NINE_SPIRIT_NEST_BLOCKED)
    })
    .await;
    assert!(seen.arrivals.is_empty(), "{seen:?}");
}
