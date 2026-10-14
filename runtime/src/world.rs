//! World server bootstrap: one listener per configured channel.

use crate::actor::client::{ClientActor, WorldServices};
use crate::actor::directory::WorldDirectory;
use crate::actor::registry::FieldRegistry;
use crate::db::spawn_db;
use crate::error::RuntimeError;
use crate::supervisor::spawn_supervised;
use net::login_world::{load_login_worlds, LoginChannel};
use std::collections::HashSet;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tracing::{debug, error, info};

const DIRECTORY_QUEUE_CAPACITY: usize = 1024;
/// How often fields advance time-driven state.
const FIELD_TICK: Duration = Duration::from_millis(100);

/// Run every channel of the configured world in this process. Channel `n`
/// listens on its own advertised port, so a connection's channel is known
/// from the socket it arrived on (v83 clients do not send it).
pub async fn run_world_server(bind_host: &str) -> Result<(), RuntimeError> {
    let channels = configured_channels()?;
    preload_game_data().await?;

    let channel_ids: Vec<i16> = channels.iter().map(|c| i16::from(c.channel_id)).collect();
    let cleared = spawn_db(move || db::session::clear_world_sessions(&channel_ids)).await?;
    info!(cleared, "Cleared stale in-game sessions");

    let (directory_tx, directory_rx) = mpsc::channel(DIRECTORY_QUEUE_CAPACITY);
    spawn_supervised(
        "world directory".to_string(),
        WorldDirectory::new(directory_rx).run(),
        || error!("World directory stopped; whispers are unavailable"),
    );
    let services = WorldServices {
        directory: directory_tx,
        fields: FieldRegistry::new(FIELD_TICK),
    };

    let mut listeners = JoinSet::new();
    for channel in channels {
        let addr = format!("{bind_host}:{}", channel.port);
        let listener = TcpListener::bind(&addr).await?;
        info!(channel = %channel.name, %addr, "Channel listening");
        listeners.spawn(accept_loop(listener, channel.channel_id, services.clone()));
    }
    while listeners.join_next().await.is_some() {}
    Ok(())
}

fn configured_channels() -> Result<Vec<LoginChannel>, RuntimeError> {
    let worlds = load_login_worlds().map_err(|e| RuntimeError::Handler(e.to_string()))?;
    let channels: Vec<LoginChannel> = worlds.into_iter().flat_map(|w| w.channels).collect();
    let mut ports = HashSet::new();
    if let Some(duplicate) = channels.iter().find(|c| !ports.insert(c.port)) {
        return Err(RuntimeError::Handler(format!(
            "channel {} reuses port {}; every channel needs its own port",
            duplicate.name, duplicate.port
        )));
    }
    Ok(channels)
}

/// Load static game data before accepting players, so missing or broken
/// assets fail at startup instead of on the first map change.
async fn preload_game_data() -> Result<(), RuntimeError> {
    tokio::task::spawn_blocking(|| -> Result<(), String> {
        net::get_game_data().map_err(|e| e.to_string())?;
        net::get_npc_dialogue_data().map_err(|e| e.to_string())?;
        let quests = net::quests::catalog().map_err(|e| e.to_string())?;
        info!(
            supported = quests.supported().count(),
            "Quest catalog loaded"
        );
        net::items::data().map_err(|e| format!("item data: {e}"))?;
        net::mobs::data().map_err(|e| format!("mob data: {e}"))?;
        net::skills::data().map_err(|e| format!("skill data: {e}"))?;
        info!("Server content: {}", net::content::load()?);
        Ok(())
    })
    .await
    .map_err(|e| RuntimeError::Handler(e.to_string()))?
    .map_err(|e| RuntimeError::Handler(format!("Failed to load game data: {e}")))?;
    info!("Game data loaded");
    Ok(())
}

async fn accept_loop(listener: TcpListener, channel_id: u8, services: WorldServices) {
    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                debug!(%peer_addr, channel_id, "World connection accepted");
                let services = services.clone();
                tokio::spawn(async move {
                    match ClientActor::new(stream, peer_addr.ip(), channel_id, services).await {
                        Ok(actor) => actor.run().await,
                        Err(e) => error!(error = %e, "Failed to create ClientActor"),
                    }
                });
            }
            Err(e) => error!(error = %e, channel_id, "Error accepting world connection"),
        }
    }
}
