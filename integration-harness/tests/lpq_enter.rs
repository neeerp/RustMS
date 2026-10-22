//! Ludibrium's party quest, its first slice: the Red Sign on Eos Tower's
//! 101st floor takes those of a party of five or six at levels 35 to 50
//! who are there into its own copy of the Dimensional Crack; Sgt. Anderson
//! lets players out, with the quest's passes taken, and sends them down
//! again from the way out; and the quest ends for everyone when too few
//! remain. The harness world runs events 60 times as fast
//! (`RUSTMS_EVENT_SCALE`), so the quest lasts 45 seconds.

use game_data::server_content::keys;
use integration_harness::events::{enter_lpq, party, talk, waiting_in, ANDERSON, RED_SIGN};
use integration_harness::quests::answer;
use integration_harness::trace::{read_for, step, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::LPQ;

/// The trace of `player` sending `packet`, up to the barrier.
async fn told(player: &mut WorldSession, packet: packet::Packet) -> Trace {
    let name = player.character_name.clone();
    step(&mut player.connection, &name, packet).await
}

/// Wait until `player` is moved to `map`, carrying on from `trace`.
async fn moved_on(player: &mut WorldSession, map: i32, mut trace: Trace) -> Trace {
    until(&mut player.connection, &mut trace, |t| {
        t.warps.contains(&map)
    })
    .await;
    trace
}

/// What the Red Sign tells `player` on being asked to take the party in.
async fn asked_in(player: &mut WorldSession) -> Trace {
    let menu = told(player, talk(LPQ.lobby, RED_SIGN)).await;
    assert_eq!(menu.prompts, [(RED_SIGN, 4)]);
    told(player, answer(4, 1, Some(0))).await
}

/// `key`, with who may play.
fn terms(key: &str) -> String {
    let (team, levels) = (&LPQ.team, &LPQ.levels);
    net::content::text().get(
        key,
        &[
            ("least", &team.start().to_string()),
            ("most", &team.end().to_string()),
            ("low", &levels.start().to_string()),
            ("high", &levels.end().to_string()),
        ],
    )
}

#[tokio::test]
async fn the_red_sign_takes_five_in_and_anderson_lets_them_out() {
    // Six in the party: one holds a pass, one is too high a level, and one
    // is elsewhere at first.
    let mut a = vec![waiting_in(LPQ.lobby, 35, &[]).await];
    a.push(waiting_in(LPQ.lobby, 35, &[(4001022, 1)]).await);
    for _ in 0..3 {
        a.push(waiting_in(LPQ.lobby, 35, &[]).await);
    }
    a.push(waiting_in(LPQ.lobby, 51, &[]).await);
    party(&mut a).await;
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut a[4].connection, 221_000_000, "", &mut trace).await,
        221_000_000
    );

    // Only the party's leader may take it in, and only with five of the
    // right levels there.
    let trace = asked_in(&mut a[1]).await;
    assert_eq!(trace.prompts, [(RED_SIGN, 0)]);
    let not_leader = net::content::text().get(keys::LPQ_SIGN_NOT_LEADER, &[]);
    assert!(trace.prompt_texts[0].contains(&not_leader), "{trace:?}");
    let trace = asked_in(&mut a[0]).await;
    assert_eq!(trace.prompts, [(RED_SIGN, 0)]);
    let not_eligible = terms(keys::LPQ_SIGN_NOT_ELIGIBLE);
    assert!(trace.prompt_texts[0].contains(&not_eligible), "{trace:?}");

    // With the fifth back, those five go in at once, on the clock; the one
    // of level 51 stays.
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut a[4].connection, LPQ.lobby, "", &mut trace).await,
        LPQ.lobby
    );
    let traces = enter_lpq(&mut a[..5]).await;
    for (player, trace) in a[..5].iter_mut().zip(traces) {
        let mut trace = trace;
        until(&mut player.connection, &mut trace, |t| {
            !t.countdowns.is_empty()
        })
        .await;
        assert!((30..=45).contains(&trace.countdowns[0]), "{trace:?}");
    }
    let mut trace = Trace::default();
    read_for(&mut a[5].connection, &mut trace, 500).await;
    assert!(trace.warps.is_empty(), "{:?}", trace.warps);

    // Anderson lets one out, with the pass taken, which leaves too few: the
    // quest ends, and the rest follow to the way out.
    let trace = told(&mut a[1], talk(LPQ.entry, ANDERSON)).await;
    assert_eq!(trace.prompts, [(ANDERSON, 1)]);
    let seen = told(&mut a[1], answer(1, 1, None)).await;
    let mut trace = moved_on(&mut a[1], LPQ.exit, seen).await;
    until(&mut a[1].connection, &mut trace, |t| {
        t.clocks_stopped > 0 && t.removed.contains(&(4, 1))
    })
    .await;
    for i in [0, 2, 3, 4] {
        moved_on(&mut a[i], LPQ.exit, Trace::default()).await;
    }

    // From the way out he sends them down to Eos Tower.
    for player in a[..5].iter_mut() {
        let trace = told(player, talk(LPQ.exit, ANDERSON)).await;
        assert_eq!(trace.prompts, [(ANDERSON, 0)]);
        assert_eq!(told(player, answer(0, 1, None)).await.warps, [LPQ.lobby]);
    }
}
