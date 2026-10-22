//! Henesys's party quest, its first slice: Tory takes a party of three to
//! six from level 10 up from Henesys Park to its own copy of Primrose Hill;
//! Growlie lets players out to the way out, where Tommy sends them home; and
//! the quest ends for everyone when too few remain or its time is up. The
//! harness world runs events 60 times as fast (`RUSTMS_EVENT_SCALE`), so the
//! quest lasts 10 seconds.

use integration_harness::events::{enter_hpq, party, talk, waiting_in, GROWLIE, TOMMY, TORY};
use integration_harness::quests::answer;
use integration_harness::trace::{step, until, Trace};
use integration_harness::WorldSession;
use net::events::HPQ;

/// What `player` is told on sending `packet`: the NPC and the prompt's
/// message type.
async fn told(player: &mut WorldSession, packet: packet::Packet) -> Vec<(i32, u8)> {
    let name = player.character_name.clone();
    step(&mut player.connection, &name, packet).await.prompts
}

/// Say yes to the open prompt; returns the trace up to the barrier.
async fn yes(player: &mut WorldSession) -> Trace {
    let name = player.character_name.clone();
    step(&mut player.connection, &name, answer(1, 1, None)).await
}

/// Wait until `player` is moved to `map`, carrying on from `trace`.
async fn moved_on(player: &mut WorldSession, map: i32, mut trace: Trace) -> Trace {
    until(&mut player.connection, &mut trace, |t| {
        t.warps.contains(&map)
    })
    .await;
    trace
}

/// Tommy sends `player`, at the way out, back to Henesys Park.
async fn home(player: &mut WorldSession) {
    assert_eq!(told(player, talk(HPQ.exit, TOMMY)).await, [(TOMMY, 1)]);
    assert_eq!(yes(player).await.warps, [HPQ.lobby]);
}

#[tokio::test]
async fn tory_takes_a_party_up_to_primrose_hill_until_it_ends() {
    let mut a = Vec::new();
    for _ in 0..3 {
        a.push(waiting_in(HPQ.lobby, 10, &[(4001095, 1)]).await);
    }
    party(&mut a).await;

    // Only the party's leader may take it up; anyone may ask how.
    assert_eq!(told(&mut a[1], talk(HPQ.lobby, TORY)).await, [(TORY, 4)]);
    assert_eq!(told(&mut a[1], answer(4, 1, Some(0))).await, [(TORY, 0)]);

    // Tory asks the leader, and up they go together, on the clock.
    assert_eq!(told(&mut a[0], talk(HPQ.lobby, TORY)).await, [(TORY, 4)]);
    assert_eq!(told(&mut a[0], answer(4, 1, Some(0))).await, [(TORY, 1)]);
    let mut seen = Some(yes(&mut a[0]).await);
    for player in a.iter_mut() {
        let seen = seen.take().unwrap_or_default();
        let mut trace = moved_on(player, HPQ.entry, seen).await;
        until(&mut player.connection, &mut trace, |t| {
            !t.countdowns.is_empty()
        })
        .await;
        assert!((5..=10).contains(&trace.countdowns[0]), "{trace:?}");
    }

    // Growlie lets one out, with the seed taken, which leaves too few: the
    // quest ends, and the rest follow to the way out.
    assert_eq!(
        told(&mut a[2], talk(HPQ.entry, GROWLIE)).await,
        [(GROWLIE, 4)]
    );
    assert_eq!(told(&mut a[2], answer(4, 1, Some(1))).await, [(GROWLIE, 1)]);
    let seen = yes(&mut a[2]).await;
    let mut trace = moved_on(&mut a[2], HPQ.exit, seen).await;
    until(&mut a[2].connection, &mut trace, |t| {
        t.clocks_stopped > 0 && t.removed.contains(&(4, 1))
    })
    .await;
    for player in a[..2].iter_mut() {
        moved_on(player, HPQ.exit, Trace::default()).await;
    }
    for player in a.iter_mut() {
        home(player).await;
    }

    // Back up once the copy closes, until the time is up.
    enter_hpq(&mut a).await;
    for player in a.iter_mut() {
        moved_on(player, HPQ.exit, Trace::default()).await;
    }
}
