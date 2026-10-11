use integration_harness::packets::build_change_keymap;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::{login_to_world_session, WorldSession};
use tokio::time::{sleep, Duration};

/// Default binding the server gives new characters (see game::keymap).
const DEFAULT_KEY: usize = 18;

#[tokio::test]
async fn changed_keys_survive_logout() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config)
        .await
        .expect("login-to-world session flow failed");
    assert_eq!(session.keymap[DEFAULT_KEY], (4, 1), "unexpected default binding");

    // Rebind an existing default key (the case the old upsert dropped) and
    // bind a previously empty one.
    session
        .connection
        .send_packet(
            build_change_keymap(&[(DEFAULT_KEY as i32, 4, 9), (2, 5, 54)])
                .expect("failed to build keymap change"),
            "change keymap",
        )
        .await
        .expect("failed to send keymap change");
    // Keymap changes have no reply; give the server a moment to apply it
    // before the connection closes.
    sleep(Duration::from_millis(200)).await;
    drop(session);

    let session = relogin(&config).await;
    assert_eq!(session.keymap[DEFAULT_KEY], (4, 9));
    assert_eq!(session.keymap[2], (5, 54));
}

/// Log back in once the server has finished closing the previous session.
async fn relogin(config: &integration_harness::HarnessConfig) -> WorldSession {
    let mut last_error = None;
    for _ in 0..20 {
        match login_to_world_session(config).await {
            Ok(session) => return session,
            Err(error) => last_error = Some(error),
        }
        sleep(Duration::from_millis(250)).await;
    }
    panic!("relogin failed: {last_error:?}");
}
