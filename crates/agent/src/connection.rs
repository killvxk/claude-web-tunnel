//! WebSocket connection management for the tunnel agent

use std::sync::Arc;

use anyhow::{anyhow, Result};
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use common::{AgentMessage, ExistingInstance, ServerToAgentMessage};

use crate::config::AgentRuntime;
use crate::instance::InstanceManager;

/// Tunnel connection handler
pub struct TunnelConnection {
    /// Runtime configuration
    pub runtime: AgentRuntime,
    /// Instance manager
    instances: Arc<tokio::sync::Mutex<InstanceManager>>,
}

impl TunnelConnection {
    /// Create a new tunnel connection
    pub fn new(runtime: AgentRuntime) -> Self {
        Self {
            runtime,
            instances: Arc::new(tokio::sync::Mutex::new(InstanceManager::new())),
        }
    }

    /// Build WebSocket URL from server URL
    /// Converts http:// to ws:// and https:// to wss://
    fn build_ws_url(url: &str) -> Result<String> {
        let trimmed = url.trim_end_matches('/');

        let ws_url = if trimmed.starts_with("https://") {
            format!("wss://{}/ws/agent", &trimmed[8..])
        } else if trimmed.starts_with("http://") {
            format!("ws://{}/ws/agent", &trimmed[7..])
        } else if trimmed.starts_with("wss://") || trimmed.starts_with("ws://") {
            // Already a WebSocket URL
            format!("{}/ws/agent", trimmed)
        } else {
            return Err(anyhow!("Invalid server URL: {}. Must start with http://, https://, ws://, or wss://", url));
        };

        Ok(ws_url)
    }

    /// Run the connection loop
    pub async fn run(&mut self) -> Result<()> {
        let url = &self.runtime.config.server.url;
        let ws_url = Self::build_ws_url(url)?;

        info!("Connecting to server: {}", ws_url);

        // Connect to the WebSocket server
        let (ws_stream, _response) = connect_async(&ws_url)
            .await
            .map_err(|e| anyhow!("Failed to connect: {}", e))?;

        info!("Connected to server");

        let (mut ws_sink, mut ws_stream) = ws_stream.split();

        // Create channel for PTY output
        let (pty_tx, mut pty_rx) = mpsc::channel::<(Uuid, Vec<u8>)>(256);

        // ====================================================================
        // Reconnection Recovery: Rebind existing instances to new channel
        // ====================================================================
        let existing_instances: Vec<ExistingInstance> = {
            let instances = self.instances.lock().await;

            // Clean up dead instances first
            // Note: Can't call cleanup_dead_instances here because we have immutable borrow
            // Will be handled by get_instance_infos which only returns running instances

            // Rebind all existing instances to the new output channel
            instances.rebind_all_channels(pty_tx.clone()).await;

            // Get info about existing instances for registration
            let infos = instances.get_instance_infos().await;
            infos
                .into_iter()
                .map(|info| ExistingInstance {
                    id: info.id,
                    cwd: info.cwd,
                })
                .collect()
        };

        if !existing_instances.is_empty() {
            info!(
                "Reconnecting with {} existing instances",
                existing_instances.len()
            );
        }

        // Send registration message (with existing instances for reconnection)
        let register_msg = AgentMessage::Register {
            agent_id: self.runtime.agent_id,
            name: self.runtime.config.agent.name.clone(),
            admin_token: self.runtime.admin_token.clone(),
            share_token: self.runtime.share_token.clone(),
            existing_instances,
        };

        ws_sink
            .send(Message::Text(register_msg.to_json()?))
            .await
            .map_err(|e| anyhow!("Failed to send register message: {}", e))?;

        info!("Sent registration message");

        // ====================================================================
        // Send buffered output from disconnection period
        // ====================================================================
        {
            let instances = self.instances.lock().await;
            let buffers = instances.drain_all_buffers().await;
            for (instance_id, data) in buffers {
                if !data.is_empty() {
                    let msg = AgentMessage::PtyOutput {
                        instance_id,
                        data: base64::engine::general_purpose::STANDARD.encode(&data),
                    };
                    if let Ok(json) = msg.to_json() {
                        if let Err(e) = ws_sink.send(Message::Text(json)).await {
                            warn!("Failed to send buffered output for instance {}: {}", instance_id, e);
                        } else {
                            debug!("Sent {} bytes of buffered output for instance {}", data.len(), instance_id);
                        }
                    }
                }
            }
        }

        // Clone instances for the message handler (kept for future multi-instance handling)
        let _instances = Arc::clone(&self.instances);

        // Heartbeat interval from configuration
        let heartbeat_secs = self.runtime.config.server.heartbeat_interval;
        let heartbeat_interval = tokio::time::Duration::from_secs(heartbeat_secs);
        let mut heartbeat_timer = tokio::time::interval(heartbeat_interval);
        info!("Heartbeat interval: {} seconds", heartbeat_secs);

        loop {
            tokio::select! {
                // Handle incoming messages from server
                msg = ws_stream.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Err(e) = self.handle_server_message(&text, &mut ws_sink, &pty_tx).await {
                                error!("Error handling message: {}", e);
                            }
                        }
                        Some(Ok(Message::Ping(data))) => {
                            ws_sink.send(Message::Pong(data)).await.ok();
                        }
                        Some(Ok(Message::Pong(_))) => {
                            debug!("Received pong");
                        }
                        Some(Ok(Message::Close(_))) => {
                            info!("Server closed connection");
                            // Mark instances as disconnected before returning
                            let instances = self.instances.lock().await;
                            instances.set_all_disconnected().await;
                            return Ok(());
                        }
                        Some(Ok(_)) => {
                            // Ignore other message types
                        }
                        Some(Err(e)) => {
                            // Mark instances as disconnected before returning
                            let instances = self.instances.lock().await;
                            instances.set_all_disconnected().await;
                            return Err(anyhow!("WebSocket error: {}", e));
                        }
                        None => {
                            // Mark instances as disconnected before returning
                            let instances = self.instances.lock().await;
                            instances.set_all_disconnected().await;
                            return Err(anyhow!("Connection closed unexpectedly"));
                        }
                    }
                }

                // Handle PTY output
                Some((instance_id, data)) = pty_rx.recv() => {
                    let msg = AgentMessage::PtyOutput {
                        instance_id,
                        data: base64::engine::general_purpose::STANDARD.encode(&data),
                    };
                    if let Ok(json) = msg.to_json() {
                        if let Err(e) = ws_sink.send(Message::Text(json)).await {
                            warn!("Failed to send PTY output: {}", e);
                        }
                    }
                }

                // Send heartbeat - failure triggers reconnection
                _ = heartbeat_timer.tick() => {
                    let msg = AgentMessage::Heartbeat;
                    match msg.to_json() {
                        Ok(json) => {
                            if let Err(e) = ws_sink.send(Message::Text(json)).await {
                                // Mark instances as disconnected before returning
                                let instances = self.instances.lock().await;
                                instances.set_all_disconnected().await;
                                return Err(anyhow!("Heartbeat send failed, connection may be dead: {}", e));
                            }
                            debug!("Heartbeat sent");
                        }
                        Err(e) => {
                            return Err(anyhow!("Failed to serialize heartbeat: {}", e));
                        }
                    }
                }
            }
        }
    }

    /// Handle a message from the server
    async fn handle_server_message(
        &mut self,
        text: &str,
        ws_sink: &mut futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            Message,
        >,
        pty_tx: &mpsc::Sender<(Uuid, Vec<u8>)>,
    ) -> Result<()> {
        let msg: ServerToAgentMessage = ServerToAgentMessage::from_json(text)?;

        match msg {
            ServerToAgentMessage::Registered { message } => {
                info!("Registration successful: {}", message);
            }
            ServerToAgentMessage::CreateInstance { instance_id, cwd } => {
                info!("Creating instance {} in {}", instance_id, cwd);

                let mut instances = self.instances.lock().await;
                match instances.create_instance(instance_id, &cwd, pty_tx.clone()).await {
                    Ok(_) => {
                        // Send instance created confirmation
                        let msg = AgentMessage::InstanceCreated {
                            instance_id,
                            cwd: cwd.clone(),
                        };
                        if let Ok(json) = msg.to_json() {
                            ws_sink.send(Message::Text(json)).await.ok();
                        }
                        info!("Instance {} created successfully", instance_id);
                    }
                    Err(e) => {
                        error!("Failed to create instance: {}", e);
                        let msg = AgentMessage::Error {
                            message: format!("Failed to create instance: {}", e),
                        };
                        if let Ok(json) = msg.to_json() {
                            ws_sink.send(Message::Text(json)).await.ok();
                        }
                    }
                }
            }
            ServerToAgentMessage::CloseInstance { instance_id } => {
                info!("Closing instance {}", instance_id);

                let mut instances = self.instances.lock().await;
                if let Err(e) = instances.close_instance(instance_id).await {
                    error!("Failed to close instance: {}", e);
                }

                // Send instance closed confirmation
                let msg = AgentMessage::InstanceClosed { instance_id };
                if let Ok(json) = msg.to_json() {
                    ws_sink.send(Message::Text(json)).await.ok();
                }
            }
            ServerToAgentMessage::PtyInput { instance_id, data } => {
                if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&data) {
                    let instances = self.instances.lock().await;
                    if let Err(e) = instances.write_to_instance(instance_id, &bytes).await {
                        warn!("Failed to write to instance {}: {}", instance_id, e);
                    }
                }
            }
            ServerToAgentMessage::Resize { instance_id, size } => {
                debug!("Resizing instance {} to {}x{}", instance_id, size.cols, size.rows);
                let instances = self.instances.lock().await;
                if let Err(e) = instances.resize_instance(instance_id, size.cols, size.rows).await {
                    warn!("Failed to resize instance {}: {}", instance_id, e);
                }
            }
            ServerToAgentMessage::Ping => {
                // Respond with heartbeat
                let msg = AgentMessage::Heartbeat;
                if let Ok(json) = msg.to_json() {
                    ws_sink.send(Message::Text(json)).await.ok();
                }
            }
            ServerToAgentMessage::Error { message } => {
                error!("Server error: {}", message);
            }
        }

        Ok(())
    }
}
