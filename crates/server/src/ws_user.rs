//! WebSocket handler for user connections

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use common::{Role, ServerToAgentMessage, ServerToUserMessage, UserMessage};

use crate::state::AppState;

/// Handle a user WebSocket connection
pub async fn handle_user_connection(socket: WebSocket, state: Arc<AppState>, client_ip: String) {
    let (mut ws_sink, mut ws_stream) = socket.split();

    let session_id = Uuid::new_v4();

    // Wait for authentication
    let auth_result = match wait_for_auth(&mut ws_stream, &state, &client_ip).await {
        Some((role, agent_id, agent_name)) => {
            // Create channel for sending messages to user
            let (tx, mut rx) = mpsc::channel::<ServerToUserMessage>(256);

            // Register user session
            state.register_user(session_id, role, agent_id, tx).await;

            info!(
                "User authenticated: session={}, role={:?}, agent={:?}",
                session_id, role, agent_id
            );

            // Log successful authentication
            state.log_audit_event(
                "auth_success",
                session_id,
                &format!("{:?}", role),
                agent_id,
                None,
                None,
                &client_ip,
                true,
                None,
            );

            // Send auth success
            let auth_msg = ServerToUserMessage::AuthResult {
                success: true,
                role: Some(role),
                agent_name,
                agent_id,
                error: None,
            };

            if let Ok(json) = auth_msg.to_json() {
                if ws_sink.send(Message::Text(json)).await.is_err() {
                    state.unregister_user(session_id).await;
                    return;
                }
            }

            // Send initial instance list
            if let Some(aid) = agent_id {
                let instances = state.get_instances(aid).await;
                let list_msg = ServerToUserMessage::InstanceList { instances };
                if let Ok(json) = list_msg.to_json() {
                    let _ = ws_sink.send(Message::Text(json)).await;
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

            (role, agent_id)
        }
        None => {
            // Log failed authentication
            state.log_audit_event(
                "auth_failure",
                session_id,
                "unknown",
                None,
                None,
                None,
                &client_ip,
                false,
                None,
            );

            // Send auth failure
            let auth_msg = ServerToUserMessage::AuthResult {
                success: false,
                role: None,
                agent_name: None,
                agent_id: None,
                error: Some("Authentication failed".to_string()),
            };
            if let Ok(json) = auth_msg.to_json() {
                let _ = ws_sink.send(Message::Text(json)).await;
            }
            return;
        }
    };

    let (role, agent_id) = auth_result;

    // Handle incoming messages
    while let Some(msg) = ws_stream.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Err(e) =
                    handle_user_message(&text, session_id, role, agent_id, &state, &client_ip).await
                {
                    error!("Error handling user message: {}", e);

                    // Send error to user via their channel
                    let error_msg = ServerToUserMessage::Error {
                        message: e.to_string(),
                    };
                    let _ = state.send_to_user(session_id, error_msg).await;
                }
            }
            Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {
                // Handled automatically
            }
            Ok(Message::Close(_)) => {
                info!("User {} disconnected", session_id);
                break;
            }
            Ok(_) => {}
            Err(e) => {
                warn!("WebSocket error from user {}: {}", session_id, e);
                break;
            }
        }
    }

    // Unregister user
    state.unregister_user(session_id).await;
    info!("User session ended: {}", session_id);
}

/// Wait for authentication message
async fn wait_for_auth(
    ws_stream: &mut futures_util::stream::SplitStream<WebSocket>,
    state: &Arc<AppState>,
    client_ip: &str,
) -> Option<(Role, Option<Uuid>, Option<String>)> {
    // Set a timeout for authentication
    let timeout = tokio::time::Duration::from_secs(30);

    tokio::select! {
        result = async {
            while let Some(msg) = ws_stream.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(UserMessage::Auth { token }) = UserMessage::from_json(&text) {
                            // Check rate limit before authentication
                            if let Some(ref limiter) = state.rate_limiter {
                                match limiter.check_limit(client_ip).await {
                                    Ok(allowed) => {
                                        if !allowed {
                                            warn!("Rate limit exceeded for IP: {}", client_ip);
                                            return None;
                                        }
                                    }
                                    Err(e) => {
                                        // Log error but don't block auth if Redis fails
                                        warn!("Rate limit check failed: {}", e);
                                    }
                                }
                            }

                            if let Some((role, agent_id)) = state.authenticate(&token).await {
                                let agent_name = if let Some(aid) = agent_id {
                                    state.get_agent(aid).await.map(|a| a.name)
                                } else {
                                    None
                                };
                                return Some((role, agent_id, agent_name));
                            }
                        }
                        return None;
                    }
                    Ok(Message::Close(_)) | Err(_) => return None,
                    _ => {}
                }
            }
            None
        } => result,
        _ = tokio::time::sleep(timeout) => {
            warn!("Authentication timeout");
            None
        }
    }
}

/// Handle a message from a user
async fn handle_user_message(
    text: &str,
    session_id: Uuid,
    role: Role,
    agent_id: Option<Uuid>,
    state: &Arc<AppState>,
    client_ip: &str,
) -> anyhow::Result<()> {
    let msg: UserMessage = UserMessage::from_json(text)?;
    let role_str = format!("{:?}", role);

    match msg {
        UserMessage::Auth { .. } => {
            // Already authenticated, ignore
            debug!("Ignoring duplicate auth from user {}", session_id);
        }
        UserMessage::CreateInstance { cwd } => {
            if !role.can_create_instance() {
                return Err(anyhow::anyhow!("Permission denied: cannot create instance"));
            }

            // Use effective agent ID (supports SuperAdmin working agent)
            let effective_agent_id = state.get_effective_agent_id(session_id).await
                .ok_or_else(|| anyhow::anyhow!("No agent associated with session. SuperAdmin must select a working agent first."))?;

            let instance_id = Uuid::new_v4();
            info!("User {} requesting instance creation in {} on agent {}", session_id, cwd, effective_agent_id);

            // Log create instance event
            state.log_audit_event(
                "create_instance",
                session_id,
                &role_str,
                Some(effective_agent_id),
                Some(instance_id),
                None,
                client_ip,
                true,
                Some(&format!("cwd: {}", cwd)),
            );

            // Send create command to agent
            let cmd = ServerToAgentMessage::CreateInstance {
                instance_id,
                cwd,
            };
            state.send_to_agent(effective_agent_id, cmd).await?;
        }
        UserMessage::CloseInstance { instance_id } => {
            if !role.can_close_instance() {
                return Err(anyhow::anyhow!("Permission denied: cannot close instance"));
            }

            // Use effective agent ID (supports SuperAdmin working agent)
            let effective_agent_id = state.get_effective_agent_id(session_id).await
                .ok_or_else(|| anyhow::anyhow!("No agent associated with session. SuperAdmin must select a working agent first."))?;

            info!("User {} requesting to close instance {} on agent {}", session_id, instance_id, effective_agent_id);

            // Log close instance event
            state.log_audit_event(
                "close_instance",
                session_id,
                &role_str,
                Some(effective_agent_id),
                Some(instance_id),
                None,
                client_ip,
                true,
                None,
            );

            // Send close command to agent
            let cmd = ServerToAgentMessage::CloseInstance { instance_id };
            state.send_to_agent(effective_agent_id, cmd).await?;
        }
        UserMessage::Attach { instance_id } => {
            info!("User {} attaching to instance {}", session_id, instance_id);
            state.attach_user_to_instance(session_id, instance_id).await;

            // Log attach event
            state.log_audit_event(
                "attach",
                session_id,
                &role_str,
                agent_id,
                Some(instance_id),
                None,
                client_ip,
                true,
                None,
            );

            // Send terminal history (if enabled and available)
            if let Ok(history_msgs) = state.get_terminal_history(instance_id).await {
                if !history_msgs.is_empty() {
                    debug!("Sending {} history messages to user {} for instance {}",
                           history_msgs.len(), session_id, instance_id);
                    for msg in history_msgs {
                        let _ = state.send_to_user(session_id, msg).await;
                    }
                }
            }

            // Notify instance of user count change
            let user_count = state.get_instance_user_count(instance_id).await;
            let msg = ServerToUserMessage::UserJoined {
                instance_id,
                user_count,
            };
            state.broadcast_to_instance(instance_id, msg).await;
        }
        UserMessage::Detach { instance_id } => {
            info!("User {} detaching from instance {}", session_id, instance_id);
            state.detach_user_from_instance(session_id, instance_id).await;

            // Log detach event
            state.log_audit_event(
                "detach",
                session_id,
                &role_str,
                agent_id,
                Some(instance_id),
                None,
                client_ip,
                true,
                None,
            );

            // Notify instance of user count change
            let user_count = state.get_instance_user_count(instance_id).await;
            let msg = ServerToUserMessage::UserLeft {
                instance_id,
                user_count,
            };
            state.broadcast_to_instance(instance_id, msg).await;
        }
        UserMessage::PtyInput { instance_id, data } => {
            // Find the agent for this instance and forward
            // Use effective agent ID (supports SuperAdmin working agent)
            if let Some(effective_agent_id) = state.get_effective_agent_id(session_id).await {
                let cmd = ServerToAgentMessage::PtyInput { instance_id, data };
                state.send_to_agent(effective_agent_id, cmd).await?;
            }
        }
        UserMessage::Resize { instance_id, size } => {
            // Use effective agent ID (supports SuperAdmin working agent)
            if let Some(effective_agent_id) = state.get_effective_agent_id(session_id).await {
                let cmd = ServerToAgentMessage::Resize { instance_id, size };
                state.send_to_agent(effective_agent_id, cmd).await?;
            }
        }
        UserMessage::ListInstances => {
            // Use effective agent ID (supports SuperAdmin working agent)
            if let Some(effective_agent_id) = state.get_effective_agent_id(session_id).await {
                let instances = state.get_instances(effective_agent_id).await;
                debug!("User {} requested instance list: {} instances", session_id, instances.len());

                // Send instance list back to user
                let msg = ServerToUserMessage::InstanceList { instances };
                state.send_to_user(session_id, msg).await?;
            } else {
                // SuperAdmin without working agent - send empty list
                debug!("User {} requested instance list but no agent selected", session_id);
                let msg = ServerToUserMessage::InstanceList { instances: vec![] };
                state.send_to_user(session_id, msg).await?;
            }
        }
        UserMessage::Heartbeat => {
            debug!("Heartbeat from user {}", session_id);
            // Send pong response
            let msg = ServerToUserMessage::Pong;
            state.send_to_user(session_id, msg).await?;
        }
        // ====================================================================
        // Admin commands (SuperAdmin only)
        // ====================================================================
        UserMessage::GetAdminStats => {
            if !role.can_manage_all_agents() {
                return Err(anyhow::anyhow!("Permission denied: not a super admin"));
            }

            let (agents, stats) = state.get_admin_stats().await;
            let msg = ServerToUserMessage::AdminStats { agents, stats };
            state.send_to_user(session_id, msg).await?;
        }
        UserMessage::ForceDisconnectAgent { agent_id: target_agent_id } => {
            if !role.can_manage_all_agents() {
                return Err(anyhow::anyhow!("Permission denied: not a super admin"));
            }

            info!("SuperAdmin {} force disconnecting agent {}", session_id, target_agent_id);
            state.force_disconnect_agent(target_agent_id).await?;

            // Log force disconnect event
            state.log_audit_event(
                "force_disconnect_agent",
                session_id,
                &role_str,
                None,
                None,
                Some(target_agent_id),
                client_ip,
                true,
                None,
            );

            // Notify the requesting admin and broadcast to all super admins
            let msg = ServerToUserMessage::AgentDisconnected { agent_id: target_agent_id };
            state.send_to_user(session_id, msg.clone()).await?;
            state.broadcast_to_super_admins(msg).await;
        }
        UserMessage::ForceCloseInstance { instance_id } => {
            if !role.can_manage_all_agents() {
                return Err(anyhow::anyhow!("Permission denied: not a super admin"));
            }

            info!("SuperAdmin {} force closing instance {}", session_id, instance_id);
            let owning_agent_id = state.force_close_instance(instance_id).await?;

            // Log force close instance event
            state.log_audit_event(
                "force_close_instance",
                session_id,
                &role_str,
                Some(owning_agent_id),
                Some(instance_id),
                None,
                client_ip,
                true,
                None,
            );

            // The instance closed notification will be sent when agent confirms
        }
        UserMessage::DeleteAgent { agent_id: target_agent_id } => {
            if !role.can_manage_all_agents() {
                return Err(anyhow::anyhow!("Permission denied: not a super admin"));
            }

            info!("SuperAdmin {} deleting agent {}", session_id, target_agent_id);
            state.delete_agent(target_agent_id).await?;

            // Log delete agent event
            state.log_audit_event(
                "delete_agent",
                session_id,
                &role_str,
                None,
                None,
                Some(target_agent_id),
                client_ip,
                true,
                None,
            );

            // Notify the requesting admin and broadcast to all super admins
            let msg = ServerToUserMessage::AgentDeleted { agent_id: target_agent_id };
            state.send_to_user(session_id, msg.clone()).await?;
            state.broadcast_to_super_admins(msg).await;
        }
        // ====================================================================
        // Tag commands
        // ====================================================================
        UserMessage::GetAllTags => {
            if !role.can_create_instance() {
                return Err(anyhow::anyhow!("Permission denied: not an admin"));
            }

            let tags = state.get_all_tags().await?;
            let msg = ServerToUserMessage::TagList { tags };
            state.send_to_user(session_id, msg).await?;
        }
        UserMessage::GetAgentTags { agent_id: target_agent_id } => {
            if !role.can_create_instance() {
                return Err(anyhow::anyhow!("Permission denied: not an admin"));
            }

            let tags = state.get_agent_tags(target_agent_id).await?;
            let msg = ServerToUserMessage::AgentTags {
                agent_id: target_agent_id,
                tags,
            };
            state.send_to_user(session_id, msg).await?;
        }
        UserMessage::AddAgentTag { agent_id: target_agent_id, tag } => {
            if !role.can_create_instance() {
                return Err(anyhow::anyhow!("Permission denied: not an admin"));
            }

            info!("Admin {} adding tag '{}' to agent {}", session_id, tag, target_agent_id);
            state.add_agent_tag(target_agent_id, &tag).await?;

            // Log add tag event
            state.log_audit_event(
                "add_agent_tag",
                session_id,
                &role_str,
                None,
                None,
                Some(target_agent_id),
                client_ip,
                true,
                Some(&format!("tag: {}", tag)),
            );

            // Broadcast to all admins
            let msg = ServerToUserMessage::TagAdded {
                agent_id: target_agent_id,
                tag,
            };
            state.broadcast_to_admins(msg).await;
        }
        UserMessage::RemoveAgentTag { agent_id: target_agent_id, tag } => {
            if !role.can_create_instance() {
                return Err(anyhow::anyhow!("Permission denied: not an admin"));
            }

            info!("Admin {} removing tag '{}' from agent {}", session_id, tag, target_agent_id);
            state.remove_agent_tag(target_agent_id, &tag).await?;

            // Log remove tag event
            state.log_audit_event(
                "remove_agent_tag",
                session_id,
                &role_str,
                None,
                None,
                Some(target_agent_id),
                client_ip,
                true,
                Some(&format!("tag: {}", tag)),
            );

            // Broadcast to all admins
            let msg = ServerToUserMessage::TagRemoved {
                agent_id: target_agent_id,
                tag,
            };
            state.broadcast_to_admins(msg).await;
        }
        // ====================================================================
        // Audit log commands (SuperAdmin only)
        // ====================================================================
        UserMessage::GetAuditLogs { limit, offset, event_type } => {
            if !role.can_manage_all_agents() {
                return Err(anyhow::anyhow!("Permission denied: not a super admin"));
            }

            let limit = limit.unwrap_or(100);
            let offset = offset.unwrap_or(0);

            debug!("SuperAdmin {} requesting audit logs (limit={}, offset={}, event_type={:?})",
                   session_id, limit, offset, event_type);

            let (logs, total) = state
                .get_audit_logs(event_type.as_deref(), limit, offset)
                .await?;

            let msg = ServerToUserMessage::AuditLogList { logs, total };
            state.send_to_user(session_id, msg).await?;
        }
        // ====================================================================
        // Working Agent commands (SuperAdmin only)
        // ====================================================================
        UserMessage::SelectWorkingAgent { agent_id: target_agent_id } => {
            if !role.can_manage_all_agents() {
                return Err(anyhow::anyhow!("Permission denied: not a super admin"));
            }

            info!("SuperAdmin {} selecting working agent {}", session_id, target_agent_id);

            // Check if agent exists and is online
            let agent = state.get_agent(target_agent_id).await;
            if let Some(agent_info) = agent {
                // Set the working agent
                state.set_working_agent(session_id, target_agent_id).await;

                // Log the event
                state.log_audit_event(
                    "select_working_agent",
                    session_id,
                    &role_str,
                    Some(target_agent_id),
                    None,
                    None,
                    client_ip,
                    true,
                    Some(&format!("agent_name: {}", agent_info.name)),
                );

                // Send success response with instance list
                let msg = ServerToUserMessage::WorkingAgentSelected {
                    agent_id: target_agent_id,
                    agent_name: agent_info.name,
                    success: true,
                    error: None,
                };
                state.send_to_user(session_id, msg).await?;

                // Send the instance list for this agent
                let instances = state.get_instances(target_agent_id).await;
                let list_msg = ServerToUserMessage::InstanceList { instances };
                state.send_to_user(session_id, list_msg).await?;
            } else {
                // Agent not found or offline
                let msg = ServerToUserMessage::WorkingAgentSelected {
                    agent_id: target_agent_id,
                    agent_name: String::new(),
                    success: false,
                    error: Some("Agent not found or offline".to_string()),
                };
                state.send_to_user(session_id, msg).await?;
            }
        }
        UserMessage::ClearWorkingAgent => {
            if !role.can_manage_all_agents() {
                return Err(anyhow::anyhow!("Permission denied: not a super admin"));
            }

            info!("SuperAdmin {} clearing working agent", session_id);

            state.clear_working_agent(session_id).await;

            // Log the event
            state.log_audit_event(
                "clear_working_agent",
                session_id,
                &role_str,
                None,
                None,
                None,
                client_ip,
                true,
                None,
            );

            let msg = ServerToUserMessage::WorkingAgentCleared;
            state.send_to_user(session_id, msg).await?;
        }
        UserMessage::ListAgentInstances { agent_id: target_agent_id } => {
            if !role.can_manage_all_agents() {
                return Err(anyhow::anyhow!("Permission denied: not a super admin"));
            }

            debug!("SuperAdmin {} requesting instances for agent {}", session_id, target_agent_id);

            let instances = state.get_instances(target_agent_id).await;
            let msg = ServerToUserMessage::InstanceList { instances };
            state.send_to_user(session_id, msg).await?;
        }
    }

    Ok(())
}
