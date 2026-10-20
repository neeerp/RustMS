//! Infinity makes every skill free of MP while it lasts: a Bishop with too
//! little MP for Magic Guard casts it once Infinity is on, and keeps their
//! MP.

use integration_harness::database::{grant_skill, set_job};
use integration_harness::hunting::use_skill;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::step;
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const INFINITY: i32 = 2321004;
const MAGIC_GUARD: i32 = 2001002;
/// STAT_CHANGED's MP bit.
const MP: i32 = 0x1000;

#[tokio::test]
async fn infinity_makes_skills_free_of_mp() {
    let config = load_harness_config_or_fail().await;
    let id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    // Magic Guard 20 costs 12 MP.
    set_job(id, 232, 120, 10).await.expect("bishop");
    grant_skill(id, INFINITY, 1).await.expect("teach Infinity");
    grant_skill(id, MAGIC_GUARD, 20)
        .await
        .expect("teach Magic Guard");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;

    let t = step(c, "magic guard", use_skill(MAGIC_GUARD, 20)).await;
    assert!(t.buffs.is_empty(), "cast without the MP: {:?}", t.buffs);
    let t = step(c, "infinity", use_skill(INFINITY, 1)).await;
    assert_eq!(t.buffs, [(INFINITY, 1, 11_000)]);
    let t = step(c, "magic guard", use_skill(MAGIC_GUARD, 20)).await;
    assert_eq!(t.buffs, [(MAGIC_GUARD, 80, 600_000)]);
    assert_eq!(t.stat(MP).unwrap_or(10), 10);
}
