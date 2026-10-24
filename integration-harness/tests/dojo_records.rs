//! So Gong records a challenger's score on a Mu Lung Dojo rest floor, and
//! in the hall takes them back to the floor after it, even after logging
//! out. The harness world runs events 60 times as fast, so minutes read as
//! seconds.

use game_data::server_content::keys;
use integration_harness::database::{set_dojo_trial_done, set_job};
use integration_harness::events::{said, talk, told};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::answer;
use integration_harness::trace::{send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::events::{DOJO, DOJO_EXIT, DOJO_FIRST};
use net::npc_scripts::SO_GONG;
use tokio::time::Duration;

#[tokio::test]
async fn a_recorded_score_starts_the_next_run_after_its_rest_floor() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config).await.unwrap().character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 30, 50).await.unwrap();
    set_dojo_trial_done(id).await.unwrap();
    let mut player = relogin(&config).await;
    let mut seen = Trace::default();
    let c = &mut player.connection;
    assert_eq!(warp(c, DOJO.lobby, "", &mut seen).await, DOJO.lobby);
    told(&mut player, talk(DOJO.lobby, SO_GONG)).await;
    let mut seen = Trace::default();
    send(&mut player.connection, answer(4, 1, Some(0))).await;
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&DOJO_FIRST)
    })
    .await;

    // On the sixth floor he records the score, then they leave and log
    // out.
    let rest = DOJO.stage_map(6);
    let mut seen = Trace::default();
    assert_eq!(
        warp(&mut player.connection, rest, "", &mut seen).await,
        rest
    );
    told(&mut player, talk(rest, SO_GONG)).await;
    let asked = told(&mut player, answer(4, 1, Some(2))).await;
    assert!(said(&asked, keys::DOJO_SO_GONG_RECORD_ASK), "{asked:?}");
    let recorded = told(&mut player, answer(1, 1, None)).await;
    assert!(said(&recorded, keys::DOJO_SO_GONG_RECORDED), "{recorded:?}");
    send(&mut player.connection, answer(0, -1, None)).await;
    let c = &mut player.connection;
    let mut seen = Trace::default();
    assert_eq!(warp(c, DOJO_EXIT, "", &mut seen).await, DOJO_EXIT);
    drop(player);
    let mut player = relogin(&config).await;
    let c = &mut player.connection;
    let mut seen = Trace::default();
    assert_eq!(warp(c, DOJO.lobby, "", &mut seen).await, DOJO.lobby);

    // In the hall he offers round 5, and goes on from the seventh floor
    // with the second section's clock.
    told(&mut player, talk(DOJO.lobby, SO_GONG)).await;
    let offer = told(&mut player, answer(4, 1, Some(0))).await;
    let round = net::content::text().get(keys::DOJO_SO_GONG_RESUME, &[("round", "5")]);
    assert!(
        offer.prompt_texts.iter().any(|t| t.contains(&round)),
        "{offer:?}"
    );
    let mut seen = Trace::default();
    send(&mut player.connection, answer(1, 1, None)).await;
    let next = DOJO.stage_map(7);
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&next) && t.countdowns.iter().any(|&left| left >= 5)
    })
    .await;

    // The record went with it: next time he starts from the first floor.
    let c = &mut player.connection;
    let mut seen = Trace::default();
    assert_eq!(warp(c, DOJO_EXIT, "", &mut seen).await, DOJO_EXIT);
    let mut seen = Trace::default();
    assert_eq!(warp(c, DOJO.lobby, "", &mut seen).await, DOJO.lobby);
    told(&mut player, talk(DOJO.lobby, SO_GONG)).await;
    let mut seen = Trace::default();
    send(&mut player.connection, answer(4, 1, Some(0))).await;
    until(&mut player.connection, &mut seen, |t| {
        t.warps.contains(&DOJO_FIRST)
    })
    .await;
}
