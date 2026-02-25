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

mod auth;
mod cli;
mod config;
mod db;
mod logging;
mod rate_limit;
mod routes;
mod state;
mod static_files;
mod ws_agent;
mod ws_user;

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
use crate::db::{init_database, AgentRepository};
use crate::logging::init_logging;
use crate::rate_limit::{init_redis, RateLimiter};
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Load configuration first (needed for logging config)
    let runtime = ServerRuntime::from_args(&args)?;

    // Initialize logging with file rotation
    let _log_guard = init_logging(&runtime.config.logging);

    info!("Claude Tunnel Server starting...");
    info!(
        "Server: {}:{}",
        runtime.config.server.host, runtime.config.server.port
    );
    info!("Database: {}", runtime.config.database.db_type);

    // Initialize database
    let db_pool = init_database(&runtime).await?;
    let agent_repo = AgentRepository::new(db_pool, runtime.config.database.db_type.clone());

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
                cleanup_state
                    .cleanup_expired_suspended_instances(1800)
                    .await; // 30 minutes timeout
            }
        });
        info!("Suspended instance cleanup task started (30 min timeout)");
    }

    // Start PTY buffer flush task (every 2 seconds)
    if runtime.config.terminal_history.enabled {
        let flush_state = Arc::clone(&state);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
            loop {
                interval.tick().await;
                flush_state.flush_all_pty_buffers().await;
            }
        });
        info!("PTY buffer flush task started (every 2s)");
    }

    // Build CORS layer based on configuration
    let cors = if runtime.config.server.allowed_origins.is_empty() {
        // No origins configured: no Access-Control-Allow-Origin header,
        // browsers will block cross-origin requests
        CorsLayer::new().allow_methods(Any).allow_headers(Any)
    } else {
        let origins: Vec<_> = runtime
            .config
            .server
            .allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        info!(
            "CORS allowed origins: {:?}",
            runtime.config.server.allowed_origins
        );
        CorsLayer::new()
            .allow_origin(tower_http::cors::AllowOrigin::list(origins))
            .allow_methods(Any)
            .allow_headers(Any)
    };

    // Build router
    let shutdown_state = Arc::clone(&state);
    let app = Router::new()
        .merge(routes::create_routes())
        .layer(cors)
        .with_state(state);

    // Start server
    let addr: SocketAddr = format!(
        "{}:{}",
        runtime.config.server.host, runtime.config.server.port
    )
    .parse()?;

    info!("Listening on {}", addr);

    let listener = TcpListener::bind(addr).await?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal(shutdown_state))
    .await?;

    info!("Server shut down gracefully");

    Ok(())
}

/// Wait for shutdown signal and notify all connections
async fn shutdown_signal(state: Arc<AppState>) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received Ctrl+C, initiating graceful shutdown..."),
        _ = terminate => info!("Received SIGTERM, initiating graceful shutdown..."),
    }

    // Notify all connected agents
    {
        let agents = state.agents.read().await;
        for (agent_id, agent) in agents.iter() {
            let msg = common::ServerToAgentMessage::Error {
                message: "Server is shutting down".to_string(),
            };
            if agent.tx.send(msg).await.is_err() {
                warn!("Failed to notify agent {} of shutdown", agent_id);
            }
        }
        info!("Notified {} agents of shutdown", agents.len());
    }

    // Notify all connected users
    {
        let users = state.users.read().await;
        for (session_id, session) in users.iter() {
            let msg = common::ServerToUserMessage::Error {
                message: "Server is shutting down".to_string(),
            };
            if session.tx.send(msg).await.is_err() {
                warn!("Failed to notify user {} of shutdown", session_id);
            }
        }
        info!("Notified {} users of shutdown", users.len());
    }

    // Give connections time to close gracefully
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
}
