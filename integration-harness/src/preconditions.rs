use crate::config::{HarnessConfig, MultiHarnessConfig};
use crate::error::HarnessError;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

const SERVER_CHECK_TIMEOUT: Duration = Duration::from_millis(250);

pub async fn load_harness_config_or_fail() -> HarnessConfig {
    let config = load_or_panic_config(HarnessConfig::from_file(), "single-player");
    assert_servers_reachable(config.login_addr, config.world_addr).await;
    config
}

pub async fn load_multi_harness_config_or_fail() -> MultiHarnessConfig {
    let config = load_or_panic_config(MultiHarnessConfig::from_file(), "multi-player");
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

fn load_or_panic_config<T>(result: Result<T, HarnessError>, config_kind: &'static str) -> T {
    match result {
        Ok(config) => config,
        Err(HarnessError::MissingConfigFile { path }) => {
            panic!(
                "integration precondition failed: missing {config_kind} harness config file at `{path}`"
            )
        }
        Err(other) => panic!("failed to load {config_kind} integration harness config: {other}"),
    }
}
