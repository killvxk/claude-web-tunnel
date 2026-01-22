//! Claude Tunnel Agent - Local agent for Claude Code remote access
//!
//! This binary connects to a remote tunnel server and manages local
//! Claude Code instances via PTY.

// Use mimalloc as global allocator for musl builds (better performance than musl's default allocator)
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod cli;
mod config;
mod connection;
mod instance;
mod pty;
mod logging;

use anyhow::Result;
use clap::Parser;
use tracing::{info, warn};

use crate::cli::Args;
use crate::config::AgentRuntime;
use crate::connection::TunnelConnection;
use crate::logging::init_logging;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Load or create configuration
    let runtime = AgentRuntime::from_args(&args)?;

    // Initialize logging with file rotation
    let _log_guard = init_logging(&runtime.config.logging);

    info!("Claude Tunnel Agent starting...");
    info!(
        "Agent: {} (ID: {})",
        runtime.config.agent.name,
        runtime.agent_id
    );
    info!("Server: {}", runtime.config.server.url);
    info!("Admin Token: {}", &runtime.admin_token);
    info!("Share Token: {}", &runtime.share_token);

    // Start the tunnel connection
    let mut connection = TunnelConnection::new(runtime);

    // Run the main loop with reconnection support
    loop {
        match connection.run().await {
            Ok(_) => {
                info!("Connection closed normally");
                break;
            }
            Err(e) => {
                warn!("Connection error: {}", e);
                let reconnect_interval = connection.runtime.config.server.reconnect_interval;
                info!("Reconnecting in {} seconds...", reconnect_interval);
                tokio::time::sleep(tokio::time::Duration::from_secs(reconnect_interval)).await;
            }
        }
    }

    Ok(())
}
