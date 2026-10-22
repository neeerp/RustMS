//! Orbis's party quest blesses a team with all five job lines: on arrival
//! each member gets the same one of Wonky's four blessings, an item buff
//! of half an hour, and loses it on leaving the tower.

use integration_harness::events::{enter_opq, party, talk, waiting_as, WONKY};
use integration_harness::quests::answer;
use integration_harness::trace::{step, until, Trace};
use net::events::OPQ;

#[tokio::test]
async fn a_team_of_every_job_line_is_blessed_until_it_leaves() {
    // A warrior, a magician, a bowman, a thief and a pirate.
    let mut a = Vec::new();
    for job in [100, 200, 300, 400, 500] {
        a.push(waiting_as(OPQ.lobby, 51, job).await);
    }
    party(&mut a).await;

    let traces = enter_opq(&mut a).await;
    let mut given = Vec::new();
    for (player, mut trace) in a.iter_mut().zip(traces) {
        until(&mut player.connection, &mut trace, |t| {
            t.buffs
                .iter()
                .any(|&(id, _, _)| OPQ.blessings.contains(&-id))
        })
        .await;
        let &(id, _, ms) = trace
            .buffs
            .iter()
            .find(|&&(id, _, _)| OPQ.blessings.contains(&-id))
            .unwrap();
        assert_eq!(ms, 30 * 60 * 1000, "{trace:?}");
        given.push(-id);
    }
    assert!(given.iter().all(|&item| item == given[0]), "{given:?}");

    // One leaves through Wonky and loses it; that leaves too few, so the
    // rest are sent out and lose it too.
    let name = a[4].character_name.clone();
    step(&mut a[4].connection, &name, talk(OPQ.entry, WONKY)).await;
    let mut trace = step(&mut a[4].connection, &name, answer(1, 1, None)).await;
    until(&mut a[4].connection, &mut trace, |t| {
        t.warps.contains(&OPQ.exit) && !t.cancelled_buffs.is_empty()
    })
    .await;
    for player in a[..4].iter_mut() {
        let mut trace = Trace::default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&OPQ.exit) && !t.cancelled_buffs.is_empty()
        })
        .await;
    }
}
