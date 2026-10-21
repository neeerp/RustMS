//! Quest fame rewards (`Act/1/pop`): Sabitrama's 2050 in the Sleepywood
//! dungeon gives one fame for a herb.

use integration_harness::database::{grant_item, set_job};
use integration_harness::login_to_world_session;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{act, COMPLETE, START};
use integration_harness::relogin;
use integration_harness::trace::{stat, warp, Trace};

const SABITRAMA: i32 = 1061005;
const HERB_MAP: i32 = 105040300;
const HERB: i32 = 4031020;
const ETC: i16 = 4;

#[tokio::test]
async fn sabitramas_errand_raises_fame() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let session = login_to_world_session(&config)
        .await
        .expect("first login failed");
    let (id, fame) = (session.character_id, session.fame);
    drop(session);
    drop(relogin(&config).await);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    // 2050 needs level 25.
    set_job(id, 0, 25, 50).await.expect("level up");
    grant_item(id, ETC, 1, HERB).await.expect("grant the herb");

    let mut trace = Trace::default();
    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    assert_eq!(warp(c, HERB_MAP, "", &mut trace).await, HERB_MAP);
    act(c, &mut trace, &m, START, 2050, SABITRAMA).await;
    act(c, &mut trace, &m, COMPLETE, 2050, SABITRAMA).await;
    assert_eq!(trace.states.get(&2050), Some(&2));
    assert_eq!(trace.fame_gains, [1]);
    assert_eq!(trace.stat_values(stat::FAME), [i64::from(fame) + 1]);
    drop(session);

    let restored = relogin(&config).await;
    assert_eq!(restored.fame, fame + 1);
}
