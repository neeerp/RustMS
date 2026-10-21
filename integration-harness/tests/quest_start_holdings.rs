//! Start requirements on what the player holds (`Check/0/item`): Heena
//! lends Sera's mirror (1000) only to someone with the GM shirt, and v83
//! counts a worn one.

use integration_harness::database::replace_item;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::quests::{act, START};
use integration_harness::trace::Trace;
use integration_harness::{login_to_world_session, relogin};

const HEENA: i32 = 2101;
const GM_SHIRT: i32 = 1042003;
const EQUIPPED: i16 = -1;
const TOP: i16 = -5;

#[tokio::test]
async fn heena_lends_the_mirror_only_to_a_gm_shirt() {
    let config = load_harness_config_or_fail().await;
    let m = config.character_name.clone();
    let mut trace = Trace::default();
    let mut session = login_to_world_session(&config).await.unwrap();
    let id = session.character_id;
    assert_eq!(session.map_id, 0);
    act(&mut session.connection, &mut trace, &m, START, 1000, HEENA).await;
    assert_eq!(trace.states.get(&1000), None);
    drop(session);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    // Swap the beginner's shirt for the GM one.
    replace_item(id, EQUIPPED, TOP, GM_SHIRT)
        .await
        .expect("wear the shirt");

    let mut session = relogin(&config).await;
    act(&mut session.connection, &mut trace, &m, START, 1000, HEENA).await;
    assert_eq!(trace.states.get(&1000), Some(&1));
}
