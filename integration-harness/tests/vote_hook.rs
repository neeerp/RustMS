//! The vote hook (`runtime::vote_hook`): the harness world listens on the
//! port after its channels', with secret `harness-vote-secret`, and a vote
//! gives 250 NX Credit once per 12 hours.

use integration_harness::packets::build_enter_cash_shop;
use integration_harness::preconditions::load_harness_config_or_fail;
use integration_harness::trace::{read, send, Trace};
use integration_harness::{login_to_world_session, relogin};
use net::packet::op::SendOpcode;
use packet::io::read::PktRead;
use std::io::Cursor;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const SECRET: &str = "harness-vote-secret";

/// Send `request` to the hook and return the status and body.
async fn call(hook: SocketAddr, request: String) -> (u16, String) {
    let mut stream = TcpStream::connect(hook).await.expect("connect to the hook");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    let status = response[9..12].parse().unwrap();
    let body = response.split_once("\r\n\r\n").unwrap().1.to_string();
    (status, body)
}

fn get(query: &str) -> String {
    format!("GET /vote?{query} HTTP/1.1\r\nHost: harness\r\n\r\n")
}

#[tokio::test]
async fn a_vote_gives_nx_once_per_cooldown() {
    let config = load_harness_config_or_fail().await;
    login_to_world_session(&config).await.expect("login failed");
    let hook = SocketAddr::new(config.world_addr.ip(), config.world_addr.port() + 3);
    let account = &config.username;

    assert_eq!(
        call(hook, get(&format!("account={account}&secret=wrong"))).await,
        (403, "wrong secret".to_string())
    );
    assert_eq!(
        call(
            hook,
            get(&format!("account=no_such_{account}&secret={SECRET}"))
        )
        .await,
        (404, "no such account".to_string())
    );
    // A site's form post, with the secret in a header.
    let form = format!("account={account}");
    let post = format!(
        "POST /vote HTTP/1.1\r\nHost: harness\r\nContent-Type: application/x-www-form-urlencoded\r\nX-Vote-Secret: {SECRET}\r\nContent-Length: {}\r\n\r\n{form}",
        form.len()
    );
    assert_eq!(call(hook, post).await, (200, "counted".to_string()));
    let (status, body) = call(hook, get(&format!("account={account}&secret={SECRET}"))).await;
    assert_eq!(status, 429, "{body}");

    let mut session = relogin(&config).await;
    let c = &mut session.connection;
    send(c, build_enter_cash_shop()).await;
    let mut t = Trace::default();
    let credit = loop {
        let p = read(c, &mut t).await;
        if p.opcode() == SendOpcode::QueryCashResult as i16 {
            break Cursor::new(&p.bytes[2..]).read_int().unwrap();
        }
    };
    assert_eq!(credit, 250);
}
