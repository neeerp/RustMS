//! A party in Kerning City's party quest that passes its lead on: a new
//! leader in the quest leads it from then on, so the old one may leave it,
//! and a new leader who is not in it ends it for everyone (Cosmic
//! `changedLeader`). The harness world runs events 60 times as fast
//! (`RUSTMS_EVENT_SCALE`), so the quest lasts 30 seconds.

use integration_harness::events::{enter_kpq, in_lobby, party, pass_lead, talk, CLOTO, NELLA};
use integration_harness::quests::answer;
use integration_harness::trace::{send, step, until, Trace};
use integration_harness::WorldSession;
use net::events::KPQ;
use tokio::time::Duration;

/// PARTY_OPERATION's new leader.
const OP_CHANGE_LEADER: u8 = 0x1B;

/// What Cloto says to `player` on stage 1, which differs for the leader.
async fn ask_cloto(player: &mut WorldSession) -> String {
    let name = player.character_name.clone();
    let trace = step(&mut player.connection, &name, talk(KPQ.entry, CLOTO)).await;
    assert_eq!(trace.prompts, [(CLOTO, 0)], "{trace:?}");
    trace.prompt_texts[0].clone()
}

/// `from` passes the lead to `to`, and `to` hears of it.
async fn hand_over(from: &mut WorldSession, to: &mut WorldSession) {
    send(&mut from.connection, pass_lead(to.character_id)).await;
    let mut trace = Trace::default();
    until(&mut to.connection, &mut trace, |t| {
        t.party_ops.contains(&OP_CHANGE_LEADER)
    })
    .await;
}

/// What Cloto says to `player` once it is `expected`: the event hears of a
/// new leader a moment after the party does.
async fn cloto_comes_to(player: &mut WorldSession, expected: &str) {
    let mut told = String::new();
    for _ in 0..20 {
        told = ask_cloto(player).await;
        if told == expected {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("Cloto still says {told:?}, not {expected:?}");
}

/// Wait until `player` is moved to `map`, carrying on from `trace`.
async fn moved_to(player: &mut WorldSession, map: i32, mut trace: Trace) {
    until(&mut player.connection, &mut trace, |t| {
        t.warps.contains(&map)
    })
    .await;
}

#[tokio::test]
async fn a_new_leader_in_the_quest_leads_it_and_one_outside_ends_it() {
    let mut a = Vec::new();
    for _ in 0..4 {
        a.push(in_lobby(25, &[]).await);
    }
    party(&mut a).await;
    enter_kpq(&mut a).await;
    let leading = ask_cloto(&mut a[0]).await;
    let asked = ask_cloto(&mut a[1]).await;
    assert_ne!(asked, leading);

    // The lead goes to a member in the quest, whom Cloto now treats as
    // its leader.
    let (old, rest) = a.split_first_mut().unwrap();
    hand_over(old, &mut rest[0]).await;
    cloto_comes_to(&mut rest[0], &leading).await;

    // The old leader leaves, and the quest goes on with the other three:
    // the leader now needs a pass fewer.
    let name = old.character_name.clone();
    let trace = step(&mut old.connection, &name, talk(KPQ.entry, NELLA)).await;
    assert_eq!(trace.prompts, [(NELLA, 1)], "{trace:?}");
    let seen = step(&mut old.connection, &name, answer(1, 1, None)).await;
    moved_to(old, KPQ.exit, seen).await;
    let leading_three = ask_cloto(&mut rest[0]).await;
    assert!(
        leading_three != leading && leading_three != asked,
        "{leading_three}"
    );

    // Passed back to them, outside it, the lead ends it.
    hand_over(&mut rest[0], old).await;
    for player in rest.iter_mut() {
        moved_to(player, KPQ.exit, Trace::default()).await;
    }
}
