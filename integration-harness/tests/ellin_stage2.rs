//! The Forest of Poison Haze's second stage: its way on stays shut until
//! its spine has taken four Diluted Poisons, one at a time, each after it
//! has lain on it a while; the spine stays, in its last state, and the
//! stage clears. (Its pond, which gives a Diluted Poison back for each
//! Concentrated Poison its Poisoned Stone Bugs drop, and fills again on its
//! own, is the reactor set's tests': nobody can throw a Concentrated
//! Poison.) The harness world runs events 60 times as fast, so the forest
//! lasts 30 seconds, and the four take 20.

use game_data::server_content::keys;
use integration_harness::events::{cleared, maps, party, talk, throw_down, told, waiting_in};
use integration_harness::hunting::walk_to;
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{ELLIN_ENTRY, ELLIN_LOBBY, ELLIN_STAGES};
use net::npc_scripts::ELLIN as ELLIN_NPC;
use net::packet::op::RecvOpcode;
use net::reactors::ELLIN_SPINE;
use packet::io::write::PktWrite;
use packet::Packet;
use tokio::time::Duration;

const DILUTED_POISON: i32 = 4001162;
const PINK: u8 = 5;

/// CHANGE_MAP_SPECIAL: walk to `from`'s way on, `east00`, and press Up.
async fn way_on(player: &mut WorldSession, from: i32) {
    let field = maps().field(from).unwrap();
    let portal = field.portal_by_name("east00").unwrap();
    let (x, y) = (portal.x as i16, portal.y as i16);
    send(&mut player.connection, walk_to(x, y)).await;
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length("east00").unwrap();
    p.write_short(0).unwrap();
    send(&mut player.connection, p).await;
}

#[tokio::test]
async fn the_spine_takes_four_diluted_poisons_and_the_second_stage_clears() {
    let mut team = vec![waiting_in(ELLIN_LOBBY, 50, &[(DILUTED_POISON, 4)]).await];
    for _ in 0..3 {
        team.push(waiting_in(ELLIN_LOBBY, 50, &[]).await);
    }
    party(&mut team).await;

    // In they go, once any party before them is out.
    let mut seen = None;
    for _ in 0..40 {
        told(&mut team[0], talk(ELLIN_LOBBY, ELLIN_NPC)).await;
        let trace = told(&mut team[0], answer(4, 1, Some(0))).await;
        if !trace.prompts.iter().any(|&(from, _)| from == ELLIN_NPC) {
            seen = Some(trace);
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    let mut seen = seen.expect("Ellin never let the party in");
    let c = &mut team[0].connection;
    until(c, &mut seen, |t| t.warps.contains(&ELLIN_ENTRY)).await;

    // The leader goes on to the party's second stage, whose way on is shut.
    let stage = ELLIN_STAGES[1];
    let mut seen = Trace::default();
    assert_eq!(warp(c, stage, "", &mut seen).await, stage);
    until(c, &mut seen, |t| {
        t.field.reactors.iter().any(|r| r.reactor_id == ELLIN_SPINE)
    })
    .await;
    seen.warps.clear();
    way_on(&mut team[0], stage).await;
    let shut = net::content::text().get(keys::ELLIN_STAGE2_SHUT, &[]);
    let c = &mut team[0].connection;
    until(c, &mut seen, |t| {
        t.broadcasts.contains(&(PINK, shut.clone()))
    })
    .await;
    assert!(seen.warps.is_empty(), "{seen:?}");

    // Four Diluted Poisons on the spine: it takes them in turn and stays.
    let spine = *(seen.field.reactors.iter())
        .find(|r| r.reactor_id == ELLIN_SPINE)
        .unwrap();
    send(c, walk_to(spine.x, spine.y - 10)).await;
    for _ in 0..4 {
        send(c, throw_down(1, 1)).await;
    }
    for state in 1..=4 {
        until(c, &mut seen, |t| {
            t.field.reactor_hits.contains(&(spine.object_id, state))
        })
        .await;
    }
    until(c, &mut seen, cleared).await;
    let states: Vec<u8> = (seen.field.reactor_hits.iter())
        .filter(|&&(oid, _)| oid == spine.object_id)
        .map(|&(_, state)| state)
        .collect();
    assert_eq!(states, [1, 2, 3, 4], "{seen:?}");
    assert!(
        !(seen.field.reactors_removed.iter()).any(|&(oid, _)| oid == spine.object_id),
        "{seen:?}"
    );

    // Now the way on leads to the third stage.
    way_on(&mut team[0], stage).await;
    let c = &mut team[0].connection;
    let mut trace = Trace::default();
    until(c, &mut trace, |t| !t.warps.is_empty()).await;
    assert_eq!(trace.warps, [ELLIN_STAGES[2]], "{trace:?}");
}
