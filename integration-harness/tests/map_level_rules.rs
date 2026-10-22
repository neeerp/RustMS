//! Map level rules: a portal into a map with `info/lvLimit` is closed to
//! players below it ("You cannot go to that place."), and a Training Center
//! (`info/lvForceMove` 20) sends players of level 20 and up back to town.

use integration_harness::database::set_job;
use integration_harness::packets::build_change_map;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{barrier, read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::SendOpcode;
use tokio::time::Duration;

const LEAFRE: i32 = 240_000_000;
/// Past Leafre's `west00`, with lvLimit 15.
const FOREST: i32 = 240_010_000;
const TRAINING_CENTER: i32 = 910_060_000;

#[tokio::test]
async fn level_limits_close_portals_and_training_centers_send_veterans_home() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut session = login_to_world_session(&config).await.unwrap();
    let id = session.character_id;
    let c = &mut session.connection;

    // At level 1, Leafre's west portal is closed.
    let mut t = Trace::default();
    assert_eq!(warp(c, LEAFRE, "", &mut t).await, LEAFRE);
    send(c, build_change_map(-1, "west00").unwrap()).await;
    until(c, &mut t, |t| !t.blocked.is_empty()).await;
    assert_eq!(t.blocked, [(SendOpcode::BlockedMap as i16, 2)]);
    assert_eq!(t.warps, [LEAFRE]);

    // A Training Center keeps a beginner.
    let mut t = Trace::default();
    assert_eq!(warp(c, TRAINING_CENTER, "", &mut t).await, TRAINING_CENTER);
    barrier(c, &m, &mut t).await;
    read_for(c, &mut t, 500).await;
    assert_eq!(t.warps, [TRAINING_CENTER]);
    // Leave before logging out, so the next login lands in Leafre.
    assert_eq!(warp(c, LEAFRE, "", &mut t).await, LEAFRE);

    drop(session);
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 0, 20, 50).await.expect("level 20");
    let mut session = relogin(&config).await;
    let c = &mut session.connection;

    // At level 20 the portal opens, and the Training Center sends the
    // player to its return map at once.
    let mut t = Trace::default();
    assert_eq!(warp(c, -1, "west00", &mut t).await, FOREST);
    let mut t = Trace::default();
    assert_eq!(warp(c, TRAINING_CENTER, "", &mut t).await, TRAINING_CENTER);
    until(c, &mut t, |t| t.warps.len() == 2).await;
    assert_ne!(t.warps[1], TRAINING_CENTER);
    assert_eq!(t.arrivals.last().map(|arrival| arrival.1), Some(0));
}
