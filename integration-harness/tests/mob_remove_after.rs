//! Mobs that leave on their own (`removeAfter`): each Kitty from Cheese
//! Storage (9500324) in Magatia's cheese storage (926120400) is gone 5 s
//! after it came, giving nothing, and its spawn point fills again.

use std::time::{Duration, Instant};

use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, until, warp, Trace};

const MAP: i32 = 926_120_400;
const KITTY: i32 = 9_500_324;

#[tokio::test]
async fn a_kitty_leaves_five_seconds_after_it_came() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, MAP, "", &mut t).await, MAP);
    // One that fades in while we are here, so we know when it came.
    let came = |t: &Trace| {
        t.field
            .alive(KITTY)
            .into_iter()
            .find(|id| t.field.appearances.get(id) == Some(&-2))
    };
    until(c, &mut t, |t| came(t).is_some()).await;
    let kitty = came(&t).unwrap();
    let seen = Instant::now();

    read_for(c, &mut t, 4_500).await;
    assert!(!t.field.killed.contains(&kitty), "gone too soon");
    until(c, &mut t, |t| t.field.killed.contains(&kitty)).await;
    assert!(seen.elapsed() < Duration::from_millis(5_600), "gone late");
    assert!(t.gains.is_empty(), "EXP for a mob that left: {:?}", t.gains);
    assert!(t.field.drops.is_empty(), "drops from a mob that left");
}
