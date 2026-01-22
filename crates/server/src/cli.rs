//! Command line argument parsing for the server

use clap::Parser;
use std::path::PathBuf;

/// Claude Tunnel Server - Host Claude Code tunnel service
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "server.toml")]
    pub config: PathBuf,

    /// Log level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Override host to bind to
    #[arg(long)]
    pub host: Option<String>,

    /// Override port to listen on
    #[arg(long)]
    pub port: Option<u16>,
}
