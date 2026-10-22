//! Ludibrium's party quest, won: the leader hands the last balloon Alishar's
//! key, and the win takes the whole team to the bonus stage at once, on a
//! clock of its own. When it runs out everyone there goes on to the reward,
//! where Arturo gives each member one of the quest's rewards and sends them
//! back to Eos Tower. The harness world runs events 60 times as fast
//! (`RUSTMS_EVENT_SCALE`), so the bonus lasts a second.

use integration_harness::events::{enter_lpq, party, talk, waiting_in};
use integration_harness::quests::answer;
use integration_harness::trace::{step, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{LPQ, LPQ_KEY, LPQ_REWARDS};
use net::npc_scripts::ARTURO;

/// The trace of `player` sending `packet`, up to the barrier.
async fn told(player: &mut WorldSession, packet: packet::Packet) -> Trace {
    let name = player.character_name.clone();
    step(&mut player.connection, &name, packet).await
}

#[tokio::test]
async fn the_win_takes_the_team_to_the_bonus_then_arturo_rewards_each() {
    let bonus = LPQ.bonus.as_ref().unwrap();
    let mut a = vec![waiting_in(LPQ.lobby, 35, &[(LPQ_KEY, 1)]).await];
    for _ in 0..4 {
        a.push(waiting_in(LPQ.lobby, 35, &[]).await);
    }
    party(&mut a).await;
    enter_lpq(&mut a).await;

    // By debug warp, which no gate checks, to the last stage, where the
    // leader hears the balloon out and then hands in the key.
    let last = LPQ.last_stage();
    let map = LPQ.stage_map(last);
    for player in a.iter_mut() {
        let mut trace = Trace::default();
        assert_eq!(warp(&mut player.connection, map, "", &mut trace).await, map);
    }
    let balloon = LPQ.hand_in(last).unwrap().npc;
    told(&mut a[0], talk(map, balloon)).await;
    let mut seen = Some(told(&mut a[0], talk(map, balloon)).await);

    // Everyone gains the stage's EXP and is taken to the bonus with its
    // clock, then on to the reward once it runs out.
    for player in a.iter_mut() {
        let mut trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.contains(&bonus.exit)
        })
        .await;
        assert_eq!(trace.gains, [LPQ.stage_exp(last)]);
        assert_eq!(trace.warps, [bonus.map, bonus.exit], "{trace:?}");
        assert!(!trace.countdowns.is_empty(), "{trace:?}");
    }

    // Arturo gives each who comes to him a reward and sends them home.
    for player in a[..2].iter_mut() {
        let trace = told(player, talk(bonus.exit, ARTURO)).await;
        assert_eq!(trace.prompts, [(ARTURO, 0)], "{trace:?}");
        let trace = told(player, answer(0, 1, None)).await;
        assert_eq!(trace.warps, [LPQ.lobby], "{trace:?}");
        assert!(
            (trace.items.iter()).any(|&(id, n)| LPQ_REWARDS.contains(&(id, n as i16))),
            "{trace:?}"
        );
    }
}
