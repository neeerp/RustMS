//! The way into Crimsonwood Keep's party quest: the Inner Sanctum's gate
//! (9201114) lets one with the Crimsonwood Keystone through to its
//! entrance, and turns anyone else back in pink. There anyone of level 90
//! gathers an expedition at the recruiter (9201113) and takes it into a
//! copy of the Keep's first stage, on a clock of two minutes, which sees
//! it out when it runs out. A minute in, the Keep's Master Guardians come
//! for anyone still on it. The harness lets two set out
//! (`RUSTMS_EXPEDITION_MIN`) and runs events 60 times as fast
//! (`RUSTMS_EVENT_SCALE`), so the clock shows at most 2 seconds and the
//! guardians come after 1.

use game_data::server_content::keys;
use integration_harness::database::{grant_item, set_job};
use integration_harness::events::{said, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{CWK_ENTRY, CWK_LOBBY, CWK_MASTER_GUARDIAN};
use net::npc_scripts::{CWK_GATE, CWK_RECRUITER};
use packet::Packet;
use tokio::time::Duration;

/// The Keep's hall before the gate, where the gate stands.
const GATE_HALL: i32 = 610_030_010;
const KEYSTONE: i32 = 3_992_041;
const SETUP: i16 = 3;
/// The portal past the gate, `out00`.
const PAST_GATE: u8 = 3;
const PINK: u8 = 5;

/// A new character of `level`, with the Keystone if `keyed`, logged in
/// before the gate.
async fn adventurer(level: i16, keyed: bool) -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, level, 50).await.unwrap();
    if keyed {
        grant_item(id, SETUP, 1, KEYSTONE).await.unwrap();
    }
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, GATE_HALL, "", &mut seen).await, GATE_HALL);
    session
}

fn recruiter() -> Packet {
    talk(CWK_LOBBY, CWK_RECRUITER)
}

/// `player` goes through the gate, without a word.
async fn through_the_gate(player: &mut WorldSession) {
    let mut t = told(player, talk(GATE_HALL, CWK_GATE)).await;
    until(&mut player.connection, &mut t, |t| {
        (t.arrivals.iter()).any(|a| (a.0, a.1) == (CWK_LOBBY, PAST_GATE))
    })
    .await;
    assert!(t.prompts.is_empty(), "{t:?}");
}

/// `player`, on `t` from before setting out, arrives in the Keep: the
/// clock it shows, which may come first.
async fn arrives(player: &mut WorldSession, t: &mut Trace) -> i32 {
    until(&mut player.connection, t, |t| {
        t.warps.contains(&CWK_ENTRY) && !t.countdowns.is_empty()
    })
    .await;
    t.countdowns[0]
}

// One test: a channel holds one expedition at a time.
#[tokio::test]
async fn the_keystone_s_holders_set_out_into_the_keep_until_the_time_is_up() {
    let mut leader = adventurer(90, true).await;
    let mut member = adventurer(90, true).await;
    let mut low = adventurer(89, false).await;
    let leader_name = leader.character_name.clone();
    let text = net::content::text();

    // Without the Keystone the gate won't move.
    let shut = text.get(keys::CWK_GATE_NO_KEY, &[]);
    let mut t = told(&mut low, talk(GATE_HALL, CWK_GATE)).await;
    until(&mut low.connection, &mut t, |t| {
        t.broadcasts.contains(&(PINK, shut.clone()))
    })
    .await;
    assert!(t.arrivals.is_empty() && t.prompts.is_empty(), "{t:?}");

    // With it, the gate lets them through.
    through_the_gate(&mut leader).await;
    through_the_gate(&mut member).await;

    // Under level 90, no one may lead or join.
    let mut t = Trace::default();
    assert_eq!(
        warp(&mut low.connection, CWK_LOBBY, "", &mut t).await,
        CWK_LOBBY
    );
    let t = told(&mut low, recruiter()).await;
    assert_eq!(t.prompts, [(CWK_RECRUITER, 0)]);
    let level = text.get(keys::CWK_SQUAD_LEVEL, &[("low", "90"), ("high", "255")]);
    assert_eq!(t.prompt_texts, [level]);
    told(&mut low, answer(0, 1, None)).await;

    // The leader gathers one, and the member joins.
    let t = told(&mut leader, recruiter()).await;
    assert_eq!(t.prompts, [(CWK_RECRUITER, 1)]);
    let lt = told(&mut leader, answer(1, 1, None)).await;
    assert!(said(&lt, keys::SQUAD_OPENED), "{lt:?}");
    told(&mut leader, answer(0, 1, None)).await;
    let t = told(&mut member, recruiter()).await;
    assert!(said(&t, keys::CWK_SQUAD_JOIN_ASK), "{t:?}");
    told(&mut member, answer(1, 1, None)).await;
    told(&mut member, answer(0, 1, None)).await;

    // The leader sets out, and both arrive on the first stage, hearing so,
    // on its two minutes' clock.
    let t = told(&mut leader, recruiter()).await;
    assert_eq!(t.prompts, [(CWK_RECRUITER, 4)]);
    let mut lt = told(&mut leader, answer(4, 1, Some(1))).await;
    let clock = arrives(&mut leader, &mut lt).await;
    assert!((1..=2).contains(&clock), "{lt:?}");
    let started = text.get(keys::CWK_SQUAD_NOTICE_STARTED, &[("name", &leader_name)]);
    let mut mt = Trace::default();
    let clock = arrives(&mut member, &mut mt).await;
    assert!((1..=2).contains(&clock), "{mt:?}");
    assert!(mt.item_lines.contains(&(started, 0)), "{mt:?}");

    // A minute in, both hear the guardians come, and see the twenty.
    let line = (PINK, text.get(keys::CWK_GUARDIANS_DETECTED, &[]));
    for (player, t) in [(&mut leader, &mut lt), (&mut member, &mut mt)] {
        until(&mut player.connection, t, |t| {
            t.broadcasts.contains(&line) && t.field.alive(CWK_MASTER_GUARDIAN).len() == 20
        })
        .await;
    }

    // The time up sees them back out to the entrance.
    for (player, t) in [(&mut leader, &mut lt), (&mut member, &mut mt)] {
        until(&mut player.connection, t, |t| t.warps.contains(&CWK_LOBBY)).await;
    }
}
