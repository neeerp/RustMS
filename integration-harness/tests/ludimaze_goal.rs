//! The Ludibrium Maze's goal: Pietri there serves only the leader, and
//! only once the whole team has found its way to him; then he takes all
//! the leader's coupons, every member earns 50 EXP for each, and the team
//! goes on to the clear room, off the clock, where Rolly gives each a
//! present and sends them out. The maze lasts 15 seconds here
//! (`RUSTMS_EVENT_SCALE`).

use game_data::server_content::keys;
use integration_harness::events::{party, talk, told, waiting_in};
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{
    LUDIBRIUM, MAZE_CLEAR, MAZE_COUPON, MAZE_COUPON_EXP, MAZE_EXIT, MAZE_GOAL, MAZE_REWARDS,
    MAZE_ROOM,
};
use net::npc_scripts::{PIETRI, ROLLY, ROLLY_PRESENT};
use tokio::time::Duration;

const COUPONS: i16 = 35;

/// Straight to the goal room, by debug warp, which no portal checks.
async fn to_goal(player: &mut WorldSession) {
    let mut trace = Trace::default();
    assert_eq!(
        warp(&mut player.connection, MAZE_GOAL, "", &mut trace).await,
        MAZE_GOAL
    );
}

#[tokio::test]
async fn pietri_takes_the_leaders_coupons_once_the_team_is_all_there() {
    let mut team = vec![waiting_in(LUDIBRIUM, 51, &[(MAZE_COUPON, COUPONS)]).await];
    for _ in 0..2 {
        team.push(waiting_in(LUDIBRIUM, 51, &[]).await);
    }
    party(&mut team).await;

    // The leader takes the party in once any party before them is out.
    let in_room = |map: &i32| (MAZE_ROOM..MAZE_GOAL).contains(map);
    let mut seen = None;
    for _ in 0..20 {
        told(&mut team[0], talk(LUDIBRIUM, ROLLY)).await;
        let trace = told(&mut team[0], answer(4, 1, Some(0))).await;
        if !trace.prompts.iter().any(|&(from, _)| from == ROLLY) {
            seen = Some(trace);
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    assert!(seen.is_some(), "Rolly never let the party in");
    for player in team.iter_mut() {
        let mut trace = seen.take().unwrap_or_default();
        until(&mut player.connection, &mut trace, |t| {
            t.warps.iter().any(in_room)
        })
        .await;
    }

    // Alone at the goal, the leader is told to wait for the others; a
    // member is sent to the leader.
    to_goal(&mut team[0]).await;
    let says = |trace: &Trace| trace.prompt_texts.join("\n");
    let wait = net::content::text().get(keys::LUDIMAZE_PIETRI_WAIT, &[]);
    let trace = told(&mut team[0], talk(MAZE_GOAL, PIETRI)).await;
    assert!(says(&trace).contains(&wait), "{trace:?}");
    to_goal(&mut team[1]).await;
    let not_leader = net::content::text().get(keys::LUDIMAZE_PIETRI_NOT_LEADER, &[]);
    let trace = told(&mut team[1], talk(MAZE_GOAL, PIETRI)).await;
    assert!(says(&trace).contains(&not_leader), "{trace:?}");

    // With all three there, the leader hands in every coupon.
    to_goal(&mut team[2]).await;
    let trace = told(&mut team[0], talk(MAZE_GOAL, PIETRI)).await;
    assert_eq!(trace.prompts, [(PIETRI, 1)], "{trace:?}");
    assert!(says(&trace).contains(&COUPONS.to_string()), "{trace:?}");
    let mut trace = told(&mut team[0], answer(1, 1, None)).await;
    until(&mut team[0].connection, &mut trace, |t| {
        t.removed.contains(&(4, 1))
    })
    .await;

    // Everyone earns the coupons' EXP and goes on to the clear room, where
    // no clock runs.
    let exp = i32::from(COUPONS) * MAZE_COUPON_EXP;
    for (i, player) in team.iter_mut().enumerate() {
        let mut seen = if i == 0 {
            std::mem::take(&mut trace)
        } else {
            Trace::default()
        };
        until(&mut player.connection, &mut seen, |t| {
            t.gains.contains(&exp) && t.clocks_stopped > 0 && t.warps.contains(&MAZE_CLEAR)
        })
        .await;
    }

    // Rolly there gives each who asks one present and sends them out.
    for player in team.iter_mut() {
        let trace = told(player, talk(MAZE_CLEAR, ROLLY_PRESENT)).await;
        assert_eq!(trace.prompts, [(ROLLY_PRESENT, 1)], "{trace:?}");
        let trace = told(player, answer(1, 1, None)).await;
        assert_eq!(trace.warps, [MAZE_EXIT], "{trace:?}");
        assert!(
            (trace.items.iter()).any(|&(id, n)| MAZE_REWARDS.contains(&(id, n as i16))),
            "{trace:?}"
        );
    }
}
