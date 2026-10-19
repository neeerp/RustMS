//! Mobs that come out of another when it dies (`revive`): the bottle of
//! Blue Perfume (4300000, 1 HP) in Kerning Square (103040200) lets out a
//! Blue Perfume that fights (4300001) where it broke, once its dying
//! (2.38 s) is over.

use integration_harness::hunting::attack;
use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};

const MAP: i32 = 103_040_200;
const BOTTLE: i32 = 4_300_000;
const PERFUME: i32 = 4_300_001;

#[tokio::test]
async fn a_broken_perfume_bottle_lets_the_perfume_out() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config).await.unwrap();
    let c = &mut session.connection;
    let mut t = Trace::default();
    assert_eq!(warp(c, MAP, "", &mut t).await, MAP);
    until(c, &mut t, |t| !t.field.alive(BOTTLE).is_empty()).await;
    read_for(c, &mut t, 300).await;
    let bottle = t.field.alive(BOTTLE)[0];
    let out = t.field.alive(PERFUME).len();

    send(c, attack(bottle, 1)).await;
    until(c, &mut t, |t| t.field.killed.contains(&bottle)).await;
    read_for(c, &mut t, 1_500).await;
    assert_eq!(
        t.field.alive(PERFUME).len(),
        out,
        "out before it is done dying"
    );
    until(c, &mut t, |t| t.field.alive(PERFUME).len() > out).await;
    let perfume = *t.field.alive(PERFUME).last().unwrap();
    assert_eq!(
        t.field.mob_positions[&perfume],
        t.field.mob_positions[&bottle]
    );
}
