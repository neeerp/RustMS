use std::env;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

// Workers get 16 MiB stacks: in a debug build a client actor's NPC talk
// (handle_server_message, start_npc_conversation, talk_to) takes about 1.95
// MiB of frames, past tokio's 2 MiB default, and aborts the whole server.
const WORKER_STACK: usize = 16 * 1024 * 1024;

fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(WORKER_STACK)
        .build()
        .expect("Failed to build the tokio runtime")
        .block_on(run());
}

async fn run() {
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

    // Off unless configured (runtime::vote_hook).
    if let Some(hook) = runtime::vote_hook::VoteHook::from_env() {
        tokio::spawn(hook.serve());
    }

    // Channels listen on the ports from the channel configuration
    // (RUSTMS_LOGIN_CHANNELS); this only picks the interface.
    let bind_host = env::var("RUSTMS_WORLD_BIND_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    if let Err(e) = runtime::run_world_server(&bind_host).await {
        error!(error = %e, "World server error");
        std::process::exit(1);
    }
}
