//! Snipe on Snail Hunting Ground II (40001): its shot may hit for up to
//! 199,999, and its 4 second cooltime refuses another until it is over.

use integration_harness::database::{grant_skill, grant_stack, replace_item, set_job};
use integration_harness::hunting::ranged_attack;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const HUNTING_GROUND: i32 = 40_001;
const SNAIL: i32 = 100100;
const SNIPE: i32 = 3221007;
const CROSSBOW: i32 = 1462000;
const ARROWS: i32 = 2061000;
const EQUIPPED: i16 = -1;
const WEAPON: i16 = -11;
const USE: i16 = 2;
const QUIVER: i16 = 1;

#[tokio::test]
async fn snipe_hits_hard_and_cools_down() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(id, 321, 120, 1000).await.expect("marksman");
    grant_skill(id, SNIPE, 30).await.expect("Snipe");
    replace_item(id, EQUIPPED, WEAPON, CROSSBOW)
        .await
        .expect("give a crossbow");
    grant_stack(id, USE, QUIVER, ARROWS, 10)
        .await
        .expect("give arrows");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    let mut h = Trace::default();
    assert_eq!(warp(c, HUNTING_GROUND, "", &mut h).await, HUNTING_GROUND);
    until(c, &mut h, |t| t.field.alive(SNAIL).len() >= 2).await;
    let snails = h.field.alive(SNAIL);

    // More than 199,999 is refused.
    send(c, ranged_attack(SNIPE, QUIVER, snails[0], 200_000)).await;
    read_for(c, &mut h, 400).await;
    assert!(!h.field.killed.contains(&snails[0]));
    assert!(h.cooldowns.is_empty());

    // 199,999 kills, and the client hears of the cooltime.
    send(c, ranged_attack(SNIPE, QUIVER, snails[0], 199_999)).await;
    until(c, &mut h, |t| t.field.killed.contains(&snails[0])).await;
    assert_eq!(h.cooldowns, [(SNIPE, 4)]);

    // Another at once is refused; once the cooltime is over it hits.
    send(c, ranged_attack(SNIPE, QUIVER, snails[1], 199_999)).await;
    read_for(c, &mut h, 400).await;
    assert!(!h.field.killed.contains(&snails[1]));
    until(c, &mut h, |t| t.cooldowns.contains(&(SNIPE, 0))).await;
    send(c, ranged_attack(SNIPE, QUIVER, snails[1], 199_999)).await;
    until(c, &mut h, |t| t.field.killed.contains(&snails[1])).await;
}
