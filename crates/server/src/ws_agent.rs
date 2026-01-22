//! WebSocket handler for agent connections

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use common::{AgentMessage, ExistingInstance, Instance, InstanceStatus, ServerToAgentMessage};

use crate::state::AppState;

/// Handle an agent WebSocket connection
pub async fn handle_agent_connection(socket: WebSocket, state: Arc<AppState>) {
    let (mut ws_sink, mut ws_stream) = socket.split();

    // Wait for registration message
    let (agent_id, agent_name, existing_instances) = match wait_for_registration(&mut ws_stream).await {
        Some((id, name, admin_token, share_token, existing_instances)) => {
            // Create channel for sending messages to agent
            let (tx, mut rx) = mpsc::channel::<ServerToAgentMessage>(256);

            // Register agent (this may be a reconnection)
            state
                .register_agent(id, name.clone(), admin_token, share_token, tx)
                .await;

            info!("Agent registered: {} ({})", name, id);

            // Broadcast agent online status to users
            state.broadcast_agent_status(id, true).await;

            // Send registration confirmation
            let confirm = ServerToAgentMessage::Registered {
                message: "Registration successful".to_string(),
            };
            if let Ok(json) = confirm.to_json() {
                if ws_sink.send(Message::Text(json)).await.is_err() {
                    state.unregister_agent(id).await;
                    return;
                }
            }

            // Spawn task to forward messages from channel to WebSocket
            let mut ws_sink_clone = ws_sink;
            tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    if let Ok(json) = msg.to_json() {
                        if ws_sink_clone.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                }
            });

            (id, name, existing_instances)
        }
        None => {
            warn!("Agent connection closed before registration");
            return;
        }
    };

    // ========================================================================
    // Reconnection Recovery: Restore instances from agent
    // ========================================================================
    if !existing_instances.is_empty() {
        info!(
            "Agent {} reconnecting with {} existing instances",
            agent_id,
            existing_instances.len()
        );

        // Restore instances that were suspended or create new ones
        for existing in existing_instances {
            let instance = Instance {
                id: existing.id,
                agent_id,
                cwd: existing.cwd.clone(),
                status: InstanceStatus::Running,
                created_at: chrono::Utc::now(), // Use current time for recovered instances
                attached_users: 0,
            };

            // Try to restore from suspended state first, or add as new
            let was_suspended = state.restore_instance(agent_id, existing.id).await;
            if was_suspended {
                info!("Restored suspended instance {} for agent {}", existing.id, agent_id);
            } else {
                // Add as new instance (wasn't tracked before)
                state.add_instance(agent_id, instance.clone()).await;
                info!("Added recovered instance {} for agent {}", existing.id, agent_id);
            }

            // Notify users about the instance
            let msg = common::ServerToUserMessage::InstanceCreated { instance };
            broadcast_to_agent_users(&state, agent_id, msg).await;
        }
    }

    // Handle incoming messages
    while let Some(msg) = ws_stream.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Err(e) = handle_agent_message(&text, agent_id, &state).await {
                    error!("Error handling agent message: {}", e);
                }
            }
            Ok(Message::Ping(_data)) => {
                // Ping handled automatically by axum
                debug!("Received ping from agent {}", agent_id);
            }
            Ok(Message::Pong(_)) => {
                debug!("Received pong from agent {}", agent_id);
            }
            Ok(Message::Close(_)) => {
                info!("Agent {} disconnected", agent_id);
                break;
            }
            Ok(_) => {}
            Err(e) => {
                warn!("WebSocket error from agent {}: {}", agent_id, e);
                break;
            }
        }
    }

    // Unregister agent and notify users
    state.update_agent_instances_status(agent_id, InstanceStatus::Suspended).await;
    state.unregister_agent(agent_id).await;
    state.broadcast_agent_status(agent_id, false).await;
    info!("Agent unregistered: {} ({})", agent_name, agent_id);
}

/// Wait for the registration message from an agent
async fn wait_for_registration(
    ws_stream: &mut futures_util::stream::SplitStream<WebSocket>,
) -> Option<(Uuid, String, String, String, Vec<ExistingInstance>)> {
    while let Some(msg) = ws_stream.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(AgentMessage::Register {
                    agent_id,
                    name,
                    admin_token,
                    share_token,
                    existing_instances,
                }) = AgentMessage::from_json(&text)
                {
                    return Some((agent_id, name, admin_token, share_token, existing_instances));
                }
            }
            Ok(Message::Close(_)) | Err(_) => return None,
            _ => {}
        }
    }
    None
}

/// Handle a message from an agent
async fn handle_agent_message(
    text: &str,
    agent_id: Uuid,
    state: &Arc<AppState>,
) -> anyhow::Result<()> {
    let msg: AgentMessage = AgentMessage::from_json(text)?;

    match msg {
        AgentMessage::Register { .. } => {
            // Already registered, ignore duplicate
            debug!("Ignoring duplicate registration from agent {}", agent_id);
        }
        AgentMessage::InstanceCreated { instance_id, cwd } => {
            info!("Agent {} created instance {} in {}", agent_id, instance_id, cwd);

            let instance = Instance {
                id: instance_id,
                agent_id,
                cwd: cwd.clone(),
                status: InstanceStatus::Running,
                created_at: chrono::Utc::now(),
                attached_users: 0,
            };

            state.add_instance(agent_id, instance.clone()).await;

            // Notify all users of this agent
            let msg = common::ServerToUserMessage::InstanceCreated { instance };
            broadcast_to_agent_users(state, agent_id, msg).await;
        }
        AgentMessage::InstanceClosed { instance_id } => {
            info!("Agent {} closed instance {}", agent_id, instance_id);

            state.remove_instance(agent_id, instance_id).await;

            // Clean up terminal history for this instance
            state.delete_terminal_history(instance_id).await;

            // Notify all users
            let msg = common::ServerToUserMessage::InstanceClosed { instance_id };
            broadcast_to_agent_users(state, agent_id, msg).await;
        }
        AgentMessage::PtyOutput { instance_id, data } => {
            // Save to terminal history (async, non-blocking)
            state.save_pty_output(instance_id, &data).await;

            // Forward to all attached users
            let msg = common::ServerToUserMessage::PtyOutput { instance_id, data };
            state.broadcast_to_instance(instance_id, msg).await;
        }
        AgentMessage::Heartbeat => {
            debug!("Heartbeat from agent {}", agent_id);
        }
        AgentMessage::Error { message } => {
            warn!("Error from agent {}: {}", agent_id, message);
        }
    }

    Ok(())
}

/// Broadcast a message to all users of an agent
async fn broadcast_to_agent_users(
    state: &Arc<AppState>,
    agent_id: Uuid,
    msg: common::ServerToUserMessage,
) {
    let users = state.users.read().await;
    for session in users.values() {
        // Send to users associated with this agent or super admins
        if session.agent_id == Some(agent_id) || session.agent_id.is_none() {
            let _ = session.tx.send(msg.clone()).await;
        }
    }
}
