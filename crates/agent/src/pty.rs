//! PTY management for Claude Code instances

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use portable_pty::{native_pty_system, Child, CommandBuilder, PtySize};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Maximum buffer size (1MB) to prevent memory exhaustion during long disconnects
const MAX_BUFFER_SIZE: usize = 1024 * 1024;

/// PTY wrapper for a Claude Code instance
pub struct PtyInstance {
    /// Instance ID
    pub id: Uuid,
    /// Working directory (for reconnection sync)
    pub cwd: String,
    /// PTY master handle
    master: Box<dyn portable_pty::MasterPty + Send>,
    /// Writer to send data to PTY
    writer: Box<dyn Write + Send>,
    /// Handle to the reader task
    reader_handle: Option<JoinHandle<()>>,
    /// Handle to the process monitor task
    process_monitor_handle: Option<JoinHandle<()>>,
    /// Child process handle (wrapped in Arc<Mutex> for shared access)
    child: Arc<std::sync::Mutex<Box<dyn Child + Send + Sync>>>,
    /// Output channel sender (wrapped for rebinding support)
    output_tx: Arc<tokio::sync::Mutex<mpsc::Sender<(Uuid, Vec<u8>)>>>,
    /// Output buffer for disconnected state
    output_buffer: Arc<tokio::sync::Mutex<Vec<u8>>>,
    /// Connection state flag
    is_connected: Arc<AtomicBool>,
}

impl PtyInstance {
    /// Create a new PTY instance running Claude Code
    pub fn new(
        id: Uuid,
        cwd: &str,
        output_tx: mpsc::Sender<(Uuid, Vec<u8>)>,
    ) -> Result<Self> {
        let pty_system = native_pty_system();

        // Create PTY with default size
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| anyhow!("Failed to open PTY: {}", e))?;

        // Build command for shell (user can then run claude manually)
        #[cfg(windows)]
        let mut cmd = CommandBuilder::new("cmd.exe");
        #[cfg(not(windows))]
        let mut cmd = CommandBuilder::new(std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string()));

        cmd.cwd(cwd);

        // Spawn the child process
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| anyhow!("Failed to spawn shell: {}", e))?;

        info!("Spawned shell process for instance {} in {}", id, cwd);

        // Get reader and writer
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| anyhow!("Failed to clone reader: {}", e))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| anyhow!("Failed to take writer: {}", e))?;

        // Wrap child in Arc<Mutex> for shared access
        let child = Arc::new(std::sync::Mutex::new(child));

        // Wrap reader in Arc<Mutex> for use in spawn_blocking
        let reader = Arc::new(std::sync::Mutex::new(reader));

        // Create shared state for reconnection support
        let output_tx = Arc::new(tokio::sync::Mutex::new(output_tx));
        let output_buffer = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let is_connected = Arc::new(AtomicBool::new(true));

        // Spawn a task to read PTY output using spawn_blocking to avoid blocking the async executor
        let reader_clone = Arc::clone(&reader);
        let output_tx_clone = Arc::clone(&output_tx);
        let output_buffer_clone = Arc::clone(&output_buffer);
        let is_connected_clone = Arc::clone(&is_connected);
        let reader_handle = tokio::spawn(async move {
            loop {
                let reader_ref = Arc::clone(&reader_clone);

                // Use spawn_blocking to perform the blocking read
                let read_result = tokio::task::spawn_blocking(move || {
                    let mut buffer = [0u8; 4096];
                    let mut guard = reader_ref.lock().unwrap();
                    guard.read(&mut buffer).map(|n| {
                        let data = buffer[..n].to_vec();
                        (n, data)
                    })
                })
                .await;

                match read_result {
                    Ok(Ok((0, _))) => {
                        debug!("PTY reader EOF for instance {}", id);
                        break;
                    }
                    Ok(Ok((_, data))) => {
                        // Check if connected - if so, send; if not, buffer
                        if is_connected_clone.load(Ordering::SeqCst) {
                            let tx = output_tx_clone.lock().await;
                            if tx.send((id, data)).await.is_err() {
                                // Channel closed - buffer instead of breaking
                                debug!("Output channel closed for instance {}, buffering", id);
                                is_connected_clone.store(false, Ordering::SeqCst);
                            }
                        } else {
                            // Buffer the output while disconnected
                            let mut buffer = output_buffer_clone.lock().await;
                            if buffer.len() + data.len() <= MAX_BUFFER_SIZE {
                                buffer.extend(data);
                            } else {
                                // Buffer full - drop oldest data to make room
                                let overflow = (buffer.len() + data.len()) - MAX_BUFFER_SIZE;
                                buffer.drain(..overflow);
                                buffer.extend(data);
                                debug!("Output buffer overflow for instance {}, dropped {} bytes", id, overflow);
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        if e.kind() != std::io::ErrorKind::WouldBlock {
                            error!("PTY read error for instance {}: {}", id, e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("spawn_blocking panicked for instance {}: {}", id, e);
                        break;
                    }
                }
            }
        });

        // Spawn a process monitor task to detect when the child exits
        let child_clone = Arc::clone(&child);
        let monitor_tx = Arc::clone(&output_tx);
        let monitor_connected = Arc::clone(&is_connected);
        let monitor_id = id;
        let process_monitor_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(500));
            loop {
                interval.tick().await;

                // Check if child process has exited
                let exited = {
                    let mut child_guard = child_clone.lock().unwrap();
                    match child_guard.try_wait() {
                        Ok(Some(status)) => {
                            info!("Process exited for instance {} with status {:?}", monitor_id, status);
                            true
                        }
                        Ok(None) => false, // Still running
                        Err(e) => {
                            warn!("Error checking process status for instance {}: {}", monitor_id, e);
                            true // Assume exited on error
                        }
                    }
                };

                if exited {
                    // Send empty data to signal process exit (only if connected)
                    if monitor_connected.load(Ordering::SeqCst) {
                        let tx = monitor_tx.lock().await;
                        let _ = tx.send((monitor_id, vec![])).await;
                    }
                    break;
                }
            }
        });

        Ok(Self {
            id,
            cwd: cwd.to_string(),
            master: pair.master,
            writer,
            reader_handle: Some(reader_handle),
            process_monitor_handle: Some(process_monitor_handle),
            child,
            output_tx,
            output_buffer,
            is_connected,
        })
    }

    /// Write data to the PTY input
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.writer
            .write_all(data)
            .map_err(|e| anyhow!("Failed to write to PTY: {}", e))?;
        self.writer
            .flush()
            .map_err(|e| anyhow!("Failed to flush PTY: {}", e))?;
        Ok(())
    }

    /// Resize the PTY
    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| anyhow!("Failed to resize PTY: {}", e))?;
        Ok(())
    }

    /// Check if the child process is still running
    pub fn is_running(&self) -> bool {
        let mut child_guard = self.child.lock().unwrap();
        match child_guard.try_wait() {
            Ok(Some(_)) => false, // Process has exited
            Ok(None) => true,     // Process is still running
            Err(_) => false,      // Error checking status
        }
    }

    /// Kill the child process
    pub fn kill(&self) -> Result<()> {
        let mut child_guard = self.child.lock().unwrap();
        child_guard
            .kill()
            .map_err(|e| anyhow!("Failed to kill process: {}", e))?;
        Ok(())
    }

    /// Rebind the output channel for reconnection
    /// This allows the PTY to send output to a new WebSocket connection
    pub async fn rebind_output_channel(&self, new_tx: mpsc::Sender<(Uuid, Vec<u8>)>) {
        let mut tx = self.output_tx.lock().await;
        *tx = new_tx;
        // Mark as connected so the reader task starts sending again
        self.is_connected.store(true, Ordering::SeqCst);
        debug!("Rebound output channel for instance {}", self.id);
    }

    /// Get and clear the buffered output
    /// Returns all output that was buffered while disconnected
    pub async fn drain_buffer(&self) -> Vec<u8> {
        let mut buffer = self.output_buffer.lock().await;
        std::mem::take(&mut *buffer)
    }

    /// Set the connection state
    /// When set to false, output will be buffered instead of sent
    pub fn set_connected(&self, connected: bool) {
        self.is_connected.store(connected, Ordering::SeqCst);
        debug!("Set connection state for instance {} to {}", self.id, connected);
    }

    /// Check if currently connected
    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::SeqCst)
    }
}

impl Drop for PtyInstance {
    fn drop(&mut self) {
        // Kill the child process if still running
        if self.is_running() {
            self.kill().ok();
        }

        // Abort the reader task
        if let Some(handle) = self.reader_handle.take() {
            handle.abort();
        }

        // Abort the process monitor task
        if let Some(handle) = self.process_monitor_handle.take() {
            handle.abort();
        }
    }
}
