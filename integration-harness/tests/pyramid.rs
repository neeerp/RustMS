//! Nett's Pyramid. Duarte in its lobby sends a player in alone, or a
//! party with its leader, in the mode they choose. Each stage lasts until
//! its clock runs out, 2 minutes for the first and 3 for the rest, and
//! then the run goes on to the next; after the fifth it ends on the mode's
//! result map. The harness world runs events 60 times as fast, so a run
//! takes 14 seconds. Duarte on a stage lets a player give up, back to the
//! lobby. The run counts its kills and misses, which keep its gauge up or
//! pull it down; once it runs out the run ends. Its kill counts give uses
//! of Rage of Pharaoh, which can't be used without one. On the result map
//! the run's kills give its rank and EXP. A run through all five stages
//! earns a room of Pharaoh Yeti's Tomb, now or with a gem for later.

use game_data::server_content::keys;
use integration_harness::events::{party, said, talk, told, waiting_in};
use integration_harness::hunting::{attack, skill_attack};
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::WorldSession;
use net::events::{
    EventKind, PyramidMode, PYRAMID_GEM, PYRAMID_LOBBY, PYRAMID_RESULT, PYRAMID_SHADES,
    PYRAMID_STAGES, PYRAMID_YETI,
};
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

    // Each stage's time running out takes the player on to the next. After
    // the fifth, without a kill, the result map gives D and no EXP.
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&run.exit) && !t.pyramid_scores.is_empty()
    })
    .await;
    assert_eq!(seen.pyramid_scores, [(4, 0)]);
    let stages: Vec<i32> = (1..=PYRAMID_STAGES).map(|n| run.stage_map(n)).collect();
    assert_eq!(seen.warps, [stages, vec![PYRAMID_RESULT + 1]].concat());
    // The later stages' clocks give 3 minutes (3 s).
    assert_eq!(seen.countdowns.len(), 5, "{:?}", seen.countdowns);
    assert!(seen.countdowns[1..].iter().all(|&s| (2..=3).contains(&s)));
}

const RAGE_OF_PHARAOH: i32 = 1020;

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
            && t.field.mobs.len() >= 17
            && told_value(t, "massacre_skill", 0)
    })
    .await;
    let mobs: Vec<i32> = seen.field.mobs.iter().map(|&(id, _)| id).collect();

    // Rage of Pharaoh needs a use, and the run has given none yet. A kill
    // counts, toward the run and toward Protector of Pharaoh's 7760. It puts
    // back Hell's 1 once the gauge has lost anything, a second into the
    // stage, and nothing before.
    send(
        &mut player.connection,
        skill_attack(RAGE_OF_PHARAOH, mobs[16], 1),
    )
    .await;
    send(&mut player.connection, attack(mobs[0], 1)).await;
    until(&mut player.connection, &mut seen, |t| {
        told_value(t, "massacre_hit", 1) && t.pyramid_gauges.len() == 2
    })
    .await;
    assert!(!seen.field.killed.contains(&mobs[16]));
    assert_eq!(seen.kills.get(&7760).map(String::as_str), Some("1"));
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
    // the run ends on its result map: D, with 2 EXP for the kill.
    for &mob in &mobs[2..16] {
        send(&mut player.connection, attack(mob, 0)).await;
    }
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&run.exit) && !t.pyramid_scores.is_empty()
    })
    .await;
    assert_eq!(seen.pyramid_scores, [(4, 2)]);
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

#[tokio::test]
async fn a_run_through_every_stage_gives_a_gem_for_pharaoh_yetis_tomb() {
    let mut player = waiting_in(PYRAMID_LOBBY, 45, &[]).await;
    let run = EventKind::Pyramid(PyramidMode::Normal).event();
    let tomb = EventKind::PyramidTomb(PyramidMode::Normal).event();
    let gem = PYRAMID_GEM + PyramidMode::Normal.index();
    let mut seen = choose(&mut player, 0, PyramidMode::Normal).await;
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&run.exit) && !t.pyramid_scores.is_empty()
    })
    .await;

    // Duarte sends the player on to the Shades, where, after all five
    // stages, he offers the Tomb now or its gem for later.
    told(&mut player, talk(run.exit, DUARTE)).await;
    send(&mut player.connection, answer(1, 1, None)).await;
    let shades = PYRAMID_SHADES + PyramidMode::Normal.index();
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&shades)
    })
    .await;
    let offer = told(&mut player, talk(shades, DUARTE)).await;
    assert!(
        said(&offer, keys::PYRAMID_DUARTE_SHADES_PASSED),
        "{:?} {:?}",
        offer.prompt_texts,
        seen.warps
    );
    send(&mut player.connection, answer(4, 1, Some(1))).await;
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&PYRAMID_LOBBY) && t.added.iter().any(|&(_, _, id)| id == gem)
    })
    .await;

    // In the lobby he takes the gem for a room of the mode's Tomb, alone
    // with Pharaoh Yeti, whose chest is the player's.
    told(&mut player, talk(PYRAMID_LOBBY, DUARTE)).await;
    told(&mut player, answer(4, 1, Some(2))).await;
    send(
        &mut player.connection,
        answer(4, 1, Some(PyramidMode::Normal.index())),
    )
    .await;
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&tomb.entry) && !t.field.alive(PYRAMID_YETI).is_empty()
    })
    .await;
    assert!(
        seen.removed.iter().any(|&(kind, _)| kind == 4),
        "{:?}",
        seen.removed
    );
    let yeti = seen.field.alive(PYRAMID_YETI)[0];
    send(&mut player.connection, attack(yeti, 1)).await;
    until(&mut player.connection, &mut seen, |t| {
        t.field.item_drop(PYRAMID_YETI_CHEST).is_some()
    })
    .await;
    let chest = seen.field.item_drop(PYRAMID_YETI_CHEST).unwrap();
    assert_eq!(chest.owner, player.character_id);

    // Its way out goes back to the lobby.
    assert_eq!(
        warp(&mut player.connection, -1, "out00", &mut seen).await,
        PYRAMID_LOBBY
    );
}

/// What Pharaoh Yeti drops on Easy to Hard.
const PYRAMID_YETI_CHEST: i32 = 2022613;
