use runtime::LoginServerActor;
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

    if let Err(e) = LoginServerActor::run("0.0.0.0:8484").await {
        tracing::error!(error = %e, "Login server error");
    }
}
