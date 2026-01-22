//! Database repository for CRUD operations

use anyhow::Result;
use chrono::Utc;
use sqlx::AnyPool;
use uuid::Uuid;

use super::schema::{AgentRecord, AuditLogRecord, TerminalHistoryRecord, TerminalHistoryMetaRecord};
use crate::auth::hash_token;

/// Repository for agent database operations
#[derive(Clone)]
pub struct AgentRepository {
    pool: AnyPool,
}

impl AgentRepository {
    /// Create a new repository
    pub fn new(pool: AnyPool) -> Self {
        Self { pool }
    }

    /// Create or update an agent in the database
    pub async fn upsert_agent(
        &self,
        id: Uuid,
        name: &str,
        admin_token: &str,
        share_token: &str,
    ) -> Result<()> {
        let id_str = id.to_string();
        let admin_hash = hash_token(admin_token);
        let share_hash = hash_token(share_token);
        let now = Utc::now().to_rfc3339();

        // Try to insert, if exists update
        sqlx::query(
            r#"
            INSERT INTO agents (id, name, admin_token_hash, share_token_hash, created_at, last_connected_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                admin_token_hash = excluded.admin_token_hash,
                share_token_hash = excluded.share_token_hash,
                last_connected_at = excluded.last_connected_at
            "#,
        )
        .bind(&id_str)
        .bind(name)
        .bind(&admin_hash)
        .bind(&share_hash)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update last connected timestamp
    #[allow(dead_code)]
    pub async fn update_last_connected(&self, id: Uuid) -> Result<()> {
        let id_str = id.to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query("UPDATE agents SET last_connected_at = ? WHERE id = ?")
            .bind(&now)
            .bind(&id_str)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Find agent by admin token hash
    pub async fn find_by_admin_token(&self, token: &str) -> Result<Option<AgentRecord>> {
        let hash = hash_token(token);

        let record = sqlx::query_as::<_, AgentRecord>(
            "SELECT id, name, admin_token_hash, share_token_hash, created_at, last_connected_at FROM agents WHERE admin_token_hash = ?"
        )
        .bind(&hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Find agent by share token hash
    pub async fn find_by_share_token(&self, token: &str) -> Result<Option<AgentRecord>> {
        let hash = hash_token(token);

        let record = sqlx::query_as::<_, AgentRecord>(
            "SELECT id, name, admin_token_hash, share_token_hash, created_at, last_connected_at FROM agents WHERE share_token_hash = ?"
        )
        .bind(&hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Find agent by ID
    #[allow(dead_code)]
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<AgentRecord>> {
        let id_str = id.to_string();

        let record = sqlx::query_as::<_, AgentRecord>(
            "SELECT id, name, admin_token_hash, share_token_hash, created_at, last_connected_at FROM agents WHERE id = ?"
        )
        .bind(&id_str)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get all agents
    #[allow(dead_code)]
    pub async fn find_all(&self) -> Result<Vec<AgentRecord>> {
        let records = sqlx::query_as::<_, AgentRecord>(
            "SELECT id, name, admin_token_hash, share_token_hash, created_at, last_connected_at FROM agents ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Delete an agent
    #[allow(dead_code)]
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let id_str = id.to_string();

        let result = sqlx::query("DELETE FROM agents WHERE id = ?")
            .bind(&id_str)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // Tag operations
    // ========================================================================

    /// Get all tags for an agent
    pub async fn get_agent_tags(&self, agent_id: Uuid) -> Result<Vec<String>> {
        let id_str = agent_id.to_string();

        let tags: Vec<(String,)> = sqlx::query_as(
            "SELECT tag FROM agent_tags WHERE agent_id = ? ORDER BY tag"
        )
        .bind(&id_str)
        .fetch_all(&self.pool)
        .await?;

        Ok(tags.into_iter().map(|(t,)| t).collect())
    }

    /// Add a tag to an agent
    pub async fn add_agent_tag(&self, agent_id: Uuid, tag: &str) -> Result<()> {
        let id_str = agent_id.to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO agent_tags (agent_id, tag, created_at)
            VALUES (?, ?, ?)
            ON CONFLICT(agent_id, tag) DO NOTHING
            "#
        )
        .bind(&id_str)
        .bind(tag)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Remove a tag from an agent
    pub async fn remove_agent_tag(&self, agent_id: Uuid, tag: &str) -> Result<()> {
        let id_str = agent_id.to_string();

        sqlx::query("DELETE FROM agent_tags WHERE agent_id = ? AND tag = ?")
            .bind(&id_str)
            .bind(tag)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get all unique tags across all agents
    pub async fn get_all_tags(&self) -> Result<Vec<String>> {
        let tags: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT tag FROM agent_tags ORDER BY tag"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(tags.into_iter().map(|(t,)| t).collect())
    }

    // ========================================================================
    // Terminal history operations
    // ========================================================================

    /// Initialize terminal history metadata for an instance
    pub async fn init_terminal_history_meta(
        &self,
        instance_id: Uuid,
        buffer_size_kb: i32,
    ) -> Result<()> {
        let id_str = instance_id.to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO terminal_history_meta (instance_id, total_bytes, next_sequence, buffer_size_kb, created_at, updated_at)
            VALUES (?, 0, 0, ?, ?, ?)
            ON CONFLICT(instance_id) DO UPDATE SET
                buffer_size_kb = excluded.buffer_size_kb,
                updated_at = excluded.updated_at
            "#
        )
        .bind(&id_str)
        .bind(buffer_size_kb)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get terminal history metadata for an instance
    pub async fn get_terminal_history_meta(
        &self,
        instance_id: Uuid,
    ) -> Result<Option<TerminalHistoryMetaRecord>> {
        let id_str = instance_id.to_string();

        let record = sqlx::query_as::<_, TerminalHistoryMetaRecord>(
            "SELECT instance_id, total_bytes, next_sequence, buffer_size_kb FROM terminal_history_meta WHERE instance_id = ?"
        )
        .bind(&id_str)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Save terminal history output
    /// Returns the new total bytes stored for this instance
    pub async fn save_terminal_history(
        &self,
        instance_id: Uuid,
        output_data: &str,
        byte_size: i32,
        buffer_size_kb: i32,
    ) -> Result<i64> {
        let id_str = instance_id.to_string();
        let now = Utc::now().to_rfc3339();
        let buffer_limit = (buffer_size_kb as i64) * 1024;

        // Get or create metadata
        let meta = self.get_terminal_history_meta(instance_id).await?;
        let (next_seq, mut total_bytes) = match meta {
            Some(m) => (m.next_sequence, m.total_bytes),
            None => {
                self.init_terminal_history_meta(instance_id, buffer_size_kb).await?;
                (0, 0)
            }
        };

        // Insert new history record
        sqlx::query(
            r#"
            INSERT INTO terminal_history (instance_id, sequence_number, output_data, byte_size, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#
        )
        .bind(&id_str)
        .bind(next_seq)
        .bind(output_data)
        .bind(byte_size)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        total_bytes += byte_size as i64;

        // If over limit, delete oldest records to stay within 90% of buffer
        let target_size = (buffer_limit as f64 * 0.9) as i64;
        if total_bytes > buffer_limit {
            total_bytes = self.trim_terminal_history(instance_id, target_size).await?;
        }

        // Update metadata
        sqlx::query(
            r#"
            UPDATE terminal_history_meta
            SET next_sequence = ?, total_bytes = ?, updated_at = ?
            WHERE instance_id = ?
            "#
        )
        .bind(next_seq + 1)
        .bind(total_bytes)
        .bind(&now)
        .bind(&id_str)
        .execute(&self.pool)
        .await?;

        Ok(total_bytes)
    }

    /// Trim terminal history to target size, returns new total bytes
    async fn trim_terminal_history(
        &self,
        instance_id: Uuid,
        target_size: i64,
    ) -> Result<i64> {
        let id_str = instance_id.to_string();

        // Get total bytes
        let total: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(byte_size), 0) FROM terminal_history WHERE instance_id = ?"
        )
        .bind(&id_str)
        .fetch_one(&self.pool)
        .await?;

        let mut current_total = total.0;

        // Delete oldest records until we're under target
        while current_total > target_size {
            // Find the oldest record
            let oldest: Option<(i64, i32)> = sqlx::query_as(
                "SELECT id, byte_size FROM terminal_history WHERE instance_id = ? ORDER BY sequence_number ASC LIMIT 1"
            )
            .bind(&id_str)
            .fetch_optional(&self.pool)
            .await?;

            match oldest {
                Some((id, size)) => {
                    sqlx::query("DELETE FROM terminal_history WHERE id = ?")
                        .bind(id)
                        .execute(&self.pool)
                        .await?;
                    current_total -= size as i64;
                }
                None => break,
            }
        }

        Ok(current_total)
    }

    /// Get terminal history for an instance (ordered by sequence)
    pub async fn get_terminal_history(
        &self,
        instance_id: Uuid,
    ) -> Result<Vec<TerminalHistoryRecord>> {
        let id_str = instance_id.to_string();

        let records = sqlx::query_as::<_, TerminalHistoryRecord>(
            "SELECT id, instance_id, sequence_number, output_data, byte_size, created_at FROM terminal_history WHERE instance_id = ? ORDER BY sequence_number ASC"
        )
        .bind(&id_str)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Delete all terminal history for an instance
    pub async fn delete_terminal_history(&self, instance_id: Uuid) -> Result<()> {
        let id_str = instance_id.to_string();

        sqlx::query("DELETE FROM terminal_history WHERE instance_id = ?")
            .bind(&id_str)
            .execute(&self.pool)
            .await?;

        sqlx::query("DELETE FROM terminal_history_meta WHERE instance_id = ?")
            .bind(&id_str)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Clean up old terminal history records (older than retention_days)
    pub async fn cleanup_old_terminal_history(&self, retention_days: u32) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
        let cutoff_str = cutoff.to_rfc3339();

        let result = sqlx::query("DELETE FROM terminal_history WHERE created_at < ?")
            .bind(&cutoff_str)
            .execute(&self.pool)
            .await?;

        let deleted = result.rows_affected();

        // Also clean up orphaned metadata
        sqlx::query(
            r#"
            DELETE FROM terminal_history_meta
            WHERE instance_id NOT IN (SELECT DISTINCT instance_id FROM terminal_history)
            "#
        )
        .execute(&self.pool)
        .await?;

        Ok(deleted)
    }

    // ========================================================================
    // Audit log operations
    // ========================================================================

    /// Insert an audit log entry
    pub async fn insert_audit_log(
        &self,
        event_type: &str,
        session_id: &str,
        user_role: &str,
        agent_id: Option<&str>,
        instance_id: Option<&str>,
        target_id: Option<&str>,
        client_ip: &str,
        success: bool,
        details: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let success_int = if success { 1 } else { 0 };

        sqlx::query(
            r#"
            INSERT INTO audit_logs (timestamp, event_type, session_id, user_role, agent_id, instance_id, target_id, client_ip, success, details)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&now)
        .bind(event_type)
        .bind(session_id)
        .bind(user_role)
        .bind(agent_id)
        .bind(instance_id)
        .bind(target_id)
        .bind(client_ip)
        .bind(success_int)
        .bind(details)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get audit logs with optional filters and pagination
    pub async fn get_audit_logs(
        &self,
        event_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<AuditLogRecord>, u64)> {
        // Build WHERE clause
        let where_clause = if event_type.is_some() {
            "WHERE event_type = ?"
        } else {
            "WHERE 1=1"
        };

        // Get total count
        let count_query = format!("SELECT COUNT(*) as count FROM audit_logs {}", where_clause);
        let total: (i64,) = if let Some(et) = event_type {
            sqlx::query_as(&count_query)
                .bind(et)
                .fetch_one(&self.pool)
                .await?
        } else {
            sqlx::query_as(&count_query)
                .fetch_one(&self.pool)
                .await?
        };

        // Get records with pagination
        let query = format!(
            "SELECT id, timestamp, event_type, session_id, user_role, agent_id, instance_id, target_id, client_ip, success, details FROM audit_logs {} ORDER BY timestamp DESC LIMIT ? OFFSET ?",
            where_clause
        );

        let records = if let Some(et) = event_type {
            sqlx::query_as::<_, AuditLogRecord>(&query)
                .bind(et)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query_as::<_, AuditLogRecord>(&query)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
        };

        Ok((records, total.0 as u64))
    }

    /// Clean up old audit logs (older than retention_days)
    pub async fn cleanup_old_audit_logs(&self, retention_days: u32) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
        let cutoff_str = cutoff.to_rfc3339();

        let result = sqlx::query("DELETE FROM audit_logs WHERE timestamp < ?")
            .bind(&cutoff_str)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}
