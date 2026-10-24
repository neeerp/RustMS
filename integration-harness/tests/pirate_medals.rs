//! The Pirate Ship's second stage: only the pirates of the Mark in turn
//! come out, at every point of theirs, and the leader hands Guon twenty of
//! each Mark in turn, the Rookie's, the Rising's, then the Veteran's
//! (Cosmic `2094002`, `PiratePQ`'s `respawnStages`). Each swaps the pirates
//! for the next Mark's, the third clears the stage and leaves it empty, and
//! only then does the way on (`davy_next1`) open. The harness world runs
//! events 60 times as fast (`RUSTMS_EVENT_SCALE`), so the stage's 6
//! minutes are 6 seconds.

use game_data::server_content::keys;
use integration_harness::events::{
    armed_in, armed_with, cleared, enter_pirate, maps, party, said, talk, told,
};
use integration_harness::hunting::walk_to;
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{PIRATE_LOBBY, PIRATE_MARKS, PIRATE_STAGES};
use net::npc_scripts::GUON_ABOARD;
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const PINK: u8 = 5;
const ETC: u8 = 4;
/// How many points of its own each Mark's pirates have (Map.nx).
const POINTS: [usize; 3] = [31, 33, 33];

fn alive(trace: &Trace, mob_id: i32) -> usize {
    trace.field.alive(mob_id).len()
}

/// Only the pirates of Mark `n` are out, at all of their points, or none
/// past the third.
fn only(trace: &Trace, n: usize) -> bool {
    PIRATE_MARKS.iter().enumerate().all(|(i, &(_, mob_id))| {
        let wanted = if i == n { POINTS[i] } else { 0 };
        alive(trace, mob_id) == wanted
    })
}

/// CHANGE_MAP_SPECIAL: walk to the second stage's way on, `next00`, and
/// press Up.
async fn way_on(player: &mut WorldSession) {
    let portal = maps().field(PIRATE_STAGES[1]).unwrap();
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

#[tokio::test]
async fn guon_takes_each_mark_in_turn_swapping_the_pirates_until_the_way_on_opens() {
    // The leader holds the Rising's and the Veteran's ahead of the
    // Rookie's; a member holds the Rookie's too.
    let (rookie, rising, veteran) = (PIRATE_MARKS[0].0, PIRATE_MARKS[1].0, PIRATE_MARKS[2].0);
    let mut team = vec![
        armed_with(
            PIRATE_LOBBY,
            70,
            &[(rising, 20), (veteran, 20), (rookie, 20)],
        )
        .await,
        armed_with(PIRATE_LOBBY, 70, &[(rookie, 20)]).await,
        armed_in(PIRATE_LOBBY, 70).await,
    ];
    party(&mut team).await;
    enter_pirate(&mut team).await;
    let stage = PIRATE_STAGES[1];
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut team[0].connection, stage, "", &mut seen).await,
        stage
    );
    let ahead = &mut team[1].connection;
    assert_eq!(warp(ahead, stage, "", &mut Trace::default()).await, stage);
    until(&mut team[0].connection, &mut seen, |t| only(t, 0)).await;

    // A member is only asked whether to leave, and keeps their Marks.
    let asked = told(&mut team[1], talk(stage, GUON_ABOARD)).await;
    assert!(said(&asked, keys::PIRATE_GUON_LEAVE), "{asked:?}");
    let no = told(&mut team[1], answer(1, 0, None)).await;
    assert!(no.removed.is_empty() && no.warps.is_empty(), "{no:?}");

    // Each Mark in turn, whatever the order in the bag: the Rookie's in the
    // third slot, then the Rising's in the first, then the Veteran's in the
    // second, which clears the stage and leaves it empty.
    let leader = &mut team[0];
    for (n, (key, slot)) in [
        (keys::PIRATE_GUON_RISING, 3),
        (keys::PIRATE_GUON_VETERAN, 1),
        (keys::PIRATE_GUON_MARKS_IN, 2),
    ]
    .into_iter()
    .enumerate()
    {
        if n == 2 {
            // Until the last is in, the way on says so in pink and stays
            // shut.
            let warps = seen.warps.len();
            way_on(leader).await;
            let shut = net::content::text().get(keys::PIRATE_MARKS_SHUT, &[]);
            let c = &mut leader.connection;
            until(c, &mut seen, |t| {
                t.broadcasts.contains(&(PINK, shut.clone()))
            })
            .await;
            read_for(c, &mut seen, 300).await;
            assert_eq!(seen.warps.len(), warps, "{seen:?}");
        }
        let c = &mut leader.connection;
        let (said_before, removed_before) = (seen.prompt_texts.len(), seen.removed.len());
        send(c, talk(stage, GUON_ABOARD)).await;
        until(c, &mut seen, |t| {
            let text = net::content::text().get(key, &[]);
            t.prompt_texts[said_before..]
                .iter()
                .any(|s| s.contains(&text))
                && t.removed[removed_before..] == [(ETC, slot)]
                && only(t, n + 1)
        })
        .await;
    }
    let c = &mut leader.connection;
    until(c, &mut seen, cleared).await;
    read_for(c, &mut seen, 1_000).await;
    assert!(only(&seen, 3), "none come back");
    let done = told(leader, talk(stage, GUON_ABOARD)).await;
    assert!(said(&done, keys::PIRATE_GUON_MARKS_IN), "{done:?}");
    assert!(done.removed.is_empty());

    // Now the way on leads to the third stage.
    way_on(leader).await;
    let mut trace = Trace::default();
    until(&mut leader.connection, &mut trace, |t| !t.warps.is_empty()).await;
    assert_eq!(trace.warps, [PIRATE_STAGES[2]], "{trace:?}");
}
