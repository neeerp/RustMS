use integration_harness::packets::{build_change_keymap, build_skill_macro_change};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::protocol::WorldSession;
use integration_harness::{login_to_world_session, relogin};
use tokio::time::{sleep, Duration};

/// A key with no default binding.
const KEY: i32 = 21;
/// Key kind 8: the skill macro at the binding's place in the list.
const MACRO_KEY: u8 = 8;

async fn send_and_leave(mut session: WorldSession, packet: packet::Packet) {
    session
        .connection
        .send_packet(packet, "skill macro change")
        .await
        .expect("failed to send the change");
    // There is no reply; give the server a moment before the connection closes.
    sleep(Duration::from_millis(200)).await;
}

#[tokio::test]
async fn macros_survive_logout() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config)
        .await
        .expect("login-to-world session flow failed");
    assert!(session.skill_macros.is_empty(), "a new character has none");

    session
        .connection
        .send_packet(
            build_change_keymap(&[(KEY, MACRO_KEY, 1)]).expect("failed to build keymap change"),
            "change keymap",
        )
        .await
        .expect("failed to send keymap change");
    // Teleport and a passive skill may not be held, so they come back empty.
    let macros = [
        ("buffs".to_string(), true, [1001003, 2101002, 0]),
        ("hit".to_string(), false, [1000000, 0, 1001004]),
    ];
    send_and_leave(session, build_skill_macro_change(&macros, 2).unwrap()).await;

    let session = relogin(&config).await;
    assert_eq!(
        session.skill_macros,
        [
            ("buffs".to_string(), true, [1001003, 0, 0]),
            ("hit".to_string(), false, [0, 0, 1001004]),
        ]
    );
    assert_eq!(session.keymap[KEY as usize], (MACRO_KEY, 1));

    // A list of six is ignored whole.
    let six = vec![("a".to_string(), false, [0; 3]); 6];
    send_and_leave(session, build_skill_macro_change(&six, 6).unwrap()).await;
    let session = relogin(&config).await;
    assert_eq!(session.skill_macros.len(), 2);

    send_and_leave(session, build_skill_macro_change(&[], 0).unwrap()).await;
    let session = relogin(&config).await;
    assert!(session.skill_macros.is_empty());
}
