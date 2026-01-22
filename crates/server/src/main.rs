//! Claude Tunnel Server - Remote server for Claude Code tunnel
//!
//! This binary handles:
//! - WebSocket connections from agents
//! - WebSocket connections from web users
//! - Token-based authentication
//! - Message routing between users and agents

// Use mimalloc as global allocator for musl builds (better performance than musl's default allocator)
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod cli;
mod config;
mod state;
mod auth;
mod ws_agent;
mod ws_user;
mod routes;
mod db;
mod rate_limit;
mod logging;
mod static_files;

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use clap::Parser;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

use crate::cli::Args;
use crate::config::ServerRuntime;
use crate::state::AppState;
use crate::db::{init_database, AgentRepository};
use crate::rate_limit::{init_redis, RateLimiter};
use crate::logging::init_logging;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Load configuration first (needed for logging config)
    let runtime = ServerRuntime::from_args(&args)?;

    // Initialize logging with file rotation
    let _log_guard = init_logging(&runtime.config.logging);

    info!("Claude Tunnel Server starting...");
    info!("Server: {}:{}", runtime.config.server.host, runtime.config.server.port);
    info!("Database: {}", runtime.config.database.db_type);

    // Initialize database
    let db_pool = init_database(&runtime).await?;
    let agent_repo = AgentRepository::new(db_pool);

    // Initialize Redis (optional - only if redis_url is configured)
    let rate_limiter = match &runtime.config.database.redis_url {
        Some(redis_url) => match init_redis(redis_url).await {
            Ok(pool) => {
                let limit = runtime.config.security.rate_limit_per_minute;
                Some(RateLimiter::new(pool, limit))
            }
            Err(e) => {
                warn!("Redis not available, rate limiting disabled: {}", e);
                None
            }
        },
        None => {
            info!("Redis not configured, rate limiting disabled");
            None
        }
    };

    // Create application state
    let state = Arc::new(AppState::new(runtime.clone(), agent_repo, rate_limiter).await?);

    // Start background cleanup task for terminal history
    if runtime.config.terminal_history.enabled {
        let cleanup_state = Arc::clone(&state);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600)); // Every hour
            loop {
                interval.tick().await;
                match cleanup_state.cleanup_old_terminal_history().await {
                    Ok(deleted) if deleted > 0 => {
                        info!("Cleaned up {} old terminal history records", deleted);
                    }
                    Ok(_) => {}
                    Err(e) => {
                        warn!("Failed to cleanup terminal history: {}", e);
                    }
                }
            }
        });
        info!("Terminal history cleanup task started (runs hourly)");
    }

    // Start background cleanup task for audit logs
    if runtime.config.audit_log.enabled {
        let cleanup_state = Arc::clone(&state);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600)); // Every hour
            loop {
                interval.tick().await;
                match cleanup_state.cleanup_old_audit_logs().await {
                    Ok(deleted) if deleted > 0 => {
                        info!("Cleaned up {} old audit log records", deleted);
                    }
                    Ok(_) => {}
                    Err(e) => {
                        warn!("Failed to cleanup audit logs: {}", e);
                    }
                }
            }
        });
        info!("Audit log cleanup task started (runs hourly)");
    }

    // Start background cleanup task for suspended instances (30 minutes timeout)
    {
        let cleanup_state = Arc::clone(&state);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60)); // Check every minute
            loop {
                interval.tick().await;
                cleanup_state.cleanup_expired_suspended_instances(1800).await; // 30 minutes timeout
            }
        });
        info!("Suspended instance cleanup task started (30 min timeout)");
    }

    // Build router
    let app = Router::new()
        .merge(routes::create_routes())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    // Start server
    let addr: SocketAddr = format!(
        "{}:{}",
        runtime.config.server.host, runtime.config.server.port
    )
    .parse()?;

    info!("Listening on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;

    Ok(())
}
