//! Database module for agent persistence

mod schema;
mod repository;

pub use repository::*;

use anyhow::Result;
use sqlx::{any::AnyPoolOptions, AnyPool};

use crate::config::ServerRuntime;

/// Initialize database connection pool
pub async fn init_database(runtime: &ServerRuntime) -> Result<AnyPool> {
    // Install default drivers for sqlx::any
    sqlx::any::install_default_drivers();

    let db_type = &runtime.config.database.db_type;
    let url = match db_type.as_str() {
        "sqlite" => {
            let path = runtime.config.database.sqlite_path.as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "claude_tunnel.db".to_string());
            format!("sqlite://{}?mode=rwc", path)
        }
        "mysql" => {
            runtime.config.database.mysql_url.clone()
                .unwrap_or_else(|| "mysql://localhost/claude_tunnel".to_string())
        }
        _ => {
            return Err(anyhow::anyhow!("Unsupported database type: {}", db_type));
        }
    };

    tracing::info!("Connecting to database: {} (type: {})", url.split('@').last().unwrap_or(&url), db_type);

    let pool = AnyPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await?;

    // Run migrations
    run_migrations(&pool, db_type).await?;

    Ok(pool)
}

/// Run database migrations
async fn run_migrations(pool: &AnyPool, db_type: &str) -> Result<()> {
    let migrations = match db_type {
        "sqlite" => vec![
            include_str!("../../migrations/sqlite/001_create_agents.sql"),
            include_str!("../../migrations/sqlite/002_agent_tags.sql"),
            include_str!("../../migrations/sqlite/003_terminal_history.sql"),
            include_str!("../../migrations/sqlite/004_audit_logs.sql"),
        ],
        "mysql" => vec![
            include_str!("../../migrations/mysql/001_create_agents.sql"),
            include_str!("../../migrations/mysql/002_agent_tags.sql"),
            include_str!("../../migrations/mysql/003_terminal_history.sql"),
            include_str!("../../migrations/mysql/004_audit_logs.sql"),
        ],
        _ => return Err(anyhow::anyhow!("Unsupported database type: {}", db_type)),
    };

    for migration_sql in migrations {
        // Execute migration statements one by one
        for statement in migration_sql.split(';').filter(|s| !s.trim().is_empty()) {
            sqlx::query(statement)
                .execute(pool)
                .await
                .map_err(|e| {
                    // Ignore "already exists" errors for CREATE INDEX
                    if e.to_string().contains("already exists") || e.to_string().contains("Duplicate key name") {
                        tracing::debug!("Skipping existing object: {}", e);
                        return anyhow::anyhow!("skip");
                    }
                    anyhow::anyhow!("Migration failed: {}", e)
                })
                .ok();
        }
    }

    tracing::info!("Database migrations completed");
    Ok(())
}
