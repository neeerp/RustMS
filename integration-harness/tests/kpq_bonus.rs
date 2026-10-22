//! Kerning City's party quest, after the win: each member who talks to
//! Cloto on stage 5 gets one of the rewards and goes to the bonus stage,
//! where Nella asks a question of her own and lets them out, taking the
//! party quest's own items.

use integration_harness::events::{enter_kpq, in_lobby, party, talk, CLOTO, NELLA};
use integration_harness::quests::answer;
use integration_harness::trace::{barrier, step, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{COUPON, KPQ, KPQ_BONUS, KPQ_REWARDS, PASS};

const ETC: u8 = 4;

/// The trace of `player` sending `packet`, up to the barrier.
async fn told(player: &mut WorldSession, packet: packet::Packet) -> Trace {
    let name = player.character_name.clone();
    step(&mut player.connection, &name, packet).await
}

/// What Nella asks `player` on `map`.
async fn ask_nella(player: &mut WorldSession, map: i32) -> String {
    let trace = told(player, talk(map, NELLA)).await;
    assert_eq!(trace.prompts, [(NELLA, 1)], "{trace:?}");
    trace.prompt_texts[0].clone()
}

#[tokio::test]
async fn cloto_rewards_each_winner_on_the_way_to_the_bonus_stage() {
    let mut team = vec![
        in_lobby(25, &[(PASS, 10)]).await,
        in_lobby(25, &[(COUPON, 3)]).await,
        in_lobby(25, &[]).await,
    ];
    party(&mut team).await;
    enter_kpq(&mut team).await;
    let last = KPQ.stage_map(5);
    for player in team.iter_mut() {
        let name = player.character_name.clone();
        let mut trace = Trace::default();
        assert_eq!(
            warp(&mut player.connection, last, "", &mut trace).await,
            last
        );
        barrier(&mut player.connection, &name, &mut trace).await;
    }

    // Nella's question on the stage, before the win.
    let leave = ask_nella(&mut team[1], last).await;
    told(&mut team[1], answer(1, 0, None)).await;

    // The leader's ten passes win it.
    let mut trace = told(&mut team[0], talk(last, CLOTO)).await;
    assert_eq!(trace.removed, [(ETC, 1)], "{trace:?}");
    until(&mut team[0].connection, &mut trace, |t| {
        t.clocks_stopped > 0
    })
    .await;

    // Each who comes to her then is congratulated, given a reward and sent
    // to the bonus stage.
    for player in team.iter_mut() {
        let trace = told(player, talk(last, CLOTO)).await;
        assert_eq!(trace.prompts, [(CLOTO, 0)], "{trace:?}");
        assert!(
            trace.items.is_empty() && trace.warps.is_empty(),
            "{trace:?}"
        );
        let trace = told(player, answer(0, 1, None)).await;
        assert_eq!(trace.warps, [KPQ_BONUS], "{trace:?}");
        assert!(
            (trace.items.iter()).any(|&(id, n)| KPQ_REWARDS.contains(&(id, n as i16))),
            "{trace:?}"
        );
    }

    // There Nella only asks whether they are ready to go, and lets them out
    // to the exit, taking the coupons.
    let ready = ask_nella(&mut team[1], KPQ_BONUS).await;
    assert_ne!(ready, leave);
    let trace = told(&mut team[1], answer(1, 1, None)).await;
    assert_eq!(trace.warps, [KPQ.exit], "{trace:?}");
    assert!(
        trace.removed.iter().any(|&(tab, _)| tab == ETC),
        "{trace:?}"
    );
    let [leader, _, other] = &mut team[..] else {
        unreachable!()
    };
    for player in [leader, other] {
        assert_eq!(ask_nella(player, KPQ_BONUS).await, ready);
        let trace = told(player, answer(1, 1, None)).await;
        assert_eq!(trace.warps, [KPQ.exit], "{trace:?}");
    }
}
