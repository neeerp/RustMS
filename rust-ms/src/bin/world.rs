use std::env;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("runtime=info".parse().unwrap())
                .add_directive("gm_audit=info".parse().unwrap()),
        )
        .init();

    info!("Starting World Server...");

    if let Err(e) = runtime::prepare_database().await {
        error!(error = %e, "Failed to prepare database");
        std::process::exit(1);
    }

    // Channels listen on the ports from the channel configuration
    // (RUSTMS_LOGIN_CHANNELS); this only picks the interface.
    let bind_host = env::var("RUSTMS_WORLD_BIND_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    if let Err(e) = runtime::run_world_server(&bind_host).await {
        error!(error = %e, "World server error");
        std::process::exit(1);
    }
}
