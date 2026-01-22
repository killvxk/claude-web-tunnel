//! Configuration management for the agent

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use uuid::Uuid;

use common::{
    AgentConfig, AgentIdentityConfig, DirectoryConfig, LoggingConfig, ServerConnectionConfig,
    TokenConfig,
};

use crate::cli::Args;

/// Runtime configuration with resolved values
pub struct AgentRuntime {
    /// Loaded/merged configuration
    pub config: AgentConfig,
    /// Resolved agent ID
    pub agent_id: Uuid,
    /// Resolved admin token
    pub admin_token: String,
    /// Resolved share token
    pub share_token: String,
}

impl AgentRuntime {
    /// Create runtime configuration from CLI args (and optional config file)
    pub fn from_args(args: &Args) -> Result<Self> {
        // Try to load config file, or create default
        let mut config = if args.config.exists() {
            AgentConfig::from_file(&args.config)
                .map_err(|e| anyhow!("Failed to load config: {}", e))?
        } else {
            Self::default_config()
        };

        // Override with CLI arguments
        if let Some(ref server) = args.server {
            config.server.url = server.clone();
        }
        if let Some(ref name) = args.name {
            config.agent.name = name.clone();
        }
        if let Some(ref token) = args.admin_token {
            config.tokens.admin = Some(token.clone());
        }
        if let Some(ref token) = args.share_token {
            config.tokens.share = Some(token.clone());
        }

        // Validate configuration
        if config.server.url.is_empty() {
            return Err(anyhow!(
                "Server URL is required. Use --server or set in config file."
            ));
        }

        // Resolve agent ID (generate if not specified)
        let agent_id = config
            .agent
            .id
            .as_ref()
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::new_v4);

        // Resolve tokens (generate if not specified)
        let admin_token = config
            .tokens
            .admin
            .clone()
            .unwrap_or_else(|| generate_token());
        let share_token = config
            .tokens
            .share
            .clone()
            .unwrap_or_else(|| generate_token());

        // Update config with resolved values
        config.agent.id = Some(agent_id.to_string());
        config.tokens.admin = Some(admin_token.clone());
        config.tokens.share = Some(share_token.clone());

        // Save updated config if it was auto-generated
        if !args.config.exists() || args.init {
            config.to_file(&args.config).ok(); // Ignore save errors
        }

        Ok(Self {
            config,
            agent_id,
            admin_token,
            share_token,
        })
    }

    /// Create a default configuration
    fn default_config() -> AgentConfig {
        AgentConfig {
            server: ServerConnectionConfig {
                url: String::new(),
                reconnect_interval: 5,
                heartbeat_interval: 30,
            },
            agent: AgentIdentityConfig {
                name: hostname::get()
                    .ok()
                    .and_then(|h| h.into_string().ok())
                    .unwrap_or_else(|| "Unknown".to_string()),
                id: None,
            },
            tokens: TokenConfig {
                admin: None,
                share: None,
            },
            directories: DirectoryConfig {
                allowed: vec![],
                default: std::env::current_dir().ok(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file: Some(std::path::PathBuf::from("./logs/agent.log")),
                rotation: "daily".to_string(),
            },
        }
    }
}

/// Generate a random token (32 bytes, URL-safe base64 encoded)
pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_token() {
        let token1 = generate_token();
        let token2 = generate_token();

        // Tokens should be different
        assert_ne!(token1, token2);

        // Tokens should be 43 characters (32 bytes base64 without padding)
        assert_eq!(token1.len(), 43);
        assert_eq!(token2.len(), 43);
    }
}
