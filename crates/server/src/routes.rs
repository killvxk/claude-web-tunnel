//! HTTP routes and WebSocket endpoints

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{ConnectInfo, State, WebSocketUpgrade},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};

use crate::state::AppState;
use crate::static_files::{has_web_assets, static_handler};
use crate::ws_agent::handle_agent_connection;
use crate::ws_user::handle_user_connection;

/// Create all routes for the server
pub fn create_routes() -> Router<Arc<AppState>> {
    let router = Router::new()
        // Health check
        .route("/health", get(health_check))
        // WebSocket endpoints
        .route("/ws/agent", get(ws_agent_handler))
        .route("/ws/user", get(ws_user_handler));

    // Add static file serving for embedded web frontend
    if has_web_assets() {
        router.fallback(static_handler)
    } else {
        // Fallback to simple HTML when web frontend is not embedded
        router.route("/", get(fallback_index_handler))
    }
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    "OK"
}

/// Fallback index page when web frontend is not embedded
async fn fallback_index_handler() -> Html<&'static str> {
    Html(FALLBACK_HTML)
}

/// WebSocket handler for agent connections
async fn ws_agent_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_agent_connection(socket, state))
}

/// WebSocket handler for user connections
async fn ws_user_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let client_ip = addr.ip().to_string();
    ws.on_upgrade(move |socket| handle_user_connection(socket, state, client_ip))
}

/// Simple HTML fallback page (used when web frontend is not built)
const FALLBACK_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Claude Web Tunnel</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #1a1a2e;
            color: #eee;
            min-height: 100vh;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            padding: 20px;
        }
        .container { max-width: 600px; text-align: center; }
        h1 {
            font-size: 2.5rem;
            margin-bottom: 1rem;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }
        p { color: #aaa; margin-bottom: 2rem; line-height: 1.6; }
        .warning {
            background: rgba(255, 193, 7, 0.2);
            color: #ffc107;
            padding: 1rem;
            border-radius: 8px;
            margin-bottom: 1rem;
        }
        code {
            background: rgba(255, 255, 255, 0.1);
            padding: 2px 6px;
            border-radius: 4px;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>Claude Web Tunnel</h1>
        <p>Access your local Claude Code instance remotely through this secure tunnel.</p>
        <div class="warning">
            <strong>Web Frontend Not Available</strong><br>
            The web frontend is not embedded in this build.<br><br>
            To build with web frontend, run:<br>
            <code>cd web && npm run build && cd .. && cargo build</code>
        </div>
        <p>
            WebSocket endpoints are available at:<br>
            <code>/ws/agent</code> - Agent connections<br>
            <code>/ws/user</code> - User connections
        </p>
    </div>
</body>
</html>
"#;
