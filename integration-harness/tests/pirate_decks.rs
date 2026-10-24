//! The Pirate Ship's decks, its third and fourth stages: each opens with
//! twenty of the Lord Pirate's crew, ten Krus and ten Captains, a mob box
//! there breaks on the fourth hit into six of each more (Cosmic `PiratePQ`'s
//! `setup`, `2511001`), and the way on (`davy_next2`, `davy_next3`) stays
//! shut, with a pink line, until all are dead, which clears the stage. The
//! hidden way down to the crew's room (`davy2_hd1`, `davy3_hd1`) is open
//! whatever is left. The harness world runs events 60 times as fast
//! (`RUSTMS_EVENT_SCALE`), so each stage's 6 minutes are 6 seconds.

use game_data::server_content::keys;
use integration_harness::events::{armed_in, cleared, enter_pirate, maps, party};
use integration_harness::hunting::{attack, hit_reactor, walk_to};
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{PIRATE_LOBBY, PIRATE_STAGES};
use net::packet::op::RecvOpcode;
use net::reactors::PIRATE_DECK_BOX;
use packet::io::write::PktWrite;
use packet::Packet;

/// The Lord Pirate's Krus and Captains.
const CREW: [i32; 2] = [9300124, 9300125];
const PINK: u8 = 5;

fn alive(trace: &Trace, mob_id: i32) -> usize {
    trace.field.alive(mob_id).len()
}

/// CHANGE_MAP_SPECIAL: walk to `map`'s portal `name` and press Up.
async fn take(player: &mut WorldSession, map: i32, name: &str) {
    let field = maps().field(map).unwrap();
    let portal = field.portal_by_name(name).unwrap();
    send(
        &mut player.connection,
        walk_to(portal.x as i16, portal.y as i16),
    )
    .await;
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(name).unwrap();
    p.write_short(0).unwrap();
    send(&mut player.connection, p).await;
}

/// Hit the first deck mob box aboard four times, and wait until it is gone.
async fn break_box(player: &mut WorldSession, seen: &mut Trace) {
    let c = &mut player.connection;
    let reactor = *(seen.field.reactors.iter())
        .find(|r| r.reactor_id == PIRATE_DECK_BOX)
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
async fn a_deck_opens_with_its_crew_and_its_way_on_opens_once_all_are_dead() {
    let mut team = Vec::new();
    for _ in 0..3 {
        team.push(armed_in(PIRATE_LOBBY, 70).await);
    }
    party(&mut team).await;
    enter_pirate(&mut team).await;
    let deck = PIRATE_STAGES[2];
    let mut seen = Trace::default();
    let mut theirs = Trace::default();
    assert_eq!(
        warp(&mut team[0].connection, deck, "", &mut seen).await,
        deck
    );
    assert_eq!(
        warp(&mut team[1].connection, deck, "", &mut theirs).await,
        deck
    );
    let c = &mut team[0].connection;
    until(c, &mut seen, |t| {
        CREW.iter().all(|&mob_id| alive(t, mob_id) == 10)
            && (t.field.reactors.iter())
                .filter(|r| r.reactor_id == PIRATE_DECK_BOX)
                .count()
                == 6
    })
    .await;

    // With the crew aboard, the way on says so in pink and stays shut.
    let warps = seen.warps.len();
    take(&mut team[0], deck, "next00").await;
    let shut = net::content::text().get(keys::PIRATE_PORTAL_SHUT, &[]);
    let c = &mut team[0].connection;
    until(c, &mut seen, |t| {
        t.broadcasts.contains(&(PINK, shut.clone()))
    })
    .await;
    read_for(c, &mut seen, 300).await;
    assert_eq!(seen.warps.len(), warps, "{seen:?}");

    // Six Krus and six Captains come out of a mob box.
    break_box(&mut team[0], &mut seen).await;
    let c = &mut team[0].connection;
    until(c, &mut seen, |t| {
        CREW.iter().all(|&mob_id| alive(t, mob_id) == 16)
    })
    .await;

    // The leader kills them all at once; the last clears the stage for
    // the member aboard too.
    let crew: Vec<i32> = (CREW.iter())
        .flat_map(|&mob_id| seen.field.alive(mob_id))
        .collect();
    for &oid in &crew {
        send(c, attack(oid, 30_000)).await;
    }
    until(c, &mut seen, |t| {
        crew.iter().all(|oid| t.field.killed.contains(oid)) && cleared(t)
    })
    .await;
    until(&mut team[1].connection, &mut theirs, cleared).await;

    // Now the way on leads to the second deck, which has a crew of its own.
    take(&mut team[0], deck, "next00").await;
    let mut trace = Trace::default();
    let c = &mut team[0].connection;
    until(c, &mut trace, |t| !t.warps.is_empty()).await;
    assert_eq!(trace.warps, [PIRATE_STAGES[3]], "{trace:?}");
    until(c, &mut trace, |t| {
        CREW.iter().all(|&mob_id| alive(t, mob_id) == 10)
    })
    .await;

    // Its hidden way down leads to its crew's room, with the crew still
    // aboard.
    take(&mut team[0], PIRATE_STAGES[3], "in01").await;
    let mut trace = Trace::default();
    until(&mut team[0].connection, &mut trace, |t| !t.warps.is_empty()).await;
    assert_eq!(trace.warps, [PIRATE_STAGES[3] + 2], "{trace:?}");
}
