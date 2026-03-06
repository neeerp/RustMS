use crate::config::{harness_addrs_from_env, HarnessConfig, MultiHarnessConfig};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

const SERVER_CHECK_TIMEOUT: Duration = Duration::from_millis(250);

pub async fn load_harness_config_or_fail() -> HarnessConfig {
    let (login_addr, world_addr) = load_or_panic_addrs();
    let config = HarnessConfig::random(login_addr, world_addr);
    assert_servers_reachable(config.login_addr, config.world_addr).await;
    config
}

pub async fn load_multi_harness_config_or_fail() -> MultiHarnessConfig {
    let (login_addr, world_addr) = load_or_panic_addrs();
    let config = MultiHarnessConfig::random_pair(login_addr, world_addr);
    assert_servers_reachable(config.login_addr, config.world_addr).await;
    config
}

async fn assert_servers_reachable(login_addr: SocketAddr, world_addr: SocketAddr) {
    if !endpoint_reachable(login_addr).await {
        panic!("integration precondition failed: login server at {login_addr} is not reachable");
    }

    if !endpoint_reachable(world_addr).await {
        panic!("integration precondition failed: world server at {world_addr} is not reachable");
    }
}

async fn endpoint_reachable(endpoint: SocketAddr) -> bool {
    matches!(
        timeout(SERVER_CHECK_TIMEOUT, TcpStream::connect(endpoint)).await,
        Ok(Ok(_))
    )
}

fn load_or_panic_addrs() -> (SocketAddr, SocketAddr) {
    harness_addrs_from_env()
        .unwrap_or_else(|error| panic!("failed to parse integration harness addresses: {error}"))
}
