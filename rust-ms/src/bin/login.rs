use runtime::LoginServerActor;
use std::env;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("runtime=info".parse().unwrap()),
        )
        .init();

    info!("Starting Login Server...");

    let bind_addr =
        env::var("RUSTMS_LOGIN_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8484".to_string());
    if let Err(e) = LoginServerActor::run(&bind_addr).await {
        tracing::error!(error = %e, "Login server error");
    }
}
