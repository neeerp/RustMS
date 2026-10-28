//! Crimsonwood Keep's first stage and its ways on. Its way on stays shut
//! until a member finds the stage's hidden way, behind the pillar by the
//! Lion Shield; that takes the finder back to the stage's start and clears
//! it for all, with its EXP and mesos. The way on then leads to the second
//! stage, on its own clock, whose way on stays shut in turn. The harness
//! lets two set out (`RUSTMS_EXPEDITION_MIN`) and runs events 60 times as
//! fast (`RUSTMS_EVENT_SCALE`), so the first stage's clock lasts 2 seconds
//! and the second's 10.

use game_data::server_content::keys;
use integration_harness::database::{grant_item, set_job};
use integration_harness::events::{cleared, talk, told};
use integration_harness::guild_quest::touch_script_portal;
use integration_harness::hunting::walk_to;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin, WorldSession};
use net::events::{CWK, CWK_ENTRY, CWK_HIDDEN_WAY, CWK_LOBBY};
use net::npc_scripts::CWK_RECRUITER;
use tokio::time::Duration;

const KEYSTONE: i32 = 3_992_041;
const SETUP: i16 = 3;
const PINK: u8 = 5;
/// The stage's way on, `next00`.
const WAY_ON: &str = "next00";
const STAGE2: i32 = CWK_ENTRY + 100;

/// A new character of level 90 with the Keystone, logged in at the
/// Inner Sanctum's entrance.
async fn adventurer() -> WorldSession {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 90, 50).await.unwrap();
    grant_item(id, SETUP, 1, KEYSTONE).await.unwrap();
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

// One test: a channel holds one expedition at a time.
#[tokio::test]
async fn the_hidden_way_clears_the_keep_s_first_stage_and_opens_its_way_on() {
    let mut leader = adventurer().await;
    let mut member = adventurer().await;
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
        t.warps.contains(&STAGE2) && t.countdowns.iter().any(|&clock| clock > 2)
    })
    .await;
    assert!(lt.countdowns.iter().all(|&clock| clock <= 10), "{lt:?}");

    // Whose own way on is shut until its sigils are lit.
    let mut lt = Trace::default();
    touch_script_portal(&mut leader, STAGE2, WAY_ON).await;
    hears(&mut leader, &mut lt, keys::CWK_WAY_NOT_ACTIVATED).await;
    assert!(lt.warps.is_empty(), "{lt:?}");
}
