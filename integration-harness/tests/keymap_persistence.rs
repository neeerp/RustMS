use integration_harness::packets::build_change_keymap;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::{login_to_world_session, relogin};
use tokio::time::{sleep, Duration};

/// Default binding the server gives new characters (see game::keymap).
const DEFAULT_KEY: usize = 18;
/// A real skill id (Warrior's Power Strike): too large for a 16-bit column.
const SKILL_ID: i32 = 1001004;

#[tokio::test]
async fn changed_keys_survive_logout() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config)
        .await
        .expect("login-to-world session flow failed");
    assert_eq!(
        session.keymap[DEFAULT_KEY],
        (4, 1),
        "unexpected default binding"
    );
    // v83's layout: Whisper on H rather than Y, X sits, Space talks to NPCs
    // as an action, and 1 is all chat.
    assert_eq!(session.keymap[35], (4, 12));
    assert_eq!(session.keymap[21], (0, 0));
    assert_eq!(session.keymap[45], (5, 52));
    assert_eq!(session.keymap[57], (5, 55));
    assert_eq!(session.keymap[2], (4, 13));

    // Rebind an existing default key (the case the old upsert dropped), bind
    // a previously empty one, and bind a skill whose id needs 32 bits.
    session
        .connection
        .send_packet(
            build_change_keymap(&[(DEFAULT_KEY as i32, 4, 9), (8, 5, 54), (3, 1, SKILL_ID)])
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
    assert_eq!(session.keymap[8], (5, 54));
    assert_eq!(session.keymap[3], (1, SKILL_ID));
}
