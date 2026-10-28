//! Crimsonwood Keep's first two stages and their ways on. The first's way
//! on stays shut until a member finds the stage's hidden way, behind the
//! pillar by the Lion Shield; that takes the finder back to the stage's
//! start and clears it for all, with its EXP and mesos. The way on then
//! leads to the second stage, on its own clock, whose way on stays shut
//! until a skill of each job has lit that job's sigil. The harness lets
//! two set out (`RUSTMS_EXPEDITION_MIN`) and runs events 60 times as fast
//! (`RUSTMS_EVENT_SCALE`), so the first stage's clock lasts 2 seconds and
//! the second's 10.

use game_data::server_content::keys;
use integration_harness::database::{grant_item, grant_skill, set_job};
use integration_harness::events::{cleared, talk, told};
use integration_harness::guild_quest::touch_script_portal;
use integration_harness::hunting::{skill_at_reactor, walk_to, MapReactor};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{CWK, CWK_ENTRY, CWK_HIDDEN_WAY, CWK_LOBBY, CWK_SIGILS, CWK_SIGIL_LINES};
use net::npc_scripts::CWK_RECRUITER;
use tokio::time::Duration;

const KEYSTONE: i32 = 3_992_041;
const SETUP: i16 = 3;
const PINK: u8 = 5;
/// The stages' way on, `next00`.
const WAY_ON: &str = "next00";
const STAGE2: i32 = CWK_ENTRY + 100;
const STAGE3: i32 = CWK_ENTRY + 200;
/// A skill of each job that lights its sigil: a White Knight's, a
/// Ranger's, a Fire/Poison Wizard's, a Hermit's and a Marauder's.
const SIGIL_SKILLS: [i32; 5] = [1_211_002, 3_211_006, 2_111_002, 4_111_005, 5_111_005];
const WHITE_KNIGHT: i16 = 121;
const RANGER: i16 = 321;

/// A new character of level 90 of `job` with the Keystone and `skills`,
/// logged in at the Inner Sanctum's entrance.
async fn adventurer(job: i16, skills: &[i32]) -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, job, 90, 50).await.unwrap();
    grant_item(id, SETUP, 1, KEYSTONE).await.unwrap();
    for &skill in skills {
        grant_skill(id, skill, 1).await.unwrap();
    }
    let mut session = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut session.connection;
    assert_eq!(warp(c, CWK_LOBBY, "", &mut seen).await, CWK_LOBBY);
    session
}

/// `player` hears `key` in pink.
async fn hears(player: &mut WorldSession, t: &mut Trace, key: &str) {
    let line = (PINK, net::content::text().get(key, &[]));
    until(&mut player.connection, t, |t| t.broadcasts.contains(&line)).await;
}

/// The item line `key`.
fn line(key: &str) -> (String, i32) {
    (net::content::text().get(key, &[]), 0)
}

/// The sigil of job `job`, from 0, as `t` saw it.
fn sigil(t: &Trace, job: i32) -> Option<MapReactor> {
    let mut reactors = t.field.reactors.iter();
    reactors.find(|r| r.reactor_id == CWK_SIGILS + job).copied()
}

/// Whether `t` has seen all five sigils.
fn all_sigils(t: &Trace) -> bool {
    (0..5).all(|job| sigil(t, job).is_some())
}

// One test: a channel holds one expedition at a time.
#[tokio::test]
async fn the_keep_s_first_stages_clear_by_the_hidden_way_and_the_sigils() {
    let mut leader = adventurer(WHITE_KNIGHT, &SIGIL_SKILLS).await;
    let mut member = adventurer(RANGER, &SIGIL_SKILLS[1..2]).await;
    let recruiter = || talk(CWK_LOBBY, CWK_RECRUITER);

    // The leader gathers an expedition, the member joins, and they set
    // out.
    told(&mut leader, recruiter()).await;
    told(&mut leader, answer(1, 1, None)).await;
    told(&mut leader, answer(0, 1, None)).await;
    told(&mut member, recruiter()).await;
    told(&mut member, answer(1, 1, None)).await;
    told(&mut member, answer(0, 1, None)).await;
    told(&mut leader, recruiter()).await;
    let mut lt = told(&mut leader, answer(4, 1, Some(1))).await;
    until(&mut leader.connection, &mut lt, |t| {
        t.warps.contains(&CWK_ENTRY)
    })
    .await;
    let mut mt = Trace::default();
    until(&mut member.connection, &mut mt, |t| {
        t.warps.contains(&CWK_ENTRY)
    })
    .await;

    // The way on is shut.
    let mut lt = Trace::default();
    touch_script_portal(&mut leader, CWK_ENTRY, WAY_ON).await;
    hears(&mut leader, &mut lt, keys::CWK_WAY_BLOCKED).await;
    assert!(lt.warps.is_empty(), "{lt:?}");

    // The member finds the hidden way and is taken back to the start.
    let mut mt = Trace::default();
    send(
        &mut member.connection,
        walk_to(CWK_HIDDEN_WAY.0, CWK_HIDDEN_WAY.1),
    )
    .await;
    until(&mut member.connection, &mut mt, |t| {
        t.arrivals.iter().any(|a| (a.0, a.1) == (CWK_ENTRY, 0))
    })
    .await;

    // Both hear of it, clear the stage and get its EXP and mesos.
    let (exp, meso) = (CWK.stage_exp(1), CWK.stage_meso(1));
    for (player, t) in [(&mut leader, &mut lt), (&mut member, &mut mt)] {
        hears(player, t, keys::CWK_STAGE1_FOUND).await;
        until(&mut player.connection, t, |t| {
            cleared(t) && t.gains.contains(&exp) && t.meso_gains.contains(&meso)
        })
        .await;
    }

    // Now the way on leads to the second stage, whose clock starts over
    // (the first's may show first).
    let mut lt = Trace::default();
    touch_script_portal(&mut leader, CWK_ENTRY, WAY_ON).await;
    until(&mut leader.connection, &mut lt, |t| {
        t.warps.contains(&STAGE2) && t.countdowns.iter().any(|&clock| clock > 2) && all_sigils(t)
    })
    .await;
    assert!(lt.countdowns.iter().all(|&clock| clock <= 10), "{lt:?}");
    let sigils: Vec<_> = (0..5).filter_map(|job| sigil(&lt, job)).collect();
    let mut mt = Trace::default();
    touch_script_portal(&mut member, CWK_ENTRY, WAY_ON).await;
    until(&mut member.connection, &mut mt, |t| {
        t.warps.contains(&STAGE2) && all_sigils(t)
    })
    .await;

    // Whose own way on is shut until its sigils are lit.
    let mut lt = Trace::default();
    touch_script_portal(&mut leader, STAGE2, WAY_ON).await;
    hears(&mut leader, &mut lt, keys::CWK_WAY_NOT_ACTIVATED).await;
    assert!(lt.warps.is_empty(), "{lt:?}");

    // By the warriors' sigil, the bowman's own skill and a warrior's they
    // don't have do nothing; by the bowmen's, their skill lights it, and
    // everyone hears.
    let (warriors, bowmen) = (sigils[0], sigils[1]);
    let c = &mut member.connection;
    send(c, walk_to(warriors.x, warriors.y)).await;
    send(c, skill_at_reactor(warriors.object_id, SIGIL_SKILLS[1])).await;
    send(c, skill_at_reactor(warriors.object_id, SIGIL_SKILLS[0])).await;
    send(c, walk_to(bowmen.x, bowmen.y)).await;
    send(c, skill_at_reactor(bowmen.object_id, SIGIL_SKILLS[1])).await;
    until(c, &mut mt, |t| {
        t.field.reactor_hits.contains(&(bowmen.object_id, 1))
            && t.item_lines.contains(&line(CWK_SIGIL_LINES[1]))
    })
    .await;
    assert!(
        !(mt.field.reactor_hits.iter()).any(|&(oid, _)| oid == warriors.object_id),
        "{mt:?}"
    );

    // From too far off, the leader's skill does nothing; by each other
    // sigil, it lights it, the warriors' last.
    let c = &mut leader.connection;
    send(c, walk_to(bowmen.x, bowmen.y)).await;
    send(c, skill_at_reactor(warriors.object_id, SIGIL_SKILLS[0])).await;
    for job in [2, 3, 4, 0] {
        let sigil = sigils[job];
        if job == 0 {
            let mut hits = lt.field.reactor_hits.iter();
            assert!(!hits.any(|&(oid, _)| oid == sigil.object_id), "{lt:?}");
        }
        send(c, walk_to(sigil.x, sigil.y)).await;
        send(c, skill_at_reactor(sigil.object_id, SIGIL_SKILLS[job])).await;
        until(c, &mut lt, |t| {
            t.field.reactor_hits.contains(&(sigil.object_id, 1))
        })
        .await;
    }

    // With the fifth, everyone hears the way on is open, and the stage
    // clears with its EXP and mesos.
    let (exp, meso) = (CWK.stage_exp(2), CWK.stage_meso(2));
    for (player, t) in [(&mut leader, &mut lt), (&mut member, &mut mt)] {
        until(&mut player.connection, t, |t| {
            CWK_SIGIL_LINES
                .iter()
                .all(|&key| t.item_lines.contains(&line(key)))
                && t.item_lines.contains(&line(keys::CWK_SIGILS_DONE))
                && cleared(t)
                && t.gains.contains(&exp)
                && t.meso_gains.contains(&meso)
        })
        .await;
    }
    let done = line(keys::CWK_SIGILS_DONE);
    assert_eq!(lt.item_lines.iter().filter(|&l| *l == done).count(), 1);

    // And the way on leads to the third stage.
    let mut lt = Trace::default();
    touch_script_portal(&mut leader, STAGE2, WAY_ON).await;
    until(&mut leader.connection, &mut lt, |t| {
        t.warps.contains(&STAGE3)
    })
    .await;
}
