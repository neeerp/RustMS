use runtime::{ClientActor, ClientEvent, WorldServerActor};
use std::env;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("runtime=info".parse().unwrap()),
        )
        .init();

    info!("Starting World Server...");

    // Create channel for client events -> world server
    let (event_tx, event_rx) = mpsc::channel::<ClientEvent>(256);

    // Spawn world server actor
    let world_server = WorldServerActor::new(event_rx);
    tokio::spawn(async move {
        world_server.run().await;
    });

    // Accept connections
    let bind_addr =
        env::var("RUSTMS_WORLD_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8485".to_string());
    let listener = TcpListener::bind(&bind_addr).await.unwrap();
    info!("World Server listening on {}", bind_addr);

    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                info!(%peer_addr, "World connection accepted");

                let event_tx = event_tx.clone();
                tokio::spawn(async move {
                    match ClientActor::new(stream, event_tx).await {
                        Ok(actor) => actor.run().await,
                        Err(e) => error!(error = %e, "Failed to create ClientActor"),
                    }
                });
            }
            Err(e) => {
                error!(error = %e, "Error accepting world connection");
            }
        }
    }
}
