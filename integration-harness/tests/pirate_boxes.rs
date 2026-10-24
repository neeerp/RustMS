//! The Pirate Ship's first stage: its boxes break on the fourth hit, a mob
//! box into three Ginseng Jars and five Bellflowers and an item box into a
//! spray of loot (Cosmic `2511000`, `2512000`), and the way on (`davy_next0`)
//! stays shut, with a pink line, until every monster aboard is dead, which
//! clears the stage. The harness world runs events 60 times as fast
//! (`RUSTMS_EVENT_SCALE`), so the ship's 4 minutes are 4 seconds, and each
//! test does its part quickly.

use game_data::server_content::keys;
use integration_harness::events::{armed_in, cleared, enter_pirate, maps, party};
use integration_harness::hunting::{attack, hit_reactor, walk_to};
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{PIRATE_ENTRY, PIRATE_LOBBY, PIRATE_STAGES};
use net::packet::op::RecvOpcode;
use net::reactors::{PIRATE_ITEM_BOX, PIRATE_MOB_BOX};
use packet::io::write::PktWrite;
use packet::Packet;

/// The first stage's monsters (Map.nx): four Ginseng Jars of one kind,
/// five of another, eight Bellflowers, and nine of another kind.
const FIRST_MOBS: [(i32, usize); 4] = [(9300108, 4), (9300109, 5), (9300110, 8), (9300111, 9)];
const PINK: u8 = 5;

/// Three of level 70, aboard.
async fn aboard() -> (Vec<WorldSession>, Vec<Trace>) {
    let mut team = Vec::new();
    for _ in 0..3 {
        team.push(armed_in(PIRATE_LOBBY, 70).await);
    }
    party(&mut team).await;
    let mut seen = enter_pirate(&mut team).await;
    let c = &mut team[0].connection;
    until(c, &mut seen[0], |t| {
        FIRST_MOBS.iter().all(|&(mob_id, n)| alive(t, mob_id) >= n)
            && boxes(t, PIRATE_MOB_BOX) == 5
            && boxes(t, PIRATE_ITEM_BOX) == 2
    })
    .await;
    (team, seen)
}

fn alive(trace: &Trace, mob_id: i32) -> usize {
    trace.field.alive(mob_id).len()
}

fn boxes(trace: &Trace, reactor_id: i32) -> usize {
    (trace.field.reactors.iter())
        .filter(|r| r.reactor_id == reactor_id)
        .count()
}

/// CHANGE_MAP_SPECIAL: walk to the first stage's way on, `next00`, and
/// press Up.
async fn way_on(player: &mut WorldSession) {
    let portal = maps().field(PIRATE_ENTRY).unwrap();
    let portal = portal.portal_by_name("next00").unwrap();
    send(
        &mut player.connection,
        walk_to(portal.x as i16, portal.y as i16),
    )
    .await;
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length("next00").unwrap();
    p.write_short(0).unwrap();
    send(&mut player.connection, p).await;
}

/// Hit the first box `reactor_id` aboard four times, and wait until it is
/// gone.
async fn break_box(player: &mut WorldSession, seen: &mut Trace, reactor_id: i32) {
    let c = &mut player.connection;
    let reactor = *(seen.field.reactors.iter())
        .find(|r| r.reactor_id == reactor_id)
        .unwrap();
    send(c, walk_to(reactor.x, reactor.y)).await;
    for state in 1..=3u8 {
        send(c, hit_reactor(reactor.object_id)).await;
        until(c, seen, |t| {
            t.field.reactor_hits.contains(&(reactor.object_id, state))
        })
        .await;
    }
    send(c, hit_reactor(reactor.object_id)).await;
    until(c, seen, |t| {
        t.field
            .reactors_removed
            .iter()
            .any(|&(oid, _)| oid == reactor.object_id)
    })
    .await;
}

#[tokio::test]
async fn a_mob_box_brings_out_eight_an_item_box_its_loot_and_the_way_on_stays_shut() {
    let (mut team, mut seen) = aboard().await;
    let seen = &mut seen[0];
    // A member on ahead starts the clock over at 6 minutes, for more room.
    let ahead = &mut team[1].connection;
    let stage2 = PIRATE_STAGES[1];
    assert_eq!(warp(ahead, stage2, "", &mut Trace::default()).await, stage2);

    // Three Ginseng Jars and five Bellflowers come out of a mob box.
    let (jars, bellflowers) = (alive(seen, 9300109), alive(seen, 9300110));
    break_box(&mut team[0], seen, PIRATE_MOB_BOX).await;
    let c = &mut team[0].connection;
    until(c, seen, |t| {
        alive(t, 9300109) >= jars + 3 && alive(t, 9300110) >= bellflowers + 5
    })
    .await;

    // At least fifteen drops come out of an item box.
    let before = seen.field.drops.len();
    break_box(&mut team[0], seen, PIRATE_ITEM_BOX).await;
    let c = &mut team[0].connection;
    until(c, seen, |t| t.field.drops.len() >= before + 15).await;

    // With monsters left, the way on says so in pink and stays shut.
    way_on(&mut team[0]).await;
    let shut = net::content::text().get(keys::PIRATE_PORTAL_SHUT, &[]);
    let c = &mut team[0].connection;
    let mut trace = Trace::default();
    until(c, &mut trace, |t| {
        t.broadcasts.contains(&(PINK, shut.clone()))
    })
    .await;
    read_for(c, &mut trace, 300).await;
    assert!(trace.warps.is_empty(), "{trace:?}");
}

#[tokio::test]
async fn the_first_stage_clears_with_its_last_monster_and_the_way_on_opens() {
    let (mut team, mut seen) = aboard().await;

    // The leader kills every monster aboard, all at once, as the clock is
    // short; the last clears the stage for everyone.
    let c = &mut team[0].connection;
    let mobs: Vec<i32> = (FIRST_MOBS.iter())
        .flat_map(|&(mob_id, _)| seen[0].field.alive(mob_id))
        .collect();
    for &oid in &mobs {
        send(c, attack(oid, 30_000)).await;
    }
    until(c, &mut seen[0], |t| {
        mobs.iter().all(|oid| t.field.killed.contains(oid)) && cleared(t)
    })
    .await;
    for (player, trace) in team[1..].iter_mut().zip(&mut seen[1..]) {
        until(&mut player.connection, trace, cleared).await;
    }

    // Now the way on leads to the second stage.
    way_on(&mut team[0]).await;
    let mut trace = Trace::default();
    until(&mut team[0].connection, &mut trace, |t| !t.warps.is_empty()).await;
    assert_eq!(trace.warps, [PIRATE_STAGES[1]], "{trace:?}");
}
