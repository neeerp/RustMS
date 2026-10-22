//! Blessing of the Fairy follows the account's highest other character in
//! the world: the world-entry character data names it, and the skill takes
//! a level for each 10 of its levels. Alone on the account, there is none.

use integration_harness::database::set_job;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::{login_to_world_session, relogin};

const BLESSING: i32 = 12;

#[tokio::test]
async fn the_fairy_blesses_by_the_accounts_highest_other_character() {
    let config = load_harness_config_or_fail().await;
    let mut sibling = config.clone();
    sibling.character_name = format!("cs{}", &config.character_name[2..]);

    let alone = login_to_world_session(&sibling)
        .await
        .expect("log in alone");
    assert_eq!(alone.linked_name, None);
    assert!(!alone.skills.contains_key(&BLESSING));
    let sibling_id = alone.character_id;
    drop(alone);

    let session = relogin(&config).await;
    assert_eq!(
        session.linked_name.as_deref(),
        Some(&*sibling.character_name)
    );
    assert_eq!(session.skills.get(&BLESSING), Some(&0));
    drop(session);

    set_job(sibling_id, 100, 57, 100)
        .await
        .expect("level the sibling");
    let session = relogin(&config).await;
    assert_eq!(
        session.linked_name.as_deref(),
        Some(&*sibling.character_name)
    );
    assert_eq!(session.skills.get(&BLESSING), Some(&5));
    drop(session);

    // A third character at level 1 links to the higher one too.
    let mut third = config.clone();
    third.character_name = format!("cr{}", &config.character_name[2..]);
    let session = relogin(&third).await;
    assert_eq!(
        session.linked_name.as_deref(),
        Some(&*sibling.character_name)
    );
    assert_eq!(session.skills.get(&BLESSING), Some(&5));
}
