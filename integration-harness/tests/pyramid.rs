//! Nett's Pyramid. Duarte in its lobby sends a player in alone, or a
//! party with its leader, in the mode they choose. Each stage lasts until
//! its clock runs out, 2 minutes for the first and 3 for the rest, and
//! then the run goes on to the next; after the fifth it ends on the mode's
//! result map. The harness world runs events 60 times as fast, so a run
//! takes 14 seconds. Duarte on a stage lets a player give up, back to the
//! lobby. The run counts its kills and misses, which keep its gauge up or
//! pull it down; once it runs out the run ends.

use game_data::server_content::keys;
use integration_harness::events::{party, said, talk, told, waiting_in};
use integration_harness::hunting::attack;
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, Trace};
use integration_harness::WorldSession;
use net::events::{EventKind, PyramidMode, PYRAMID_LOBBY, PYRAMID_RESULT, PYRAMID_STAGES};
use net::npc_scripts::DUARTE;

/// The lobby menu's Enter, then alone (0) or with a party (1), then the
/// mode.
async fn choose(player: &mut WorldSession, party: i32, mode: PyramidMode) -> Trace {
    told(player, talk(PYRAMID_LOBBY, DUARTE)).await;
    told(player, answer(4, 1, Some(1))).await;
    told(player, answer(4, 1, Some(party))).await;
    let seen = Trace::default();
    send(&mut player.connection, answer(4, 1, Some(mode.index()))).await;
    seen
}

/// Duarte turns the player's choice down with `line`, and they stay.
async fn refused(player: &mut WorldSession, party: i32, mode: PyramidMode, line: &str) {
    told(player, talk(PYRAMID_LOBBY, DUARTE)).await;
    told(player, answer(4, 1, Some(1))).await;
    told(player, answer(4, 1, Some(party))).await;
    let reply = told(player, answer(4, 1, Some(mode.index()))).await;
    assert!(reply.prompt_texts.iter().any(|said| said.contains(line)));
    let after = told(player, answer(4, 1, Some(0))).await;
    assert!(reply.warps.is_empty() && after.warps.is_empty());
}

#[tokio::test]
async fn a_run_alone_goes_through_five_timed_stages_to_its_result_map() {
    let mut player = waiting_in(PYRAMID_LOBBY, 45, &[]).await;
    let run = EventKind::Pyramid(PyramidMode::Normal).event();
    let mut seen = choose(&mut player, 0, PyramidMode::Normal).await;

    // The first stage, with its 2 minutes (2 s here) on the clock.
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&run.entry) && !t.countdowns.is_empty()
    })
    .await;
    assert!(seen.countdowns[0] <= 2, "{:?}", seen.countdowns);

    // Each stage's time running out takes the player on to the next.
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&run.exit)
    })
    .await;
    let stages: Vec<i32> = (1..=PYRAMID_STAGES).map(|n| run.stage_map(n)).collect();
    assert_eq!(seen.warps, [stages, vec![PYRAMID_RESULT + 1]].concat());
    // The later stages' clocks give 3 minutes (3 s).
    assert_eq!(seen.countdowns.len(), 5, "{:?}", seen.countdowns);
    assert!(seen.countdowns[1..].iter().all(|&s| (2..=3).contains(&s)));
}

/// Whether the run's tally `key` was last told as `value`.
fn told_value(seen: &Trace, key: &str, value: i32) -> bool {
    let last = seen.session_values.iter().rev().find(|(k, _)| k == key);
    last.is_some_and(|(_, v)| *v == value.to_string())
}

#[tokio::test]
async fn kills_and_misses_are_counted_and_enough_misses_end_the_run() {
    let mut player = waiting_in(PYRAMID_LOBBY, 45, &[]).await;
    let run = EventKind::Pyramid(PyramidMode::Hell).event();
    let mut seen = choose(&mut player, 0, PyramidMode::Hell).await;

    // The stage starts with nothing counted and its gauge full.
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&run.entry)
            && told_value(t, "massacre_miss", 0)
            && t.pyramid_gauges == [0]
            && t.field.mobs.len() >= 16
    })
    .await;
    let mobs: Vec<i32> = seen.field.mobs.iter().map(|&(id, _)| id).collect();

    // A kill counts. It puts back Hell's 1 once the gauge has lost
    // anything, a second into the stage, and nothing before.
    send(&mut player.connection, attack(mobs[0], 1)).await;
    until(&mut player.connection, &mut seen, |t| {
        told_value(t, "massacre_hit", 1) && t.pyramid_gauges.len() == 2
    })
    .await;
    let after_kill = seen.pyramid_gauges[1];
    assert!((0..=1).contains(&after_kill), "{:?}", seen.pyramid_gauges);

    // A miss takes the mob away and Hell's 7 off the gauge.
    send(&mut player.connection, attack(mobs[1], 0)).await;
    until(&mut player.connection, &mut seen, |t| {
        told_value(t, "massacre_miss", 1)
            && t.field.killed.contains(&mobs[1])
            && t.pyramid_gauges.len() == 3
    })
    .await;
    assert_eq!(seen.pyramid_gauges.last(), Some(&(after_kill - 7)));

    // Fourteen more take all its 100 (13 do once a second has gone), and
    // the run ends on its result map.
    for &mob in &mobs[2..16] {
        send(&mut player.connection, attack(mob, 0)).await;
    }
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&run.exit)
    })
    .await;
    let misses = |n| told_value(&seen, "massacre_miss", n);
    assert!(misses(14) || misses(15), "{:?}", seen.session_values);
    assert!(!seen.warps.contains(&run.stage_map(3)), "{:?}", seen.warps);
}

#[tokio::test]
async fn a_party_goes_in_together_and_a_member_may_give_up() {
    let mut team = vec![
        waiting_in(PYRAMID_LOBBY, 70, &[]).await,
        waiting_in(PYRAMID_LOBBY, 45, &[]).await,
    ];
    party(&mut team).await;
    let run = EventKind::PyramidParty(PyramidMode::Hell).event();

    // A member can't take the party in.
    let (leader, rest) = team.split_first_mut().unwrap();
    let member = &mut rest[0];
    let text = net::content::text();
    let not_leader = text.get(keys::PYRAMID_DUARTE_NOT_LEADER, &[]);
    refused(member, 1, PyramidMode::Hell, &not_leader).await;

    // Above level 60 the leader has only Hell.
    let only_hell = text.get(keys::PYRAMID_DUARTE_ONLY_HELL, &[("high", "60")]);
    refused(leader, 1, PyramidMode::Easy, &only_hell).await;

    // Both go in on Hell's first stage.
    let mut leader_seen = choose(leader, 1, PyramidMode::Hell).await;
    let mut member_seen = Trace::default();
    until(&mut leader.connection, &mut leader_seen, |t| {
        t.warps.contains(&run.entry)
    })
    .await;
    until(&mut member.connection, &mut member_seen, |t| {
        t.warps.contains(&run.entry)
    })
    .await;

    // The member gives up and goes back to the lobby; the leader goes on.
    let forfeit = told(member, talk(run.entry, DUARTE)).await;
    assert!(said(&forfeit, keys::PYRAMID_DUARTE_FORFEIT_ASK));
    send(&mut member.connection, answer(1, 1, None)).await;
    until(&mut member.connection, &mut member_seen, |t| {
        t.warps.contains(&PYRAMID_LOBBY)
    })
    .await;
    until(&mut leader.connection, &mut leader_seen, |t| {
        t.warps.contains(&run.stage_map(2))
    })
    .await;
    assert!(!member_seen.warps.contains(&run.stage_map(2)));
}
