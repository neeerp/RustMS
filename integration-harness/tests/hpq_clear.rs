//! Henesys's party quest, won: the party's leader hands Growlie 10 rice
//! cakes, which clears the quest for everyone and takes them to where Tory
//! and Tommy wait. Tory gives each member who comes to her the reward and
//! sends them home; Tommy lets the leader take the rest to Pig Town for a
//! while, after which they are moved to its exit, where Tory waits again.
//! The harness world runs events 60 times as fast (`RUSTMS_EVENT_SCALE`),
//! so the quest lasts 10 seconds and Pig Town 5.

use game_data::server_content::keys;
use integration_harness::events::{enter_hpq, party, talk, waiting_in, GROWLIE, TOMMY, TORY};
use integration_harness::quests::answer;
use integration_harness::trace::{step, until, Trace};
use integration_harness::WorldSession;
use net::events::{HPQ, HPQ_REWARD, RICE_CAKE};

/// The trace of `player` sending `packet`, up to the barrier.
async fn told(player: &mut WorldSession, packet: packet::Packet) -> Trace {
    let name = player.character_name.clone();
    step(&mut player.connection, &name, packet).await
}

/// Say yes to the open prompt.
async fn yes(player: &mut WorldSession) -> Trace {
    told(player, answer(1, 1, None)).await
}

/// Wait until `player` is moved to `map`, carrying on from `trace`.
async fn moved_on(player: &mut WorldSession, map: i32, mut trace: Trace) -> Trace {
    until(&mut player.connection, &mut trace, |t| {
        t.warps.contains(&map)
    })
    .await;
    trace
}

/// Tory, on `map`, gives `player` the reward and sends them home.
async fn rewarded(player: &mut WorldSession, map: i32) {
    assert_eq!(told(player, talk(map, TORY)).await.prompts, [(TORY, 1)]);
    let trace = yes(player).await;
    assert_eq!(trace.warps, [HPQ.lobby], "{trace:?}");
    assert!(trace.items.contains(&(HPQ_REWARD, 1)), "{trace:?}");
}

#[tokio::test]
async fn the_leader_hands_in_the_cakes_and_the_party_wins() {
    let clear_map = HPQ.clear_map.unwrap();
    let bonus = HPQ.bonus.as_ref().unwrap();
    let mut a = vec![waiting_in(HPQ.lobby, 10, &[(RICE_CAKE, 10)]).await];
    for _ in 0..2 {
        a.push(waiting_in(HPQ.lobby, 10, &[]).await);
    }
    party(&mut a).await;
    enter_hpq(&mut a).await;

    // Only the leader may hand Growlie the cakes.
    let text = net::content::text();
    let cakes = text.get(keys::HPQ_GROWLIE_CAKES, &[]);
    let menu = told(&mut a[1], talk(HPQ.entry, GROWLIE)).await;
    assert_eq!(menu.prompts, [(GROWLIE, 4)]);
    assert!(!menu.prompt_texts[0].contains(&cakes), "{menu:?}");
    let menu = told(&mut a[0], talk(HPQ.entry, GROWLIE)).await;
    assert!(menu.prompt_texts[0].contains(&cakes), "{menu:?}");

    // Handing them in clears the quest: everyone sees it, gains its EXP,
    // and goes on, with the clock stopped.
    let seen = told(&mut a[0], answer(4, 1, Some(1))).await;
    assert_eq!(seen.prompts, [(GROWLIE, 0)]);
    assert!(seen.removed.contains(&(4, 1)), "{seen:?}");
    let mut seen = Some(seen);
    for player in a.iter_mut() {
        let mut trace = moved_on(player, clear_map, seen.take().unwrap_or_default()).await;
        until(&mut player.connection, &mut trace, |t| {
            !t.gains.is_empty() && t.clocks_stopped > 0
        })
        .await;
        assert_eq!(trace.gains, [HPQ.stage_exp(1)]);
        assert_eq!(
            trace.field_effects,
            [
                (3, "quest/party/clear".to_string()),
                (4, "Party1/Clear".to_string())
            ]
        );
        assert_eq!(trace.clocks_stopped, 1);
    }

    // Tory gives one member the reward and sends them home.
    rewarded(&mut a[2], clear_map).await;

    // Tommy tells the others of Pig Town; only the leader may take them.
    let tommy = told(&mut a[1], talk(clear_map, TOMMY)).await;
    assert_eq!(tommy.prompts, [(TOMMY, 0)]);
    let tommy = told(&mut a[1], answer(0, 1, None)).await;
    assert_eq!(tommy.prompts, [(TOMMY, 0)]);
    let leader = text.get(keys::HPQ_TOMMY_PIGS_LEADER, &[]);
    assert!(tommy.prompt_texts[0].contains(&leader), "{tommy:?}");
    told(&mut a[0], talk(clear_map, TOMMY)).await;
    assert_eq!(
        told(&mut a[0], answer(0, 1, None)).await.prompts,
        [(TOMMY, 1)]
    );
    let mut seen = Some(yes(&mut a[0]).await);
    for player in a[..2].iter_mut() {
        let mut trace = moved_on(player, bonus.map, seen.take().unwrap_or_default()).await;
        until(&mut player.connection, &mut trace, |t| {
            !t.countdowns.is_empty()
        })
        .await;
        assert!((3..=5).contains(&trace.countdowns[0]), "{trace:?}");
    }

    // One leaves Pig Town early; the leader stays until the time is up and
    // is moved to its exit, the clock stopped. Tory rewards both there.
    let trace = told(&mut a[1], talk(bonus.map, TOMMY)).await;
    assert_eq!(trace.prompts, [(TOMMY, 1)]);
    assert_eq!(yes(&mut a[1]).await.warps, [bonus.exit]);
    let mut trace = moved_on(&mut a[0], bonus.exit, Trace::default()).await;
    until(&mut a[0].connection, &mut trace, |t| t.clocks_stopped > 0).await;
    for player in a[..2].iter_mut() {
        rewarded(player, bonus.exit).await;
    }
}
