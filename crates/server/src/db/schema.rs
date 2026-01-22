//! Database schema types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Agent record in database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AgentRecord {
    /// Agent UUID
    pub id: String,
    /// Display name
    pub name: String,
    /// SHA-256 hash of admin token
    pub admin_token_hash: String,
    /// SHA-256 hash of share token
    pub share_token_hash: String,
    /// Creation timestamp
    pub created_at: String,
    /// Last connection timestamp
    pub last_connected_at: Option<String>,
}

impl AgentRecord {
    /// Parse created_at as DateTime
    #[allow(dead_code)]
    pub fn created_at_datetime(&self) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(&self.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .ok()
            .or_else(|| {
                // Try SQLite datetime format
                chrono::NaiveDateTime::parse_from_str(&self.created_at, "%Y-%m-%d %H:%M:%S")
                    .map(|ndt| ndt.and_utc())
                    .ok()
            })
    }

    /// Parse last_connected_at as DateTime
    #[allow(dead_code)]
    pub fn last_connected_at_datetime(&self) -> Option<DateTime<Utc>> {
        self.last_connected_at.as_ref().and_then(|s| {
            DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
                .or_else(|| {
                    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                        .map(|ndt| ndt.and_utc())
                        .ok()
                })
        })
    }
}

/// Agent tag record in database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AgentTagRecord {
    /// Agent UUID
    pub agent_id: String,
    /// Tag name
    pub tag: String,
    /// Creation timestamp
    pub created_at: String,
}

/// Terminal history record in database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TerminalHistoryRecord {
    /// Record ID
    pub id: i64,
    /// Instance UUID
    pub instance_id: String,
    /// Sequence number for ordering
    pub sequence_number: i64,
    /// Base64 encoded output data
    pub output_data: String,
    /// Size in bytes
    pub byte_size: i32,
    /// Creation timestamp
    pub created_at: String,
}

/// Terminal history metadata record in database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TerminalHistoryMetaRecord {
    /// Instance UUID
    pub instance_id: String,
    /// Total bytes stored
    pub total_bytes: i64,
    /// Next sequence number
    pub next_sequence: i64,
    /// Buffer size limit in KB
    pub buffer_size_kb: i32,
}

/// Audit log record in database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditLogRecord {
    /// Record ID
    pub id: i64,
    /// Event timestamp
    pub timestamp: String,
    /// Event type (auth_success, auth_failure, create_instance, etc.)
    pub event_type: String,
    /// User session ID
    pub session_id: String,
    /// User role (super_admin, admin, user)
    pub user_role: String,
    /// Associated agent ID (optional)
    pub agent_id: Option<String>,
    /// Associated instance ID (optional)
    pub instance_id: Option<String>,
    /// Target ID for operations like DeleteAgent (optional)
    pub target_id: Option<String>,
    /// Client IP address
    pub client_ip: String,
    /// Operation success (1) or failure (0)
    pub success: i32,
    /// Additional details in JSON format (optional)
    pub details: Option<String>,
}
