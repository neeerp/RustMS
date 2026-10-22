//! Ludibrium's party quest, its balloons: the party's leader first hears
//! out what a stage asks, then hands its balloon the stage's passes, which
//! clears it for everyone and opens the way on; the others are sent to
//! fetch the leader. The last stage's balloon takes Alishar's key and wins
//! the quest. The harness world runs events 60 times as fast
//! (`RUSTMS_EVENT_SCALE`), so the quest lasts 45 seconds.

use game_data::server_content::keys;
use integration_harness::events::{enter_lpq, party, talk, waiting_in};
use integration_harness::packets::build_change_map;
use integration_harness::trace::{send, step, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{HandIn, LPQ, LPQ_KEY, LPQ_PASS};
use net::packet::op::SendOpcode;

/// The trace of `player` sending `packet`, up to the barrier.
async fn told(player: &mut WorldSession, packet: packet::Packet) -> Trace {
    let name = player.character_name.clone();
    step(&mut player.connection, &name, packet).await
}

/// What `stage`'s balloon says to `player`, and the trace of it.
async fn ask_balloon(player: &mut WorldSession, stage: u8) -> (String, Trace) {
    let hand_in = LPQ.hand_in(stage).unwrap();
    let trace = told(player, talk(LPQ.stage_map(stage), hand_in.npc)).await;
    assert_eq!(trace.prompts, [(hand_in.npc, 0)], "{trace:?}");
    (trace.prompt_texts[0].clone(), trace)
}

/// `key`, with `hand_in`'s item and count.
fn about(key: &str, hand_in: &HandIn) -> String {
    let (item, n) = (hand_in.item.to_string(), hand_in.count.to_string());
    net::content::text().get(key, &[("item", &item), ("n", &n)])
}

fn clear_effects() -> Vec<(u8, String)> {
    vec![
        (3, "quest/party/clear".into()),
        (4, "Party1/Clear".into()),
        (2, "gate".into()),
    ]
}

#[tokio::test]
async fn the_leader_hears_each_balloon_out_then_hands_in_its_passes() {
    let first = LPQ.hand_in(1).unwrap();
    let mut a = vec![waiting_in(LPQ.lobby, 35, &[(LPQ_PASS, 25), (LPQ_KEY, 1)]).await];
    for _ in 0..4 {
        a.push(waiting_in(LPQ.lobby, 35, &[]).await);
    }
    party(&mut a).await;
    enter_lpq(&mut a).await;
    let text = net::content::text();

    // A member is sent to fetch the leader; the leader's first talk only
    // tells what the stage asks, and the way on stays shut.
    let (said, _) = ask_balloon(&mut a[1], 1).await;
    assert!(
        said.contains(&text.get(keys::LPQ_BALLOON_NOT_LEADER, &[])),
        "{said}"
    );
    let (said, trace) = ask_balloon(&mut a[0], 1).await;
    assert!(
        said.contains(&about(keys::LPQ_BALLOON_INTRO_1, first)),
        "{said}"
    );
    assert!(
        trace.removed.is_empty() && trace.gains.is_empty(),
        "{trace:?}"
    );
    send(
        &mut a[0].connection,
        build_change_map(-1, "next00").unwrap(),
    )
    .await;
    let mut trace = Trace::default();
    until(&mut a[0].connection, &mut trace, |t| !t.blocked.is_empty()).await;
    assert_eq!(trace.blocked, [(SendOpcode::BlockedMap as i16, 1)]);

    // The next talk takes the passes and clears the stage: everyone sees it
    // and gains its EXP.
    let (said, mut trace) = ask_balloon(&mut a[0], 1).await;
    assert!(
        said.contains(&text.get(keys::LPQ_BALLOON_DONE, &[])),
        "{said}"
    );
    assert!(trace.removed.contains(&(4, 1)), "{trace:?}");
    let mut seen = Some(trace);
    for player in a.iter_mut() {
        trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| !t.gains.is_empty()).await;
        assert_eq!(trace.field_effects, clear_effects());
        assert_eq!(trace.gains, [LPQ.stage_exp(1)]);
    }

    // Asked again, the balloon hurries anyone on, and the way is open.
    let (said, trace) = ask_balloon(&mut a[1], 1).await;
    assert!(
        said.contains(&text.get(keys::LPQ_BALLOON_HURRY, &[])),
        "{said}"
    );
    assert!(trace.gains.is_empty(), "{trace:?}");
    let seen = told(&mut a[0], build_change_map(-1, "next00").unwrap()).await;
    assert_eq!(seen.warps, [LPQ.stage_map(2)], "{seen:?}");

    // On the last stage, by debug warp, the leader hands in the key, which
    // wins the quest: its EXP for everyone, and the clock stopped.
    let last = LPQ.last_stage();
    let mut trace = Trace::default();
    let map = LPQ.stage_map(last);
    assert_eq!(warp(&mut a[0].connection, map, "", &mut trace).await, map);
    let (said, _) = ask_balloon(&mut a[0], last).await;
    let key = LPQ.hand_in(last).unwrap();
    assert!(
        said.contains(&about(keys::LPQ_BALLOON_INTRO_9, key)),
        "{said}"
    );
    let (_, trace) = ask_balloon(&mut a[0], last).await;
    let mut seen = Some(trace);
    for player in a.iter_mut() {
        let mut trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| t.clocks_stopped > 0).await;
        assert_eq!(trace.gains, [LPQ.stage_exp(last)]);
    }
}
