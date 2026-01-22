//! Application state management

use std::collections::HashMap;

use anyhow::Result;
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use common::{Agent, AgentInfo, AgentStatus, GlobalStats, Instance, InstanceStatus, Role, ServerToAgentMessage, ServerToUserMessage};

use crate::auth::hash_token;
use crate::config::ServerRuntime;
use crate::db::AgentRepository;
use crate::rate_limit::RateLimiter;

/// Connected agent information
pub struct ConnectedAgent {
    /// Agent data
    pub agent: Agent,
    /// Admin token hash (SHA-256)
    pub admin_token_hash: String,
    /// Share token hash (SHA-256)
    pub share_token_hash: String,
    /// Channel to send messages to agent
    pub tx: mpsc::Sender<ServerToAgentMessage>,
    /// Active instances
    pub instances: HashMap<Uuid, Instance>,
}

/// Connected user session
pub struct UserSession {
    /// Session ID
    #[allow(dead_code)]
    pub id: Uuid,
    /// User role
    #[allow(dead_code)]
    pub role: Role,
    /// Associated agent ID (None for super admin viewing all)
    pub agent_id: Option<Uuid>,
    /// Currently attached instance IDs
    pub attached_instances: Vec<Uuid>,
    /// Channel to send messages to user
    pub tx: mpsc::Sender<ServerToUserMessage>,
}

/// Application state shared across handlers
pub struct AppState {
    /// Server runtime configuration
    pub runtime: ServerRuntime,
    /// Agent repository for database operations
    pub agent_repo: AgentRepository,
    /// Rate limiter (optional)
    pub rate_limiter: Option<RateLimiter>,
    /// Connected agents (agent_id -> ConnectedAgent)
    pub agents: RwLock<HashMap<Uuid, ConnectedAgent>>,
    /// Connected users (session_id -> UserSession)
    pub users: RwLock<HashMap<Uuid, UserSession>>,
    /// Broadcast channel for agent status changes
    pub agent_status_tx: broadcast::Sender<(Uuid, bool)>,
}

impl AppState {
    /// Create new application state
    pub async fn new(
        runtime: ServerRuntime,
        agent_repo: AgentRepository,
        rate_limiter: Option<RateLimiter>,
    ) -> Result<Self> {
        let (agent_status_tx, _) = broadcast::channel(100);

        Ok(Self {
            runtime,
            agent_repo,
            rate_limiter,
            agents: RwLock::new(HashMap::new()),
            users: RwLock::new(HashMap::new()),
            agent_status_tx,
        })
    }

    /// Register a new agent
    pub async fn register_agent(
        &self,
        agent_id: Uuid,
        name: String,
        admin_token: String,
        share_token: String,
        tx: mpsc::Sender<ServerToAgentMessage>,
    ) {
        // Hash tokens before storing
        let admin_token_hash = hash_token(&admin_token);
        let share_token_hash = hash_token(&share_token);

        let agent = Agent {
            id: agent_id,
            name: name.clone(),
            status: AgentStatus::Online,
            connected_at: Some(chrono::Utc::now()),
            instances: Vec::new(), // This will be populated from HashMap when needed
        };

        let connected = ConnectedAgent {
            agent,
            admin_token_hash: admin_token_hash.clone(),
            share_token_hash: share_token_hash.clone(),
            tx,
            instances: HashMap::new(),
        };

        let mut agents = self.agents.write().await;
        agents.insert(agent_id, connected);

        // Persist to database (non-blocking, log errors)
        let repo = self.agent_repo.clone();
        let name_clone = name.clone();
        tokio::spawn(async move {
            if let Err(e) = repo.upsert_agent(agent_id, &name_clone, &admin_token, &share_token).await {
                tracing::error!("Failed to persist agent to database: {}", e);
            }
        });

        // Broadcast agent online
        let _ = self.agent_status_tx.send((agent_id, true));
    }

    /// Unregister an agent
    pub async fn unregister_agent(&self, agent_id: Uuid) {
        let mut agents = self.agents.write().await;
        agents.remove(&agent_id);

        // Broadcast agent offline
        let _ = self.agent_status_tx.send((agent_id, false));
    }

    /// Authenticate a token and return role and agent ID
    /// Uses hashed token comparison for security
    pub async fn authenticate(&self, token: &str) -> Option<(Role, Option<Uuid>)> {
        // Check super admin token (direct comparison for config-based token)
        if token == self.runtime.config.security.super_admin_token {
            return Some((Role::SuperAdmin, None));
        }

        let token_hash = hash_token(token);

        // First check in-memory connected agents (fast path)
        {
            let agents = self.agents.read().await;
            for (agent_id, agent) in agents.iter() {
                if token_hash == agent.admin_token_hash {
                    return Some((Role::Admin, Some(*agent_id)));
                }
                if token_hash == agent.share_token_hash {
                    return Some((Role::User, Some(*agent_id)));
                }
            }
        }

        // Then check database for offline/registered agents
        if let Ok(Some(record)) = self.agent_repo.find_by_admin_token(token).await {
            if let Ok(id) = record.id.parse::<Uuid>() {
                return Some((Role::Admin, Some(id)));
            }
        }
        if let Ok(Some(record)) = self.agent_repo.find_by_share_token(token).await {
            if let Ok(id) = record.id.parse::<Uuid>() {
                return Some((Role::User, Some(id)));
            }
        }

        None
    }

    /// Get agent by ID (with instances populated from HashMap)
    pub async fn get_agent(&self, agent_id: Uuid) -> Option<Agent> {
        let agents = self.agents.read().await;
        agents.get(&agent_id).map(|a| {
            let mut agent = a.agent.clone();
            agent.instances = a.instances.values().cloned().collect();
            agent
        })
    }

    /// Get all agents (for super admin, with instances populated)
    #[allow(dead_code)]
    pub async fn get_all_agents(&self) -> Vec<Agent> {
        let agents = self.agents.read().await;
        agents.values().map(|a| {
            let mut agent = a.agent.clone();
            agent.instances = a.instances.values().cloned().collect();
            agent
        }).collect()
    }

    /// Get instances for an agent
    pub async fn get_instances(&self, agent_id: Uuid) -> Vec<Instance> {
        let agents = self.agents.read().await;
        agents
            .get(&agent_id)
            .map(|a| a.instances.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Add instance to agent (only to HashMap, Vec is populated on-demand)
    pub async fn add_instance(&self, agent_id: Uuid, instance: Instance) {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(&agent_id) {
            agent.instances.insert(instance.id, instance);
        }
    }

    /// Remove instance from agent (only from HashMap)
    pub async fn remove_instance(&self, agent_id: Uuid, instance_id: Uuid) {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(&agent_id) {
            agent.instances.remove(&instance_id);
        }
    }

    /// Send message to agent
    pub async fn send_to_agent(&self, agent_id: Uuid, msg: ServerToAgentMessage) -> Result<()> {
        let agents = self.agents.read().await;
        if let Some(agent) = agents.get(&agent_id) {
            agent.tx.send(msg).await.map_err(|e| anyhow::anyhow!("Failed to send to agent: {}", e))?;
        }
        Ok(())
    }

    /// Register a user session
    pub async fn register_user(
        &self,
        session_id: Uuid,
        role: Role,
        agent_id: Option<Uuid>,
        tx: mpsc::Sender<ServerToUserMessage>,
    ) {
        let session = UserSession {
            id: session_id,
            role,
            agent_id,
            attached_instances: Vec::new(),
            tx,
        };

        let mut users = self.users.write().await;
        users.insert(session_id, session);
    }

    /// Unregister a user session
    pub async fn unregister_user(&self, session_id: Uuid) {
        let mut users = self.users.write().await;
        users.remove(&session_id);
    }

    /// Broadcast message to all users attached to an instance
    pub async fn broadcast_to_instance(&self, instance_id: Uuid, msg: ServerToUserMessage) {
        let users = self.users.read().await;
        for session in users.values() {
            if session.attached_instances.contains(&instance_id) {
                let _ = session.tx.send(msg.clone()).await;
            }
        }
    }

    /// Attach user to instance
    pub async fn attach_user_to_instance(&self, session_id: Uuid, instance_id: Uuid) {
        let mut users = self.users.write().await;
        if let Some(session) = users.get_mut(&session_id) {
            if !session.attached_instances.contains(&instance_id) {
                session.attached_instances.push(instance_id);
            }
        }
    }

    /// Detach user from instance
    pub async fn detach_user_from_instance(&self, session_id: Uuid, instance_id: Uuid) {
        let mut users = self.users.write().await;
        if let Some(session) = users.get_mut(&session_id) {
            session.attached_instances.retain(|&id| id != instance_id);
        }
    }

    /// Get count of users attached to an instance
    pub async fn get_instance_user_count(&self, instance_id: Uuid) -> usize {
        let users = self.users.read().await;
        users
            .values()
            .filter(|s| s.attached_instances.contains(&instance_id))
            .count()
    }

    /// Send a message to a specific user session
    pub async fn send_to_user(&self, session_id: Uuid, msg: ServerToUserMessage) -> Result<()> {
        let users = self.users.read().await;
        if let Some(session) = users.get(&session_id) {
            session.tx.send(msg).await.map_err(|e| anyhow::anyhow!("Failed to send to user: {}", e))
        } else {
            Err(anyhow::anyhow!("User session not found: {}", session_id))
        }
    }

    /// Broadcast agent status change to all relevant users
    pub async fn broadcast_agent_status(&self, agent_id: Uuid, online: bool) {
        let msg = ServerToUserMessage::AgentStatusChanged { agent_id, online };
        let users = self.users.read().await;
        for session in users.values() {
            // Send to users associated with this agent or super admins (agent_id is None)
            if session.agent_id == Some(agent_id) || session.agent_id.is_none() {
                let _ = session.tx.send(msg.clone()).await;
            }
        }
    }

    /// Update instance status when agent goes offline/online (only HashMap)
    pub async fn update_agent_instances_status(&self, agent_id: Uuid, status: InstanceStatus) {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(&agent_id) {
            for instance in agent.instances.values_mut() {
                instance.status = status.clone();
            }
        }
    }

    /// Restore a suspended instance after agent reconnection
    /// Returns true if instance was restored, false if it needs to be created as new
    pub async fn restore_instance(&self, agent_id: Uuid, instance_id: Uuid) -> bool {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(&agent_id) {
            // Check if instance exists and was suspended
            if let Some(instance) = agent.instances.get_mut(&instance_id) {
                if instance.status == InstanceStatus::Suspended {
                    instance.status = InstanceStatus::Running;
                    tracing::info!("Restored instance {} to Running status", instance_id);
                    return true;
                }
            }
        }
        // Instance not found or not suspended - will be added as new
        false
    }

    /// Cleanup expired suspended instances
    /// This should be called periodically to remove instances that have been
    /// suspended for too long (agent hasn't reconnected)
    pub async fn cleanup_expired_suspended_instances(&self, timeout_secs: u64) {
        let now = chrono::Utc::now();
        let timeout_duration = chrono::Duration::seconds(timeout_secs as i64);

        let mut agents = self.agents.write().await;
        for (agent_id, agent) in agents.iter_mut() {
            let mut to_remove = Vec::new();
            for (instance_id, instance) in agent.instances.iter() {
                if instance.status == InstanceStatus::Suspended {
                    // Check if instance has been suspended for too long
                    if now.signed_duration_since(instance.created_at) > timeout_duration {
                        to_remove.push(*instance_id);
                    }
                }
            }
            for instance_id in to_remove {
                agent.instances.remove(&instance_id);
                tracing::info!(
                    "Cleaned up expired suspended instance {} for agent {}",
                    instance_id,
                    agent_id
                );
            }
        }
    }

    // ========================================================================
    // Admin methods (SuperAdmin only)
    // ========================================================================

    /// Get admin statistics - all agents info and global stats
    pub async fn get_admin_stats(&self) -> (Vec<AgentInfo>, GlobalStats) {
        let agents = self.agents.read().await;
        let users = self.users.read().await;

        let mut agent_infos = Vec::new();
        let mut online_agents = 0;
        let mut total_instances = 0;
        let mut running_instances = 0;

        for (_, connected_agent) in agents.iter() {
            let instance_count = connected_agent.instances.len();
            let running_count = connected_agent.instances.values()
                .filter(|i| i.status == InstanceStatus::Running)
                .count();

            // Count users attached to this agent
            let user_count = users.values()
                .filter(|u| u.agent_id == Some(connected_agent.agent.id))
                .count();

            if connected_agent.agent.status == AgentStatus::Online {
                online_agents += 1;
            }

            total_instances += instance_count;
            running_instances += running_count;

            agent_infos.push(AgentInfo {
                id: connected_agent.agent.id,
                name: connected_agent.agent.name.clone(),
                status: connected_agent.agent.status.clone(),
                connected_at: connected_agent.agent.connected_at,
                instance_count,
                user_count,
            });
        }

        let stats = GlobalStats {
            total_agents: agents.len(),
            online_agents,
            total_instances,
            running_instances,
            total_users: users.len(),
        };

        (agent_infos, stats)
    }

    /// Force disconnect an agent (SuperAdmin only)
    pub async fn force_disconnect_agent(&self, agent_id: Uuid) -> Result<()> {
        // Remove agent from memory
        let mut agents = self.agents.write().await;
        if agents.remove(&agent_id).is_some() {
            // Broadcast agent offline
            drop(agents); // Release lock before broadcasting
            let _ = self.agent_status_tx.send((agent_id, false));
            Ok(())
        } else {
            Err(anyhow::anyhow!("Agent not found: {}", agent_id))
        }
    }

    /// Delete an agent record (SuperAdmin only)
    pub async fn delete_agent(&self, agent_id: Uuid) -> Result<()> {
        // Remove from memory first
        {
            let mut agents = self.agents.write().await;
            agents.remove(&agent_id);
        }

        // Delete from database
        self.agent_repo.delete(agent_id).await?;

        // Broadcast agent offline
        let _ = self.agent_status_tx.send((agent_id, false));

        Ok(())
    }

    /// Force close an instance (SuperAdmin only) - find agent by instance
    pub async fn force_close_instance(&self, instance_id: Uuid) -> Result<Uuid> {
        let agents = self.agents.read().await;

        // Find which agent owns this instance
        for (agent_id, connected_agent) in agents.iter() {
            if connected_agent.instances.contains_key(&instance_id) {
                // Send close command to agent
                let cmd = ServerToAgentMessage::CloseInstance { instance_id };
                connected_agent.tx.send(cmd).await
                    .map_err(|e| anyhow::anyhow!("Failed to send close command: {}", e))?;
                return Ok(*agent_id);
            }
        }

        Err(anyhow::anyhow!("Instance not found: {}", instance_id))
    }

    /// Broadcast message to all SuperAdmin users
    pub async fn broadcast_to_super_admins(&self, msg: ServerToUserMessage) {
        let users = self.users.read().await;
        for session in users.values() {
            // SuperAdmin has agent_id == None and role == SuperAdmin
            if session.agent_id.is_none() {
                let _ = session.tx.send(msg.clone()).await;
            }
        }
    }

    /// Broadcast message to all Admin and SuperAdmin users
    pub async fn broadcast_to_admins(&self, msg: ServerToUserMessage) {
        let users = self.users.read().await;
        for session in users.values() {
            // Send to all users that can create instances (Admin or SuperAdmin)
            let _ = session.tx.send(msg.clone()).await;
        }
    }

    // ========================================================================
    // Tag operations
    // ========================================================================

    /// Get all unique tags
    pub async fn get_all_tags(&self) -> Result<Vec<String>> {
        self.agent_repo.get_all_tags().await
    }

    /// Get tags for a specific agent
    pub async fn get_agent_tags(&self, agent_id: Uuid) -> Result<Vec<String>> {
        self.agent_repo.get_agent_tags(agent_id).await
    }

    /// Add a tag to an agent
    pub async fn add_agent_tag(&self, agent_id: Uuid, tag: &str) -> Result<()> {
        self.agent_repo.add_agent_tag(agent_id, tag).await
    }

    /// Remove a tag from an agent
    pub async fn remove_agent_tag(&self, agent_id: Uuid, tag: &str) -> Result<()> {
        self.agent_repo.remove_agent_tag(agent_id, tag).await
    }

    // ========================================================================
    // Terminal history operations
    // ========================================================================

    /// Save PTY output to terminal history (async, non-blocking)
    pub async fn save_pty_output(&self, instance_id: Uuid, data: &str) {
        if !self.runtime.config.terminal_history.enabled {
            return;
        }

        let byte_size = data.len() as i32;
        let buffer_size_kb = self.runtime.config.terminal_history.default_buffer_size_kb as i32;
        let repo = self.agent_repo.clone();
        let data_owned = data.to_string();

        // Spawn non-blocking task to avoid slowing down real-time output
        tokio::spawn(async move {
            if let Err(e) = repo.save_terminal_history(instance_id, &data_owned, byte_size, buffer_size_kb).await {
                tracing::warn!("Failed to save terminal history for instance {}: {}", instance_id, e);
            }
        });
    }

    /// Get terminal history for an instance
    pub async fn get_terminal_history(&self, instance_id: Uuid) -> Result<Vec<common::ServerToUserMessage>> {
        if !self.runtime.config.terminal_history.enabled {
            return Ok(Vec::new());
        }

        let records = self.agent_repo.get_terminal_history(instance_id).await?;

        Ok(records
            .into_iter()
            .map(|r| common::ServerToUserMessage::PtyOutput {
                instance_id,
                data: r.output_data,
            })
            .collect())
    }

    /// Delete terminal history for an instance
    pub async fn delete_terminal_history(&self, instance_id: Uuid) {
        let repo = self.agent_repo.clone();
        tokio::spawn(async move {
            if let Err(e) = repo.delete_terminal_history(instance_id).await {
                tracing::warn!("Failed to delete terminal history for instance {}: {}", instance_id, e);
            }
        });
    }

    /// Cleanup old terminal history records
    pub async fn cleanup_old_terminal_history(&self) -> Result<u64> {
        let retention_days = self.runtime.config.terminal_history.retention_days;
        self.agent_repo.cleanup_old_terminal_history(retention_days).await
    }

    // ========================================================================
    // Audit log operations
    // ========================================================================

    /// Log an audit event (async, non-blocking)
    pub fn log_audit_event(
        &self,
        event_type: &str,
        session_id: Uuid,
        user_role: &str,
        agent_id: Option<Uuid>,
        instance_id: Option<Uuid>,
        target_id: Option<Uuid>,
        client_ip: &str,
        success: bool,
        details: Option<&str>,
    ) {
        if !self.runtime.config.audit_log.enabled {
            return;
        }

        let repo = self.agent_repo.clone();
        let event_type = event_type.to_string();
        let session_id_str = session_id.to_string();
        let user_role = user_role.to_string();
        let agent_id_str = agent_id.map(|id| id.to_string());
        let instance_id_str = instance_id.map(|id| id.to_string());
        let target_id_str = target_id.map(|id| id.to_string());
        let client_ip = client_ip.to_string();
        let details = details.map(|s| s.to_string());

        // Spawn non-blocking task to avoid slowing down request handling
        tokio::spawn(async move {
            if let Err(e) = repo
                .insert_audit_log(
                    &event_type,
                    &session_id_str,
                    &user_role,
                    agent_id_str.as_deref(),
                    instance_id_str.as_deref(),
                    target_id_str.as_deref(),
                    &client_ip,
                    success,
                    details.as_deref(),
                )
                .await
            {
                tracing::warn!("Failed to write audit log: {}", e);
            }
        });
    }

    /// Get audit logs with pagination
    pub async fn get_audit_logs(
        &self,
        event_type: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<common::AuditLogEntry>, u64)> {
        let (records, total) = self
            .agent_repo
            .get_audit_logs(event_type, limit as i64, offset as i64)
            .await?;

        // Convert AuditLogRecord to AuditLogEntry
        let entries: Vec<common::AuditLogEntry> = records
            .into_iter()
            .map(|r| common::AuditLogEntry {
                id: r.id,
                timestamp: r.timestamp,
                event_type: r.event_type,
                user_role: r.user_role,
                agent_id: r.agent_id.and_then(|s| Uuid::parse_str(&s).ok()),
                instance_id: r.instance_id.and_then(|s| Uuid::parse_str(&s).ok()),
                target_id: r.target_id.and_then(|s| Uuid::parse_str(&s).ok()),
                client_ip: r.client_ip,
                success: r.success != 0,
                details: r.details,
            })
            .collect();

        Ok((entries, total))
    }

    /// Cleanup old audit logs
    pub async fn cleanup_old_audit_logs(&self) -> Result<u64> {
        let retention_days = self.runtime.config.audit_log.retention_days;
        self.agent_repo.cleanup_old_audit_logs(retention_days).await
    }
}
