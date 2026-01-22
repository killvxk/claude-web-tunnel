//! Configuration management for the server

use anyhow::{anyhow, Result};
use std::path::PathBuf;

use common::{AuditLogConfig, DatabaseConfig, HttpServerConfig, LoggingConfig, SecurityConfig, ServerConfig, TerminalHistoryConfig};

use crate::cli::Args;

/// Runtime configuration with resolved values
#[derive(Clone)]
pub struct ServerRuntime {
    /// Loaded/merged configuration
    pub config: ServerConfig,
}

impl ServerRuntime {
    /// Create runtime configuration from CLI args
    pub fn from_args(args: &Args) -> Result<Self> {
        // Load config file
        let mut config = if args.config.exists() {
            ServerConfig::from_file(&args.config)
                .map_err(|e| anyhow!("Failed to load config: {}", e))?
        } else {
            Self::default_config()
        };

        // Override with CLI arguments
        if let Some(ref host) = args.host {
            config.server.host = host.clone();
        }
        if let Some(port) = args.port {
            config.server.port = port;
        }

        // Validate configuration
        if config.security.super_admin_token.is_empty()
            || config.security.super_admin_token == "CHANGE_ME_TO_A_RANDOM_TOKEN"
        {
            return Err(anyhow!(
                "Super admin token must be configured. Please set security.super_admin_token in config file."
            ));
        }

        Ok(Self { config })
    }

    /// Create a default configuration
    fn default_config() -> ServerConfig {
        ServerConfig {
            server: HttpServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
            },
            database: DatabaseConfig {
                db_type: "sqlite".to_string(),
                sqlite_path: Some(PathBuf::from("data/tunnel.db")),
                mysql_url: None,
                redis_url: Some("redis://127.0.0.1:6379".to_string()),
            },
            security: SecurityConfig {
                super_admin_token: String::new(),
                rate_limit_per_minute: 10,
                token_min_length: 32,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file: Some(PathBuf::from("./logs/server.log")),
                rotation: "daily".to_string(),
            },
            terminal_history: TerminalHistoryConfig::default(),
            audit_log: AuditLogConfig::default(),
        }
    }
}
