//! An expedition against Horntail: anyone of level 100 gathers a squad at
//! the Mark of the Squad (2083004) on the Entrance to Horntail's Cave and
//! takes it into a copy of the cave of its own, from the first trial on.
//! While it is in, the Mark turns others away, and a member who stepped
//! out may go back. Like Zakum's, it ends only when the last of it leaves,
//! and the channel may then gather another. The harness lets two set out
//! (`RUSTMS_EXPEDITION_MIN`).

use game_data::server_content::keys;
use integration_harness::database::set_job;
use integration_harness::events::{said, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use packet::Packet;
use tokio::time::Duration;

const ENTRANCE: i32 = 240050400;
const FIRST_TRIAL: i32 = 240060000;
const ROAD_IN_BETWEEN: i32 = 240050600;
const RECRUITER: i32 = 2083004;
const CRYSTAL: i32 = 2083002;

/// A new character of `level`, logged in on the Entrance to Horntail's
/// Cave.
async fn adventurer(level: i16) -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, level, 50).await.unwrap();
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, ENTRANCE, "", &mut seen).await, ENTRANCE);
    session
}

fn recruiter() -> Packet {
    talk(ENTRANCE, RECRUITER)
}

/// `player`, in the cave, leaves by the Crystal of Roots for the Road in
/// Between.
async fn step_out(player: &mut WorldSession, map: i32) {
    told(player, talk(map, CRYSTAL)).await;
    let mut t = told(player, answer(1, 1, None)).await;
    until(&mut player.connection, &mut t, |t| {
        t.arrivals.iter().any(|a| a.0 == ROAD_IN_BETWEEN)
    })
    .await;
}

/// Wait until `player` arrives in the cave, beside `beside` if any.
async fn arrives(player: &mut WorldSession, t: &mut Trace, beside: Option<&str>) {
    until(&mut player.connection, t, |t| {
        t.arrivals.iter().any(|a| a.0 == FIRST_TRIAL)
            && beside.is_none_or(|name| t.players.iter().any(|p| p.character_name == name))
    })
    .await;
}

#[tokio::test]
async fn an_expedition_goes_into_horntails_cave_and_ends_with_its_last() {
    let mut leader = adventurer(100).await;
    let mut member = adventurer(100).await;
    let mut other = adventurer(100).await;
    let mut low = adventurer(99).await;
    let leader_name = leader.character_name.clone();
    let text = |key, name: &str| net::content::text().get(key, &[("name", name)]);

    // Under level 100, no one may lead or join.
    let t = told(&mut low, recruiter()).await;
    assert_eq!(t.prompts, [(RECRUITER, 0)]);
    let level = net::content::text().get(
        keys::HORNTAIL_SQUAD_LEVEL,
        &[("low", "100"), ("high", "255")],
    );
    assert_eq!(t.prompt_texts, [level]);
    told(&mut low, answer(0, 1, None)).await;

    // Anyone of the levels may lead one, and those around hear of it.
    let t = told(&mut leader, recruiter()).await;
    assert_eq!(t.prompts, [(RECRUITER, 1)]);
    let lt = told(&mut leader, answer(1, 1, None)).await;
    assert!(said(&lt, keys::SQUAD_OPENED), "{lt:?}");
    told(&mut leader, answer(0, 1, None)).await;
    let opened = text(keys::HORNTAIL_SQUAD_NOTICE_OPENED, &leader_name);
    let mut mt = Trace::default();
    until(&mut member.connection, &mut mt, |t| {
        t.item_lines.contains(&(opened.clone(), 0))
    })
    .await;

    // The member joins.
    let t = told(&mut member, recruiter()).await;
    assert!(said(&t, keys::HORNTAIL_SQUAD_JOIN_ASK), "{t:?}");
    told(&mut member, answer(1, 1, None)).await;
    told(&mut member, answer(0, 1, None)).await;

    // The leader sets out, and both arrive on the first trial, hearing so.
    let t = told(&mut leader, recruiter()).await;
    assert_eq!(t.prompts, [(RECRUITER, 4)]);
    let mut lt = told(&mut leader, answer(4, 1, Some(1))).await;
    arrives(&mut leader, &mut lt, None).await;
    let started = text(keys::HORNTAIL_SQUAD_NOTICE_STARTED, &leader_name);
    let mut mt = Trace::default();
    until(&mut member.connection, &mut mt, |t| {
        t.item_lines.contains(&(started.clone(), 0))
            && t.arrivals.iter().any(|a| a.0 == FIRST_TRIAL)
    })
    .await;

    // Others are turned away while it is in.
    let t = told(&mut other, recruiter()).await;
    assert_eq!(t.prompts, [(RECRUITER, 0)]);
    assert!(said(&t, keys::HORNTAIL_SQUAD_TAKEN), "{t:?}");
    told(&mut other, answer(0, 1, None)).await;

    // The leader steps out, which ends nothing (Cosmic's expeditions need
    // neither their leader nor their minimum inside), and goes back in,
    // beside the member.
    step_out(&mut leader, FIRST_TRIAL).await;
    let mut t = Trace::default();
    warp(&mut leader.connection, ENTRANCE, "", &mut t).await;
    let t = told(&mut leader, recruiter()).await;
    assert_eq!(t.prompts, [(RECRUITER, 1)]);
    assert!(said(&t, keys::HORNTAIL_SQUAD_REJOIN_ASK), "{t:?}");
    let member_name = member.character_name.clone();
    let mut lt = told(&mut leader, answer(1, 1, None)).await;
    arrives(&mut leader, &mut lt, Some(&member_name)).await;

    // Both leave: the last one's leaving ends it, and the channel may
    // gather another.
    step_out(&mut member, FIRST_TRIAL).await;
    step_out(&mut leader, FIRST_TRIAL).await;
    let mut free = false;
    for _ in 0..20 {
        let t = told(&mut other, recruiter()).await;
        told(&mut other, answer(0, 1, None)).await;
        if t.prompts == [(RECRUITER, 1)] {
            free = true;
            break;
        }
        assert!(said(&t, keys::HORNTAIL_SQUAD_TAKEN), "{t:?}");
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    assert!(free, "the expedition is still under way");
}
