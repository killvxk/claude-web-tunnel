//! Command line argument parsing for the agent

use clap::Parser;
use std::path::PathBuf;

/// Claude Tunnel Agent - Connect local Claude Code to remote server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Server WebSocket URL (e.g., wss://your-server.com)
    #[arg(short, long, env = "TUNNEL_SERVER_URL")]
    pub server: Option<String>,

    /// Agent display name
    #[arg(short, long, env = "TUNNEL_AGENT_NAME")]
    pub name: Option<String>,

    /// Admin token (auto-generated if not specified)
    #[arg(long, env = "TUNNEL_ADMIN_TOKEN")]
    pub admin_token: Option<String>,

    /// Share token (auto-generated if not specified)
    #[arg(long, env = "TUNNEL_SHARE_TOKEN")]
    pub share_token: Option<String>,

    /// Configuration file path
    #[arg(short, long, default_value = "agent.toml")]
    pub config: PathBuf,

    /// Log level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Generate a new configuration file and exit
    #[arg(long)]
    pub init: bool,

    /// Print tokens and exit (for sharing)
    #[arg(long)]
    pub show_tokens: bool,
}
