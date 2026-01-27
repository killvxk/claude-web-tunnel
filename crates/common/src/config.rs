//! Configuration types for Claude Web Tunnel

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Server connection settings
    pub server: ServerConnectionConfig,
    /// Agent identity settings
    pub agent: AgentIdentityConfig,
    /// Token settings
    pub tokens: TokenConfig,
    /// Directory settings
    #[serde(default)]
    pub directories: DirectoryConfig,
    /// Logging settings
    #[serde(default)]
    pub logging: LoggingConfig,
    /// Terminal settings
    #[serde(default)]
    pub terminal: TerminalConfig,
}

/// Server connection configuration for agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConnectionConfig {
    /// Server WebSocket URL (e.g., wss://your-server.com)
    pub url: String,
    /// Reconnect interval in seconds
    #[serde(default = "default_reconnect_interval")]
    pub reconnect_interval: u64,
    /// Heartbeat interval in seconds
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval: u64,
}

fn default_reconnect_interval() -> u64 {
    5
}

fn default_heartbeat_interval() -> u64 {
    30
}

/// Agent identity configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentityConfig {
    /// Agent display name
    pub name: String,
    /// Agent unique ID (auto-generated if not specified)
    #[serde(default)]
    pub id: Option<String>,
}

/// Token configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenConfig {
    /// Admin token (auto-generated if not specified)
    #[serde(default)]
    pub admin: Option<String>,
    /// Share token (auto-generated if not specified)
    #[serde(default)]
    pub share: Option<String>,
}

/// Directory configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DirectoryConfig {
    /// Allowed directories whitelist
    #[serde(default)]
    pub allowed: Vec<PathBuf>,
    /// Default working directory
    #[serde(default)]
    pub default: Option<PathBuf>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (error, warn, info, debug, trace)
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Log file path (optional, e.g., "./logs/server.log")
    #[serde(default)]
    pub file: Option<PathBuf>,
    /// Log rotation policy: "daily" or "hourly"
    #[serde(default = "default_log_rotation")]
    pub rotation: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_rotation() -> String {
    "daily".to_string()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file: None,
            rotation: default_log_rotation(),
        }
    }
}

/// Terminal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    /// Show visible terminal window (dual-mode: local window + web interface)
    /// When true, terminal runs in visible mode with input/output synchronized
    /// When false, terminal runs in background mode (web-only)
    #[serde(default = "default_terminal_visible")]
    pub visible: bool,
}

fn default_terminal_visible() -> bool {
    false
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            visible: default_terminal_visible(),
        }
    }
}

/// Terminal history configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalHistoryConfig {
    /// Enable terminal history recording
    #[serde(default = "default_terminal_history_enabled")]
    pub enabled: bool,
    /// Default buffer size in KB
    #[serde(default = "default_buffer_size_kb")]
    pub default_buffer_size_kb: u32,
    /// Maximum buffer size in KB
    #[serde(default = "default_max_buffer_size_kb")]
    pub max_buffer_size_kb: u32,
    /// Retention days for history records
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
}

fn default_terminal_history_enabled() -> bool {
    true
}

fn default_buffer_size_kb() -> u32 {
    64
}

fn default_max_buffer_size_kb() -> u32 {
    512
}

fn default_retention_days() -> u32 {
    7
}

impl Default for TerminalHistoryConfig {
    fn default() -> Self {
        Self {
            enabled: default_terminal_history_enabled(),
            default_buffer_size_kb: default_buffer_size_kb(),
            max_buffer_size_kb: default_max_buffer_size_kb(),
            retention_days: default_retention_days(),
        }
    }
}

/// Audit log configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogConfig {
    /// Enable audit logging
    #[serde(default = "default_audit_log_enabled")]
    pub enabled: bool,
    /// Retention days for audit logs
    #[serde(default = "default_audit_retention_days")]
    pub retention_days: u32,
}

fn default_audit_log_enabled() -> bool {
    true
}

fn default_audit_retention_days() -> u32 {
    30
}

impl Default for AuditLogConfig {
    fn default() -> Self {
        Self {
            enabled: default_audit_log_enabled(),
            retention_days: default_audit_retention_days(),
        }
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// HTTP server settings
    pub server: HttpServerConfig,
    /// Database settings
    pub database: DatabaseConfig,
    /// Security settings
    pub security: SecurityConfig,
    /// Logging settings
    #[serde(default)]
    pub logging: LoggingConfig,
    /// Terminal history settings
    #[serde(default)]
    pub terminal_history: TerminalHistoryConfig,
    /// Audit log settings
    #[serde(default)]
    pub audit_log: AuditLogConfig,
}

/// HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpServerConfig {
    /// Host to bind to
    #[serde(default = "default_host")]
    pub host: String,
    /// Port to listen on
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

impl Default for HttpServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database type: "sqlite" or "mysql"
    #[serde(rename = "type", default = "default_db_type")]
    pub db_type: String,
    /// SQLite database path
    #[serde(default)]
    pub sqlite_path: Option<PathBuf>,
    /// MySQL connection URL
    #[serde(default)]
    pub mysql_url: Option<String>,
    /// Redis connection URL (optional, rate limiting disabled if not set)
    #[serde(default)]
    pub redis_url: Option<String>,
}

fn default_db_type() -> String {
    "sqlite".to_string()
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Super admin token
    pub super_admin_token: String,
    /// Rate limit per minute for token validation
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: u32,
    /// Minimum token length
    #[serde(default = "default_token_min_length")]
    pub token_min_length: usize,
}

fn default_rate_limit() -> u32 {
    10
}

fn default_token_min_length() -> usize {
    32
}

impl AgentConfig {
    /// Load configuration from a TOML file
    pub fn from_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a TOML file
    pub fn to_file(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

impl ServerConfig {
    /// Load configuration from a TOML file
    pub fn from_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_deserialization() {
        let toml_content = r#"
[server]
url = "wss://example.com"
reconnect_interval = 10

[agent]
name = "Test Agent"

[tokens]
admin = "admin_token"
share = "share_token"

[directories]
allowed = ["/home/user/projects"]
default = "/home/user"

[logging]
level = "debug"
"#;
        let config: AgentConfig = toml::from_str(toml_content).unwrap();
        assert_eq!(config.server.url, "wss://example.com");
        assert_eq!(config.server.reconnect_interval, 10);
        assert_eq!(config.agent.name, "Test Agent");
    }

    #[test]
    fn test_server_config_deserialization() {
        let toml_content = r#"
[server]
host = "0.0.0.0"
port = 9090

[database]
type = "mysql"
mysql_url = "mysql://user:pass@localhost/db"
redis_url = "redis://localhost:6379"

[security]
super_admin_token = "super_secret"
rate_limit_per_minute = 20

[logging]
level = "info"
"#;
        let config: ServerConfig = toml::from_str(toml_content).unwrap();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 9090);
        assert_eq!(config.database.db_type, "mysql");
    }
}
