//! Echo of Hero reaches everyone around the player, in their party or not:
//! a Beginner's cast gives both them and a stranger beside them its 4%.

use integration_harness::database::{grant_skill, set_job};
use integration_harness::hunting::use_skill;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read_for, send, until, warp, Trace};
use integration_harness::{login_to_world_session, relogin};
use tokio::time::Duration;

const FIELD: i32 = 101_020_001;
const ECHO_OF_HERO: i32 = 1005;
/// Its `x` percent for its `time` of 2400 seconds.
const ECHO: (i32, i16, i32) = (ECHO_OF_HERO, 4, 2_400_000);

#[tokio::test]
async fn echo_of_hero_reaches_strangers_too() {
    let config = load_harness_config_or_fail().await;
    let hero_id = login_to_world_session(&config)
        .await
        .expect("first login failed")
        .character_id;
    drop(relogin(&config).await);
    let other_config = load_harness_config_or_fail().await;
    login_to_world_session(&other_config)
        .await
        .expect("second login failed");
    drop(relogin(&other_config).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    set_job(hero_id, 0, 30, 100).await.expect("beginner");
    grant_skill(hero_id, ECHO_OF_HERO, 1)
        .await
        .expect("teach Echo of Hero");
    for config in [&config, &other_config] {
        let mut session = relogin(config).await;
        let mut trace = Trace::default();
        assert_eq!(
            warp(&mut session.connection, FIELD, "", &mut trace).await,
            FIELD
        );
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut hero = relogin(&config).await;
    let mut stranger = relogin(&other_config).await;
    let (h, s) = (&mut hero.connection, &mut stranger.connection);
    let (mut ht, mut st) = (Trace::default(), Trace::default());
    read_for(h, &mut ht, 300).await;
    read_for(s, &mut st, 300).await;

    send(h, use_skill(ECHO_OF_HERO, 1)).await;
    until(h, &mut ht, |t| !t.buffs.is_empty()).await;
    assert_eq!(ht.buffs, [ECHO]);
    until(s, &mut st, |t| !t.buffs.is_empty()).await;
    assert_eq!(st.buffs, [ECHO]);
}
