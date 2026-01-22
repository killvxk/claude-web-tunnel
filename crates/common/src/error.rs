//! Error types for Claude Web Tunnel

use thiserror::Error;

/// Common error type for the tunnel system
#[derive(Debug, Error)]
pub enum TunnelError {
    /// WebSocket connection error
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// Authentication error
    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Instance not found
    #[error("Instance not found: {0}")]
    InstanceNotFound(String),

    /// Agent not found
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    /// Agent offline
    #[error("Agent is offline: {0}")]
    AgentOffline(String),

    /// PTY error
    #[error("PTY error: {0}")]
    PtyError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Database error
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Redis error
    #[error("Redis error: {0}")]
    RedisError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Invalid message format
    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    /// Connection timeout
    #[error("Connection timeout")]
    Timeout,

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<serde_json::Error> for TunnelError {
    fn from(err: serde_json::Error) -> Self {
        TunnelError::SerializationError(err.to_string())
    }
}

/// Result type with TunnelError
pub type TunnelResult<T> = Result<T, TunnelError>;
