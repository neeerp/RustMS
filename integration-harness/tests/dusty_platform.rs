//! The Abandoned Subway's Dusty Platform. The Dimensional Mirror sends one
//! of level 25 to 30 down to the station, whose `out00` leads back. Mr.
//! Lim there takes a party's leader in with those of the party's members
//! of the same levels, even a party of one. The run is a Massacre run, as
//! Nett's Pyramid's are: three stages of 3 minutes (3 s, as the harness
//! world runs events 60 times as fast), its kills counted toward the
//! Honorary Employee medal's 7662 and its mirror 7762, and a rank and EXP
//! on the Dusty Platform's result map. There Mr. Lim puts one on Train
//! 999 at once, or gives a Boarding Pass for it and sends one back up; at
//! the station he takes the pass for it. Train 999's platform is a minute
//! alone with 30 Metro Bubblings. One on the result map without a run is
//! only sent up.

use game_data::server_content::keys;
use integration_harness::database::{set_job, set_quest};
use integration_harness::events::{party_of_one, said, talk, told, waiting_in};
use integration_harness::hunting::attack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::WorldSession;
use integration_harness::{login_to_world_session, relogin};
use net::events::{
    EventKind, METRO_BUBBLING, SUBWAY_ARRIVAL_PORTAL, SUBWAY_LOBBY, SUBWAY_PASS, SUBWAY_RESULT,
    SUBWAY_STAGES, SUBWAY_TRAIN_ROOM,
};
use net::npc_scripts::{DIMENSIONAL_MIRROR, MR_LIM};
use net::packet::op::RecvOpcode;
use packet::io::write::PktWrite;
use packet::Packet;

const HENESYS: i32 = 100_000_000;

/// CHANGE_MAP_SPECIAL: press Up on the scripted portal `portal`.
fn enter(portal: &str) -> Packet {
    let mut p = Packet::new_empty();
    p.write_short(RecvOpcode::ChangeMapSpecial as i16).unwrap();
    p.write_byte(0).unwrap();
    p.write_str_with_length(portal).unwrap();
    p.write_short(0).unwrap();
    p
}

fn text(key: &str) -> String {
    net::content::text().get(key, &[])
}

/// Whether the run's tally `key` was last told as `value`.
fn told_value(seen: &Trace, key: &str, value: i32) -> bool {
    let last = seen.session_values.iter().rev().find(|(k, _)| k == key);
    last.is_some_and(|(_, v)| *v == value.to_string())
}

/// Train 999's platform, just boarded: 30 Metro Bubblings, and the time
/// running out (well under a second here, too soon for a kill) sends the
/// rider back to the station.
async fn ride_train_999(player: &mut WorldSession, mut seen: Trace) {
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&SUBWAY_TRAIN_ROOM) && t.field.mobs.len() >= 30
    })
    .await;
    assert!(!seen.countdowns.is_empty(), "{seen:?}");
    assert!(
        seen.field.mobs.iter().all(|m| m.1 == METRO_BUBBLING),
        "{seen:?}"
    );
    assert!(seen
        .field_effects
        .contains(&(3, "killing/bonus/bonus".to_string())));
    until(&mut player.connection, &mut seen, |t| {
        t.arrivals.iter().any(|a| a.0 == SUBWAY_LOBBY)
    })
    .await;
}

/// A level-27 character the Mirror has sent from Henesys to the station.
async fn at_the_station() -> WorldSession {
    let mut player = waiting_in(HENESYS, 27, &[]).await;
    let menu = told(&mut player, talk(HENESYS, DIMENSIONAL_MIRROR)).await;
    let expected = format!(
        "#0#{}#1#{}#6#{}",
        text(keys::MIRROR_ARIANT),
        text(keys::MIRROR_DOJO),
        text(keys::MIRROR_SUBWAY)
    );
    assert_eq!(menu.prompt_texts, [expected]);
    let went = told(&mut player, answer(14, 1, Some(6))).await;
    assert!(
        matches!(
            went.arrivals[..],
            [(SUBWAY_LOBBY, SUBWAY_ARRIVAL_PORTAL, _)]
        ),
        "{went:?}"
    );
    player
}

#[tokio::test]
async fn the_station_warns_opens_mr_lim_and_leads_back_to_the_mirrors_town() {
    let mut player = at_the_station().await;
    let warned = told(&mut player, enter("chat00")).await;
    assert_eq!(warned.scene_parts, ["Effect/Direction2.img/metro/Im"]);
    assert!(warned.arrivals.is_empty(), "{warned:?}");
    let opened = told(&mut player, enter("metro00")).await;
    assert_eq!(opened.prompts, [(MR_LIM, 4)], "{opened:?}");
    told(&mut player, answer(4, 0, None)).await;
    let back = told(&mut player, enter("out00")).await;
    assert!(matches!(back.arrivals[..], [(HENESYS, _, _)]), "{back:?}");
}

#[tokio::test]
async fn a_party_runs_the_dusty_platforms_three_stages_to_its_result() {
    let mut player = at_the_station().await;
    let run = EventKind::Subway.event();

    // No party, no run.
    told(&mut player, talk(SUBWAY_LOBBY, MR_LIM)).await;
    let terms = told(&mut player, answer(4, 1, Some(0))).await;
    let vars = [("low", "25"), ("high", "30")];
    let line = net::content::text().get(keys::SUBWAY_LIM_TERMS, &vars);
    assert!(
        terms.prompt_texts.iter().any(|t| t.contains(&line)),
        "{terms:?}"
    );
    told(&mut player, answer(0, 0, None)).await;
    assert!(terms.arrivals.is_empty(), "{terms:?}");

    // A party of one goes in, onto the first stage's clock.
    party_of_one(&mut player).await;
    told(&mut player, talk(SUBWAY_LOBBY, MR_LIM)).await;
    let mut seen = Trace::default();
    send(&mut player.connection, answer(4, 1, Some(0))).await;
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&run.entry) && !t.countdowns.is_empty() && !t.field.mobs.is_empty()
    })
    .await;
    assert!(seen.countdowns[0] <= 3, "{:?}", seen.countdowns);

    // A hit counts toward the run and toward the medal's two records.
    let mob = seen.field.mobs[0].0;
    send(&mut player.connection, attack(mob, 1)).await;
    until(&mut player.connection, &mut seen, |t| {
        told_value(t, "massacre_hit", 1) && t.kills.contains_key(&7762)
    })
    .await;
    assert_eq!(seen.kills.get(&7662).map(String::as_str), Some("1"));
    assert_eq!(seen.kills.get(&7762).map(String::as_str), Some("1"));

    // Each stage's time running out takes the party on; after the third
    // the result map gives D, with 2 EXP for the hit.
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&SUBWAY_RESULT) && !t.pyramid_scores.is_empty()
    })
    .await;
    assert_eq!(seen.pyramid_scores, [(4, 2)]);
    let stages: Vec<i32> = (1..=SUBWAY_STAGES).map(|n| run.stage_map(n)).collect();
    assert_eq!(seen.warps, [stages, vec![SUBWAY_RESULT]].concat());
    // Each stage opens with "Stage N" and "Start!".
    for n in 1..=SUBWAY_STAGES {
        let number = format!("killing/first/number/{n}");
        assert!(seen.field_effects.contains(&(3, number)), "{n}");
    }

    // Mr. Lim on the result map gives a pass for later and sends one back
    // to the station.
    let offer = told(&mut player, talk(SUBWAY_RESULT, MR_LIM)).await;
    assert!(said(&offer, keys::SUBWAY_LIM_RESULT_MENU), "{offer:?}");
    let back = told(&mut player, answer(4, 1, Some(1))).await;
    assert!(
        matches!(back.arrivals[..], [(SUBWAY_LOBBY, 0, _)]),
        "{back:?}"
    );
    assert!(back.added.iter().any(|a| a.2 == SUBWAY_PASS), "{back:?}");
    told(&mut player, answer(0, 1, None)).await;

    // There he takes it for Train 999.
    told(&mut player, talk(SUBWAY_LOBBY, MR_LIM)).await;
    let mut seen = Trace::default();
    send(&mut player.connection, answer(4, 1, Some(1))).await;
    until(&mut player.connection, &mut seen, |t| !t.removed.is_empty()).await;
    ride_train_999(&mut player, seen).await;

    // Without another, he sends one off.
    told(&mut player, talk(SUBWAY_LOBBY, MR_LIM)).await;
    let none = told(&mut player, answer(4, 1, Some(1))).await;
    assert!(said(&none, keys::SUBWAY_LIM_NO_PASS), "{none:?}");
}

#[tokio::test]
async fn the_result_map_boards_train_999_at_once() {
    let mut player = at_the_station().await;
    party_of_one(&mut player).await;
    told(&mut player, talk(SUBWAY_LOBBY, MR_LIM)).await;
    let mut seen = Trace::default();
    send(&mut player.connection, answer(4, 1, Some(0))).await;
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&SUBWAY_RESULT) && !t.pyramid_scores.is_empty()
    })
    .await;
    told(&mut player, talk(SUBWAY_RESULT, MR_LIM)).await;
    send(&mut player.connection, answer(4, 1, Some(0))).await;
    ride_train_999(&mut player, Trace::default()).await;
}

#[tokio::test]
async fn mr_lim_on_a_stage_lets_one_leave_for_the_station() {
    let mut player = at_the_station().await;
    let run = EventKind::Subway.event();
    party_of_one(&mut player).await;
    told(&mut player, talk(SUBWAY_LOBBY, MR_LIM)).await;
    let mut seen = Trace::default();
    send(&mut player.connection, answer(4, 1, Some(0))).await;
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&run.entry)
    })
    .await;
    let ask = told(&mut player, talk(run.entry, MR_LIM)).await;
    assert!(said(&ask, keys::SUBWAY_LIM_LEAVE_ASK), "{ask:?}");
    let left = told(&mut player, answer(1, 1, None)).await;
    assert!(
        matches!(left.arrivals[..], [(SUBWAY_LOBBY, 0, _)]),
        "{left:?}"
    );
}

#[tokio::test]
async fn mr_lim_gives_the_medal_for_10000_kills_ending_its_quest() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    set_job(id, 0, 27, 50).await.unwrap();
    set_quest(id, 7662, 1, "10000").await.unwrap();
    let mut player = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut player.connection;
    assert_eq!(warp(c, SUBWAY_RESULT, "", &mut seen).await, SUBWAY_RESULT);

    // On the result map without a run, Mr. Lim only sends one up.
    let up = told(&mut player, talk(SUBWAY_RESULT, MR_LIM)).await;
    assert!(matches!(up.arrivals[..], [(SUBWAY_LOBBY, 0, _)]), "{up:?}");
    assert!(up.added.is_empty() && up.prompts.is_empty(), "{up:?}");

    // Never taken from Dalair, the quest is started and ended with it.
    told(&mut player, talk(SUBWAY_LOBBY, MR_LIM)).await;
    let mut seen = told(&mut player, answer(4, 1, Some(2))).await;
    until(&mut player.connection, &mut seen, |t| {
        t.states.get(&29931) == Some(&2)
    })
    .await;
    assert!(said(&seen, keys::SUBWAY_LIM_MEDAL_GIVEN), "{seen:?}");
    assert!(seen.added.iter().any(|a| a.2 == 1_142_141), "{seen:?}");

    // One each.
    told(&mut player, answer(0, 1, None)).await;
    told(&mut player, talk(SUBWAY_LOBBY, MR_LIM)).await;
    let again = told(&mut player, answer(4, 1, Some(2))).await;
    assert!(said(&again, keys::SUBWAY_LIM_MEDAL_HAVE), "{again:?}");
}
