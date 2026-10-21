use integration_harness::packets::build_quick_slot;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::protocol::WorldSession;
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::SendOpcode;
use tokio::time::{sleep, timeout_at, Duration, Instant};

/// v83's layout when none is set.
const DEFAULT: [i32; 8] = [42, 82, 71, 73, 29, 83, 79, 81];
const NUMBERS: [i32; 8] = [2, 3, 4, 5, 16, 17, 18, 19];

async fn send(session: &mut WorldSession, keys: &[i32]) {
    session
        .connection
        .send_packet(
            build_quick_slot(keys).expect("failed to build quick slots"),
            "quick slots",
        )
        .await
        .expect("failed to send quick slots");
}

/// The server never answers QUICK_SLOT, or the client's echo would loop.
async fn expect_no_answer(session: &mut WorldSession) {
    let deadline = Instant::now() + Duration::from_millis(500);
    while let Ok(Ok(envelope)) =
        timeout_at(deadline, session.connection.read_packet("no answer")).await
    {
        assert_ne!(
            envelope.opcode(),
            SendOpcode::QuickSlotInit as i16,
            "QUICK_SLOT was answered"
        );
    }
}

async fn send_and_leave(mut session: WorldSession, keys: &[i32]) {
    send(&mut session, keys).await;
    // There is no reply; give the server a moment before the connection closes.
    sleep(Duration::from_millis(200)).await;
}

#[tokio::test]
async fn quick_slots_survive_logout() {
    let config = load_harness_config_or_fail().await;
    let mut session = login_to_world_session(&config)
        .await
        .expect("login-to-world session flow failed");
    assert_eq!(
        session.quick_slots, None,
        "a new character has the defaults"
    );

    // The client echoes the defaults; that sets nothing.
    send(&mut session, &DEFAULT).await;
    expect_no_answer(&mut session).await;
    drop(session);
    let session = relogin(&config).await;
    assert_eq!(session.quick_slots, None);

    send_and_leave(session, &NUMBERS).await;
    let mut session = relogin(&config).await;
    assert_eq!(session.quick_slots, Some(NUMBERS));

    // Its echo, as after that QUICKSLOT_INIT, is not answered either.
    send(&mut session, &NUMBERS).await;
    expect_no_answer(&mut session).await;

    // Layouts the editor cannot make are ignored: a key used twice, Esc,
    // and a key past the key map, then a short packet.
    send(&mut session, &[2, 3, 4, 5, 16, 17, 18, 2]).await;
    send(&mut session, &[1, 3, 4, 5, 16, 17, 18, 19]).await;
    send(&mut session, &[89, 3, 4, 5, 16, 17, 18, 19]).await;
    send_and_leave(session, &[20, 21, 22, 23, 24, 25, 26]).await;
    let session = relogin(&config).await;
    assert_eq!(session.quick_slots, Some(NUMBERS));

    // Back to the defaults, which are now kept as a layout of their own.
    send_and_leave(session, &DEFAULT).await;
    let session = relogin(&config).await;
    assert_eq!(session.quick_slots, Some(DEFAULT));
}
