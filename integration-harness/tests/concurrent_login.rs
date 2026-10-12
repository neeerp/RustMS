use integration_harness::check_char_name_available;
use integration_harness::connection::MapleTestConnection;
use integration_harness::packets::{
    build_login_credentials, build_login_started, decode_login_status,
};
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::HarnessConfig;
use tokio::task::JoinSet;
use tokio::time::{sleep, timeout, Duration};

const SUCCESS: i32 = 0;
const ALREADY_LOGGED_IN: i32 = 7;

/// Send credentials and return the login status, keeping the connection
/// (and so any session it opened) alive for the caller.
async fn attempt_login(config: &HarnessConfig) -> (i32, MapleTestConnection) {
    let mut conn = MapleTestConnection::connect(config.login_addr, "login handshake")
        .await
        .expect("connect to login server");
    conn.send_packet(build_login_started(), "login start")
        .await
        .expect("send login start");
    conn.send_packet(
        build_login_credentials(&config.username, &config.password).expect("build credentials"),
        "login credentials",
    )
    .await
    .expect("send credentials");
    let reply = timeout(Duration::from_secs(5), conn.read_packet("login status"))
        .await
        .expect("timed out waiting for login status")
        .expect("read login status");
    let status = decode_login_status(&reply.packet)
        .expect("decode login status")
        .status;
    (status, conn)
}

/// Logins fired at once per round. The race being guarded against is a few
/// milliseconds wide (between "is the account online?" and opening the
/// session), so many simultaneous attempts are needed to land inside it.
const ATTEMPTS: usize = 16;
/// Rounds that must each admit exactly one login.
const ROUNDS: usize = 3;

#[tokio::test(flavor = "multi_thread")]
async fn simultaneous_logins_to_one_account_admit_exactly_one() {
    let config = load_harness_config_or_fail().await;
    // Create the account (with terms accepted and a character), then let the
    // connection close so the account is offline.
    check_char_name_available(&config, "SetupProbe")
        .await
        .expect("account setup");

    let mut clean_rounds = 0;
    for _ in 0..40 {
        let mut attempts = JoinSet::new();
        for _ in 0..ATTEMPTS {
            let config = config.clone();
            attempts.spawn(async move { attempt_login(&config).await });
        }
        // Keep every connection open until all statuses are in, so sessions
        // opened by winners are still held while the others are checked.
        let results = attempts.join_all().await;

        let admitted = results
            .iter()
            .filter(|(status, _)| *status == SUCCESS)
            .count();
        assert!(
            admitted <= 1,
            "{admitted} concurrent logins were admitted to the same account"
        );
        assert!(results
            .iter()
            .all(|(status, _)| *status == SUCCESS || *status == ALREADY_LOGGED_IN));
        if admitted == 1 {
            clean_rounds += 1;
            if clean_rounds == ROUNDS {
                return;
            }
        }
        // Close every connection (releasing the winner's session) and wait for
        // the server to clean up before the next round. A round where every
        // attempt saw the account online is simply retried.
        drop(results);
        sleep(Duration::from_millis(250)).await;
    }
    panic!("only {clean_rounds} of {ROUNDS} rounds found the account available for login");
}
