//! Core data types for Claude Web Tunnel

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User role in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Super admin - can manage all agents
    SuperAdmin,
    /// Agent admin - can manage single agent
    Admin,
    /// Regular user - can only operate terminals
    User,
}

impl Role {
    /// Check if this role can create instances
    pub fn can_create_instance(&self) -> bool {
        matches!(self, Role::SuperAdmin | Role::Admin)
    }

    /// Check if this role can close instances
    pub fn can_close_instance(&self) -> bool {
        matches!(self, Role::SuperAdmin | Role::Admin)
    }

    /// Check if this role can manage all agents
    pub fn can_manage_all_agents(&self) -> bool {
        matches!(self, Role::SuperAdmin)
    }
}

/// Agent connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    /// Agent is connected and operational
    Online,
    /// Agent is disconnected but instances are preserved
    Offline,
}

/// Instance running status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceStatus {
    /// Instance is running normally
    Running,
    /// Instance is suspended (agent offline)
    Suspended,
    /// Instance is stopped
    Stopped,
}

/// Agent information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Unique identifier
    pub id: Uuid,
    /// Display name
    pub name: String,
    /// Connection status
    pub status: AgentStatus,
    /// Connection timestamp
    pub connected_at: Option<DateTime<Utc>>,
    /// List of instances
    #[serde(default)]
    pub instances: Vec<Instance>,
}

impl Agent {
    /// Create a new agent
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            status: AgentStatus::Offline,
            connected_at: None,
            instances: Vec::new(),
        }
    }
}

/// Claude Code instance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    /// Unique identifier
    pub id: Uuid,
    /// Parent agent ID
    pub agent_id: Uuid,
    /// Working directory
    pub cwd: String,
    /// Instance status
    pub status: InstanceStatus,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Number of attached users
    #[serde(default)]
    pub attached_users: usize,
}

impl Instance {
    /// Create a new instance
    pub fn new(agent_id: Uuid, cwd: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            agent_id,
            cwd: cwd.into(),
            status: InstanceStatus::Running,
            created_at: Utc::now(),
            attached_users: 0,
        }
    }
}

/// User session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    /// Session ID
    pub id: Uuid,
    /// User role
    pub role: Role,
    /// Associated agent ID (None for super admin)
    pub agent_id: Option<Uuid>,
    /// Currently attached instance IDs
    #[serde(default)]
    pub attached_instances: Vec<Uuid>,
}

impl UserSession {
    /// Create a new user session
    pub fn new(role: Role, agent_id: Option<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            role,
            agent_id,
            attached_instances: Vec::new(),
        }
    }
}

/// Terminal dimensions
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct TerminalSize {
    /// Number of columns
    pub cols: u16,
    /// Number of rows
    pub rows: u16,
}

impl TerminalSize {
    /// Create new terminal size
    pub fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }
}

/// Agent information for admin panel (includes online users count)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Unique identifier
    pub id: Uuid,
    /// Display name
    pub name: String,
    /// Connection status
    pub status: AgentStatus,
    /// Connection timestamp
    pub connected_at: Option<DateTime<Utc>>,
    /// Number of instances
    pub instance_count: usize,
    /// Number of connected users
    pub user_count: usize,
}

/// Global statistics for admin panel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalStats {
    /// Total number of agents (online + offline)
    pub total_agents: usize,
    /// Number of online agents
    pub online_agents: usize,
    /// Total number of instances
    pub total_instances: usize,
    /// Number of running instances
    pub running_instances: usize,
    /// Total number of connected users
    pub total_users: usize,
}

/// Audit log entry for admin viewing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Record ID
    pub id: i64,
    /// Event timestamp (ISO 8601)
    pub timestamp: String,
    /// Event type (auth_success, auth_failure, create_instance, etc.)
    pub event_type: String,
    /// User role (super_admin, admin, user)
    pub user_role: String,
    /// Associated agent ID (optional)
    pub agent_id: Option<Uuid>,
    /// Associated instance ID (optional)
    pub instance_id: Option<Uuid>,
    /// Target ID for operations (optional)
    pub target_id: Option<Uuid>,
    /// Client IP address
    pub client_ip: String,
    /// Operation success
    pub success: bool,
    /// Additional details (optional)
    pub details: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_permissions() {
        assert!(Role::SuperAdmin.can_create_instance());
        assert!(Role::SuperAdmin.can_close_instance());
        assert!(Role::SuperAdmin.can_manage_all_agents());

        assert!(Role::Admin.can_create_instance());
        assert!(Role::Admin.can_close_instance());
        assert!(!Role::Admin.can_manage_all_agents());

        assert!(!Role::User.can_create_instance());
        assert!(!Role::User.can_close_instance());
        assert!(!Role::User.can_manage_all_agents());
    }

    #[test]
    fn test_agent_creation() {
        let agent = Agent::new("Test Agent");
        assert!(!agent.id.is_nil());
        assert_eq!(agent.name, "Test Agent");
        assert_eq!(agent.status, AgentStatus::Offline);
    }

    #[test]
    fn test_instance_creation() {
        let agent_id = Uuid::new_v4();
        let instance = Instance::new(agent_id, "/home/user/project");
        assert!(!instance.id.is_nil());
        assert_eq!(instance.agent_id, agent_id);
        assert_eq!(instance.cwd, "/home/user/project");
        assert_eq!(instance.status, InstanceStatus::Running);
    }
}
