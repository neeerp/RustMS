//! Kerning City's party quest, its first slice: Lakelis takes a party of
//! three or four at levels 21 to 30 from Kerning City into its own copy of
//! the quest's maps, one party at a time per channel; Nella lets players
//! out; and the quest ends for everyone when too few remain or its time is
//! up. The harness world runs events 60 times as fast
//! (`RUSTMS_EVENT_SCALE`), so the quest lasts 30 seconds.

use integration_harness::events::{in_lobby, party, talk, LAKELIS, NELLA};
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, step, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::KPQ;
use tokio::time::Duration;

/// A new character at `level`, logged in and waiting in Kerning City.
async fn player(level: i16) -> WorldSession {
    in_lobby(level, &[]).await
}

/// What `player` is told on talking to `npc` on `map`: the NPC and the
/// prompt's message type.
async fn talk_to(player: &mut WorldSession, map: i32, npc: i32) -> Vec<(i32, u8)> {
    let name = player.character_name.clone();
    step(&mut player.connection, &name, talk(map, npc))
        .await
        .prompts
}

/// Say yes to the open prompt; returns the trace up to the barrier.
async fn yes(player: &mut WorldSession) -> Trace {
    let name = player.character_name.clone();
    step(&mut player.connection, &name, answer(1, 1, None)).await
}

/// Wait until `player` is moved to `map`, and return the trace.
async fn moved_to(player: &mut WorldSession, map: i32) -> Trace {
    moved_on(player, map, Trace::default()).await
}

/// `moved_to`, carrying on from what `trace` has already seen.
async fn moved_on(player: &mut WorldSession, map: i32, mut trace: Trace) -> Trace {
    until(&mut player.connection, &mut trace, |t| {
        t.warps.contains(&map)
    })
    .await;
    trace
}

#[tokio::test]
async fn parties_play_kerning_city_party_quest_in_turn_in_their_own_copy() {
    let mut a = Vec::new();
    for _ in 0..3 {
        a.push(player(25).await);
    }
    let mut b = Vec::new();
    for _ in 0..4 {
        b.push(player(21).await);
    }
    party(&mut a).await;
    party(&mut b).await;

    // Only the party's leader may take it in.
    assert_eq!(talk_to(&mut a[1], KPQ.lobby, LAKELIS).await, [(LAKELIS, 0)]);

    // Lakelis asks the leader, and in they go together, on the clock.
    assert_eq!(talk_to(&mut a[0], KPQ.lobby, LAKELIS).await, [(LAKELIS, 1)]);
    let mut seen = Some(yes(&mut a[0]).await);
    for player in a.iter_mut() {
        let seen = seen.take().unwrap_or_default();
        let mut trace = moved_on(player, KPQ.entry, seen).await;
        until(&mut player.connection, &mut trace, |t| {
            !t.countdowns.is_empty()
        })
        .await;
        assert!((25..=30).contains(&trace.countdowns[0]), "{trace:?}");
    }

    // Another party must wait its turn.
    assert_eq!(talk_to(&mut b[0], KPQ.lobby, LAKELIS).await, [(LAKELIS, 0)]);

    // The party plays in its own copy: someone warping onto the map meets
    // nobody, and nobody meets them.
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut b[0].connection, KPQ.entry, "", &mut trace).await,
        KPQ.entry
    );
    read_for(&mut b[0].connection, &mut trace, 500).await;
    assert!(trace.players.is_empty(), "{:?}", trace.players);
    let mut trace = Trace::default();
    read_for(&mut a[0].connection, &mut trace, 500).await;
    let outsider = b[0].character_id;
    assert!(
        trace.players.iter().all(|p| p.character_id != outsider),
        "{:?}",
        trace.players
    );
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut b[0].connection, KPQ.lobby, "", &mut trace).await,
        KPQ.lobby
    );

    // Nella lets a member out, which leaves too few: the quest ends, and
    // the rest are sent out after them.
    assert_eq!(talk_to(&mut a[2], KPQ.entry, NELLA).await, [(NELLA, 1)]);
    let seen = yes(&mut a[2]).await;
    let mut trace = moved_on(&mut a[2], KPQ.exit, seen).await;
    until(&mut a[2].connection, &mut trace, |t| t.clocks_stopped > 0).await;
    for player in a[..2].iter_mut() {
        moved_to(player, KPQ.exit).await;
    }

    // The next party may go in once the copy closes.
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert_eq!(talk_to(&mut b[0], KPQ.lobby, LAKELIS).await, [(LAKELIS, 1)]);
    let mut seen = Some(yes(&mut b[0]).await);
    let mut seconds = 0;
    for player in b.iter_mut() {
        let seen = seen.take().unwrap_or_default();
        let mut trace = moved_on(player, KPQ.entry, seen).await;
        until(&mut player.connection, &mut trace, |t| {
            !t.countdowns.is_empty()
        })
        .await;
        seconds = trace.countdowns[0];
    }

    // One of four leaving still leaves enough to play on...
    assert_eq!(talk_to(&mut b[3], KPQ.entry, NELLA).await, [(NELLA, 1)]);
    let seen = yes(&mut b[3]).await;
    moved_on(&mut b[3], KPQ.exit, seen).await;
    let mut trace = Trace::default();
    read_for(&mut b[0].connection, &mut trace, 500).await;
    assert!(trace.warps.is_empty(), "{:?}", trace.warps);

    // ...until time is up, and everyone left is sent out.
    tokio::time::sleep(Duration::from_secs((seconds as u64).saturating_sub(5))).await;
    for player in b[..3].iter_mut() {
        moved_to(player, KPQ.exit).await;
    }

    // Nella sees them home from the exit.
    assert_eq!(talk_to(&mut b[0], KPQ.exit, NELLA).await, [(NELLA, 1)]);
    let trace = yes(&mut b[0]).await;
    assert_eq!(trace.warps, [KPQ.lobby]);
}
