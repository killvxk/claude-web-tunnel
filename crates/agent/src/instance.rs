//! Instance management for Claude Code instances

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, info};
use uuid::Uuid;

use crate::pty::PtyInstance;

/// Information about an existing instance for reconnection sync
#[derive(Debug, Clone)]
pub struct InstanceInfo {
    /// Instance ID
    pub id: Uuid,
    /// Working directory
    pub cwd: String,
}

/// Manages multiple Claude Code instances
pub struct InstanceManager {
    /// Active instances (wrapped in Arc<Mutex> for shared mutable access)
    instances: HashMap<Uuid, Arc<Mutex<PtyInstance>>>,
}

impl InstanceManager {
    /// Create a new instance manager
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
        }
    }

    /// Create a new Claude Code instance
    pub async fn create_instance(
        &mut self,
        id: Uuid,
        cwd: &str,
        output_tx: mpsc::Sender<(Uuid, Vec<u8>)>,
    ) -> Result<()> {
        if self.instances.contains_key(&id) {
            return Err(anyhow!("Instance {} already exists", id));
        }

        // Validate working directory exists
        let path = std::path::Path::new(cwd);
        if !path.exists() {
            return Err(anyhow!("Directory does not exist: {}", cwd));
        }
        if !path.is_dir() {
            return Err(anyhow!("Path is not a directory: {}", cwd));
        }

        let instance = PtyInstance::new(id, cwd, output_tx)?;
        self.instances.insert(id, Arc::new(Mutex::new(instance)));

        info!("Created instance {} in {}", id, cwd);
        Ok(())
    }

    /// Close an instance
    pub async fn close_instance(&mut self, id: Uuid) -> Result<()> {
        if let Some(instance) = self.instances.remove(&id) {
            let inst = instance.lock().await;
            inst.kill().ok();
            info!("Closed instance {}", id);
            Ok(())
        } else {
            Err(anyhow!("Instance {} not found", id))
        }
    }

    /// Write data to an instance's PTY
    pub async fn write_to_instance(&self, id: Uuid, data: &[u8]) -> Result<()> {
        if let Some(instance) = self.instances.get(&id) {
            let mut inst = instance.lock().await;
            inst.write(data)?;
            debug!("Writing {} bytes to instance {}", data.len(), id);
            Ok(())
        } else {
            Err(anyhow!("Instance {} not found", id))
        }
    }

    /// Resize an instance's PTY
    pub async fn resize_instance(&self, id: Uuid, cols: u16, rows: u16) -> Result<()> {
        if let Some(instance) = self.instances.get(&id) {
            let inst = instance.lock().await;
            inst.resize(cols, rows)?;
            debug!("Resized instance {} to {}x{}", id, cols, rows);
            Ok(())
        } else {
            Err(anyhow!("Instance {} not found", id))
        }
    }

    /// Get list of instance IDs
    #[allow(dead_code)]
    pub fn list_instances(&self) -> Vec<Uuid> {
        self.instances.keys().copied().collect()
    }

    /// Check if an instance exists
    #[allow(dead_code)]
    pub fn has_instance(&self, id: Uuid) -> bool {
        self.instances.contains_key(&id)
    }

    /// Get instance count
    #[allow(dead_code)]
    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }

    /// Get information about all existing instances for reconnection sync
    pub async fn get_instance_infos(&self) -> Vec<InstanceInfo> {
        let mut infos = Vec::new();
        for (id, instance) in &self.instances {
            let inst = instance.lock().await;
            // Only include running instances
            if inst.is_running() {
                infos.push(InstanceInfo {
                    id: *id,
                    cwd: inst.cwd.clone(),
                });
            }
        }
        infos
    }

    /// Rebind output channels for all instances after reconnection
    pub async fn rebind_all_channels(&self, new_tx: mpsc::Sender<(Uuid, Vec<u8>)>) {
        for (_id, instance) in &self.instances {
            let inst = instance.lock().await;
            inst.rebind_output_channel(new_tx.clone()).await;
        }
        info!("Rebound output channels for {} instances", self.instances.len());
    }

    /// Mark all instances as disconnected (for buffering output)
    pub async fn set_all_disconnected(&self) {
        for (_id, instance) in &self.instances {
            let inst = instance.lock().await;
            inst.set_connected(false);
        }
        debug!("Marked {} instances as disconnected", self.instances.len());
    }

    /// Get and clear all buffered output from all instances
    /// Returns a map of instance_id -> buffered_data
    pub async fn drain_all_buffers(&self) -> HashMap<Uuid, Vec<u8>> {
        let mut buffers = HashMap::new();
        for (id, instance) in &self.instances {
            let inst = instance.lock().await;
            let data = inst.drain_buffer().await;
            if !data.is_empty() {
                buffers.insert(*id, data);
            }
        }
        if !buffers.is_empty() {
            info!("Drained buffers for {} instances", buffers.len());
        }
        buffers
    }

    /// Remove instances that are no longer running
    /// Returns the number of instances removed
    #[allow(dead_code)]
    pub async fn cleanup_dead_instances(&mut self) -> usize {
        let mut to_remove = Vec::new();
        for (id, instance) in &self.instances {
            let inst = instance.lock().await;
            if !inst.is_running() {
                to_remove.push(*id);
            }
        }
        let count = to_remove.len();
        for id in to_remove {
            self.instances.remove(&id);
            debug!("Removed dead instance {}", id);
        }
        count
    }
}

impl Default for InstanceManager {
    fn default() -> Self {
        Self::new()
    }
}
