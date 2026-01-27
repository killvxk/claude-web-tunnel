//! WebSocket protocol messages for Claude Web Tunnel
//!
//! This module defines all message types used for communication between:
//! - Agent <-> Server
//! - User <-> Server

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::{AgentInfo, AuditLogEntry, GlobalStats, Instance, Role, TerminalSize};

// ============================================================================
// Reconnection Support
// ============================================================================

/// Information about an existing instance for reconnection sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExistingInstance {
    /// Instance ID
    pub id: Uuid,
    /// Working directory
    pub cwd: String,
}

// ============================================================================
// Agent <-> Server Messages
// ============================================================================

/// Messages sent from Agent to Server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentMessage {
    /// Register agent with server
    Register {
        /// Agent unique ID
        agent_id: Uuid,
        /// Agent display name
        name: String,
        /// Admin token for this agent
        admin_token: String,
        /// Share token for this agent
        share_token: String,
        /// Existing instances for reconnection sync (optional, backward compatible)
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        existing_instances: Vec<ExistingInstance>,
    },
    /// Report instance created
    InstanceCreated {
        /// Instance ID
        instance_id: Uuid,
        /// Working directory
        cwd: String,
    },
    /// Report instance closed
    InstanceClosed {
        /// Instance ID
        instance_id: Uuid,
    },
    /// PTY output data
    PtyOutput {
        /// Instance ID
        instance_id: Uuid,
        /// Output data (base64 encoded)
        data: String,
    },
    /// Heartbeat
    Heartbeat,
    /// Error report
    Error {
        /// Error message
        message: String,
    },
}

/// Messages sent from Server to Agent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerToAgentMessage {
    /// Registration successful
    Registered {
        /// Confirmation message
        message: String,
    },
    /// Create a new Claude Code instance
    CreateInstance {
        /// Instance ID (pre-assigned by server)
        instance_id: Uuid,
        /// Working directory
        cwd: String,
    },
    /// Close an instance
    CloseInstance {
        /// Instance ID
        instance_id: Uuid,
    },
    /// PTY input data
    PtyInput {
        /// Instance ID
        instance_id: Uuid,
        /// Input data (base64 encoded)
        data: String,
    },
    /// Resize terminal
    Resize {
        /// Instance ID
        instance_id: Uuid,
        /// New terminal size
        #[serde(flatten)]
        size: TerminalSize,
    },
    /// Ping (keep-alive)
    Ping,
    /// Error message
    Error {
        /// Error message
        message: String,
    },
}

// ============================================================================
// User <-> Server Messages
// ============================================================================

/// Messages sent from User to Server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserMessage {
    /// Authenticate with token
    Auth {
        /// Authentication token
        token: String,
    },
    /// Request to create a new instance (admin only)
    CreateInstance {
        /// Working directory
        cwd: String,
    },
    /// Request to close an instance (admin only)
    CloseInstance {
        /// Instance ID
        instance_id: Uuid,
    },
    /// Attach to an instance
    Attach {
        /// Instance ID
        instance_id: Uuid,
    },
    /// Detach from an instance
    Detach {
        /// Instance ID
        instance_id: Uuid,
    },
    /// PTY input data
    PtyInput {
        /// Instance ID
        instance_id: Uuid,
        /// Input data (base64 encoded)
        data: String,
    },
    /// Resize terminal
    Resize {
        /// Instance ID
        instance_id: Uuid,
        /// New terminal size
        #[serde(flatten)]
        size: TerminalSize,
    },
    /// Request instance list
    ListInstances,
    /// Heartbeat
    Heartbeat,
    // ========================================================================
    // Admin commands (SuperAdmin only)
    // ========================================================================
    /// Request admin statistics (SuperAdmin only)
    GetAdminStats,
    /// Force disconnect an agent (SuperAdmin only)
    ForceDisconnectAgent {
        /// Agent ID to disconnect
        agent_id: Uuid,
    },
    /// Force close an instance (SuperAdmin only)
    ForceCloseInstance {
        /// Instance ID to close
        instance_id: Uuid,
    },
    /// Delete an agent record (SuperAdmin only)
    DeleteAgent {
        /// Agent ID to delete
        agent_id: Uuid,
    },
    // ========================================================================
    // Tag commands (Admin only)
    // ========================================================================
    /// Get all unique tags
    GetAllTags,
    /// Get tags for a specific agent
    GetAgentTags {
        /// Agent ID
        agent_id: Uuid,
    },
    /// Add a tag to an agent
    AddAgentTag {
        /// Agent ID
        agent_id: Uuid,
        /// Tag name
        tag: String,
    },
    /// Remove a tag from an agent
    RemoveAgentTag {
        /// Agent ID
        agent_id: Uuid,
        /// Tag name
        tag: String,
    },
    // ========================================================================
    // Audit log commands (SuperAdmin only)
    // ========================================================================
    /// Get audit logs (SuperAdmin only)
    GetAuditLogs {
        /// Number of logs to retrieve (default 100)
        #[serde(default)]
        limit: Option<u32>,
        /// Offset for pagination
        #[serde(default)]
        offset: Option<u32>,
        /// Filter by event type (optional)
        #[serde(default)]
        event_type: Option<String>,
    },
    // ========================================================================
    // Working Agent commands (SuperAdmin only)
    // ========================================================================
    /// Select a working agent (SuperAdmin only)
    /// Allows SuperAdmin to operate on a specific agent's instances
    SelectWorkingAgent {
        /// Agent ID to work with
        agent_id: Uuid,
    },
    /// Clear the working agent selection (SuperAdmin only)
    ClearWorkingAgent,
    /// List instances of a specific agent (SuperAdmin only)
    ListAgentInstances {
        /// Agent ID to list instances for
        agent_id: Uuid,
    },
}

/// Messages sent from Server to User
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerToUserMessage {
    /// Authentication result
    AuthResult {
        /// Whether authentication succeeded
        success: bool,
        /// User role if successful
        role: Option<Role>,
        /// Agent name if successful
        agent_name: Option<String>,
        /// Agent ID if authenticated (None for super admin)
        agent_id: Option<Uuid>,
        /// Error message if failed
        error: Option<String>,
    },
    /// List of instances
    InstanceList {
        /// Available instances
        instances: Vec<Instance>,
    },
    /// Instance created notification
    InstanceCreated {
        /// Created instance
        instance: Instance,
    },
    /// Instance closed notification
    InstanceClosed {
        /// Instance ID
        instance_id: Uuid,
    },
    /// PTY output data
    PtyOutput {
        /// Instance ID
        instance_id: Uuid,
        /// Output data (base64 encoded)
        data: String,
    },
    /// User joined notification
    UserJoined {
        /// Instance ID
        instance_id: Uuid,
        /// Current user count
        user_count: usize,
    },
    /// User left notification
    UserLeft {
        /// Instance ID
        instance_id: Uuid,
        /// Current user count
        user_count: usize,
    },
    /// Agent status changed
    AgentStatusChanged {
        /// Agent ID
        agent_id: Uuid,
        /// Online or offline
        online: bool,
    },
    /// Error message
    Error {
        /// Error message
        message: String,
    },
    /// Pong (keep-alive response)
    Pong,
    // ========================================================================
    // Admin responses (SuperAdmin only)
    // ========================================================================
    /// Admin statistics response
    AdminStats {
        /// List of all agents
        agents: Vec<AgentInfo>,
        /// Global statistics
        stats: GlobalStats,
    },
    /// Agent disconnected notification (admin)
    AgentDisconnected {
        /// Agent ID
        agent_id: Uuid,
    },
    /// Agent deleted notification (admin)
    AgentDeleted {
        /// Agent ID
        agent_id: Uuid,
    },
    // ========================================================================
    // Tag responses
    // ========================================================================
    /// List of all tags
    TagList {
        /// All unique tags
        tags: Vec<String>,
    },
    /// Tags for a specific agent
    AgentTags {
        /// Agent ID
        agent_id: Uuid,
        /// Agent's tags
        tags: Vec<String>,
    },
    /// Tag added notification
    TagAdded {
        /// Agent ID
        agent_id: Uuid,
        /// Tag name
        tag: String,
    },
    /// Tag removed notification
    TagRemoved {
        /// Agent ID
        agent_id: Uuid,
        /// Tag name
        tag: String,
    },
    // ========================================================================
    // Audit log responses (SuperAdmin only)
    // ========================================================================
    /// Audit log list response
    AuditLogList {
        /// List of audit log entries
        logs: Vec<AuditLogEntry>,
        /// Total count (for pagination)
        total: u64,
    },
    // ========================================================================
    // Working Agent responses (SuperAdmin only)
    // ========================================================================
    /// Working agent selected notification
    WorkingAgentSelected {
        /// Agent ID that was selected
        agent_id: Uuid,
        /// Agent name
        agent_name: String,
        /// Whether the operation succeeded
        success: bool,
        /// Error message if failed
        error: Option<String>,
    },
    /// Working agent cleared notification
    WorkingAgentCleared,
}

// ============================================================================
// Helper functions
// ============================================================================

impl AgentMessage {
    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

impl ServerToAgentMessage {
    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

impl UserMessage {
    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

impl ServerToUserMessage {
    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_message_serialization() {
        let msg = AgentMessage::Register {
            agent_id: Uuid::new_v4(),
            name: "Test Agent".to_string(),
            admin_token: "admin123".to_string(),
            share_token: "share456".to_string(),
            existing_instances: vec![],
        };
        let json = msg.to_json().unwrap();
        assert!(json.contains("\"type\":\"register\""));

        let parsed = AgentMessage::from_json(&json).unwrap();
        match parsed {
            AgentMessage::Register { name, .. } => assert_eq!(name, "Test Agent"),
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_user_message_serialization() {
        let msg = UserMessage::Auth {
            token: "test_token".to_string(),
        };
        let json = msg.to_json().unwrap();
        assert!(json.contains("\"type\":\"auth\""));

        let parsed = UserMessage::from_json(&json).unwrap();
        match parsed {
            UserMessage::Auth { token } => assert_eq!(token, "test_token"),
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_pty_output_message() {
        let instance_id = Uuid::new_v4();
        let msg = ServerToUserMessage::PtyOutput {
            instance_id,
            data: "SGVsbG8gV29ybGQ=".to_string(), // "Hello World" in base64
        };
        let json = msg.to_json().unwrap();
        assert!(json.contains("\"type\":\"pty_output\""));
    }
}
