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

/// Type alias for output channel sender to reduce complexity
type OutputSender = Arc<tokio::sync::Mutex<mpsc::Sender<(Uuid, Vec<u8>)>>>;

/// PTY mode: background (invisible) or visible (with local console window)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PtyMode {
    /// Background mode - no visible console window
    #[default]
    Background,
    /// Visible mode - shows a console window for local interaction
    Visible,
}

/// PTY wrapper for a Claude Code instance
pub struct PtyInstance {
    /// Instance ID
    pub id: Uuid,
    /// Working directory (for reconnection sync)
    pub cwd: String,
    /// PTY mode (background or visible)
    #[allow(dead_code)]
    pub mode: PtyMode,
    /// PTY master handle (only used in Background mode)
    master: Option<Box<dyn portable_pty::MasterPty + Send>>,
    /// Writer to send data to PTY
    writer: Box<dyn Write + Send>,
    /// Handle to the reader task
    reader_handle: Option<JoinHandle<()>>,
    /// Handle to the process monitor task
    process_monitor_handle: Option<JoinHandle<()>>,
    /// Child process handle (wrapped in Arc<Mutex> for shared access)
    child: Arc<std::sync::Mutex<Box<dyn Child + Send + Sync>>>,
    /// Output channel sender (wrapped for rebinding support)
    output_tx: OutputSender,
    /// Output buffer for disconnected state
    output_buffer: Arc<tokio::sync::Mutex<Vec<u8>>>,
    /// Connection state flag
    is_connected: Arc<AtomicBool>,
    /// Visible mode child process (Windows only)
    #[cfg(windows)]
    visible_child: Option<std::process::Child>,
}

impl PtyInstance {
    /// Create a new PTY instance running Claude Code (background mode)
    #[allow(dead_code)]
    pub fn new(
        id: Uuid,
        cwd: &str,
        output_tx: mpsc::Sender<(Uuid, Vec<u8>)>,
    ) -> Result<Self> {
        Self::new_with_mode(id, cwd, output_tx, PtyMode::Background)
    }

    /// Create a new PTY instance with specified mode
    pub fn new_with_mode(
        id: Uuid,
        cwd: &str,
        output_tx: mpsc::Sender<(Uuid, Vec<u8>)>,
        mode: PtyMode,
    ) -> Result<Self> {
        match mode {
            PtyMode::Background => Self::create_background_pty(id, cwd, output_tx),
            PtyMode::Visible => Self::create_visible_pty(id, cwd, output_tx),
        }
    }

    /// Create a background (invisible) PTY instance
    fn create_background_pty(
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

        info!("Spawned background shell process for instance {} in {}", id, cwd);

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

        // Spawn reader task
        let reader_handle = Self::spawn_reader_task(
            id,
            Arc::clone(&reader),
            Arc::clone(&output_tx),
            Arc::clone(&output_buffer),
            Arc::clone(&is_connected),
        );

        // Spawn process monitor task
        let process_monitor_handle = Self::spawn_monitor_task(
            id,
            Arc::clone(&child),
            Arc::clone(&output_tx),
            Arc::clone(&is_connected),
        );

        Ok(Self {
            id,
            cwd: cwd.to_string(),
            mode: PtyMode::Background,
            master: Some(pair.master),
            writer,
            reader_handle: Some(reader_handle),
            process_monitor_handle: Some(process_monitor_handle),
            child,
            output_tx,
            output_buffer,
            is_connected,
            #[cfg(windows)]
            visible_child: None,
        })
    }

    /// Create a visible PTY instance with local console viewer window (read-only)
    /// Uses ConPTY for full terminal functionality, with a separate viewer console
    #[cfg(windows)]
    fn create_visible_pty(
        id: Uuid,
        cwd: &str,
        output_tx: mpsc::Sender<(Uuid, Vec<u8>)>,
    ) -> Result<Self> {
        use windows_sys::Win32::System::Console::{
            AllocConsole, FreeConsole, GetStdHandle, SetConsoleTitleW, STD_OUTPUT_HANDLE,
        };

        let pty_system = native_pty_system();

        // Create PTY with default size (same as background mode)
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| anyhow!("Failed to open PTY: {}", e))?;

        // Build command for shell
        let mut cmd = CommandBuilder::new("cmd.exe");
        cmd.cwd(cwd);

        // Spawn the child process
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| anyhow!("Failed to spawn shell: {}", e))?;

        info!("Spawned visible shell process for instance {} in {}", id, cwd);

        // Get reader and writer
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| anyhow!("Failed to clone reader: {}", e))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| anyhow!("Failed to take writer: {}", e))?;

        // Create local viewer console window
        let viewer_handle = unsafe {
            // Free any existing console first
            FreeConsole();
            // Allocate a new console window
            if AllocConsole() == 0 {
                warn!("Failed to allocate viewer console, continuing without local display");
                None
            } else {
                // Set console title
                let title: Vec<u16> = format!("PTY Viewer [{}] - Read Only\0", id)
                    .encode_utf16()
                    .collect();
                SetConsoleTitleW(title.as_ptr());

                let handle = GetStdHandle(STD_OUTPUT_HANDLE);
                if handle.is_null() || handle == -1isize as *mut _ {
                    warn!("Failed to get viewer console handle");
                    None
                } else {
                    info!("Created local viewer console for instance {}", id);
                    Some(handle as isize)
                }
            }
        };

        // Wrap child in Arc<Mutex> for shared access
        let child = Arc::new(std::sync::Mutex::new(child));

        // Wrap reader in Arc<Mutex> for use in spawn_blocking
        let reader = Arc::new(std::sync::Mutex::new(reader));

        // Create shared state
        let output_tx = Arc::new(tokio::sync::Mutex::new(output_tx));
        let output_buffer = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let is_connected = Arc::new(AtomicBool::new(true));

        // Spawn reader task with viewer support
        let reader_handle = Self::spawn_reader_task_with_viewer(
            id,
            Arc::clone(&reader),
            Arc::clone(&output_tx),
            Arc::clone(&output_buffer),
            Arc::clone(&is_connected),
            viewer_handle,
        );

        // Spawn process monitor task
        let process_monitor_handle = Self::spawn_monitor_task(
            id,
            Arc::clone(&child),
            Arc::clone(&output_tx),
            Arc::clone(&is_connected),
        );

        Ok(Self {
            id,
            cwd: cwd.to_string(),
            mode: PtyMode::Visible,
            master: Some(pair.master),
            writer,
            reader_handle: Some(reader_handle),
            process_monitor_handle: Some(process_monitor_handle),
            child,
            output_tx,
            output_buffer,
            is_connected,
            visible_child: None,
        })
    }

    /// Spawn reader task that also writes to local viewer console
    #[cfg(windows)]
    fn spawn_reader_task_with_viewer(
        id: Uuid,
        reader: Arc<std::sync::Mutex<Box<dyn Read + Send>>>,
        output_tx: OutputSender,
        output_buffer: Arc<tokio::sync::Mutex<Vec<u8>>>,
        is_connected: Arc<AtomicBool>,
        viewer_handle: Option<isize>,
    ) -> JoinHandle<()> {
        // Enable virtual terminal processing for ANSI escape sequence support
        if let Some(handle) = viewer_handle {
            use windows_sys::Win32::Foundation::HANDLE;
            use windows_sys::Win32::System::Console::{
                GetConsoleMode, SetConsoleMode, ENABLE_VIRTUAL_TERMINAL_PROCESSING,
                ENABLE_PROCESSED_OUTPUT,
            };

            unsafe {
                let mut mode: u32 = 0;
                if GetConsoleMode(handle as HANDLE, &mut mode) != 0 {
                    // Enable VT processing for ANSI escape sequences
                    let new_mode = mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING | ENABLE_PROCESSED_OUTPUT;
                    SetConsoleMode(handle as HANDLE, new_mode);
                }
            }
        }

        tokio::spawn(async move {
            loop {
                let reader_ref = Arc::clone(&reader);
                let viewer = viewer_handle;

                // Use spawn_blocking to perform the blocking read
                let read_result = tokio::task::spawn_blocking(move || {
                    let mut buffer = [0u8; 4096];
                    let mut guard = reader_ref.lock().unwrap();
                    guard.read(&mut buffer).map(|n| {
                        let data = buffer[..n].to_vec();

                        // Also write to local viewer console if available
                        if let Some(handle) = viewer {
                            use windows_sys::Win32::Foundation::HANDLE;
                            use windows_sys::Win32::System::Console::WriteConsoleW;

                            // Convert UTF-8 to UTF-16 for proper Unicode display
                            let text = String::from_utf8_lossy(&data);
                            let wide: Vec<u16> = text.encode_utf16().collect();

                            unsafe {
                                let mut written: u32 = 0;
                                WriteConsoleW(
                                    handle as HANDLE,
                                    wide.as_ptr(),
                                    wide.len() as u32,
                                    &mut written,
                                    std::ptr::null_mut(),
                                );
                            }
                        }

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
                        Self::handle_output_data(
                            id,
                            data,
                            &output_tx,
                            &output_buffer,
                            &is_connected,
                        )
                        .await;
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
        })
    }


    /// Create a visible PTY instance (non-Windows fallback - uses background mode)
    #[cfg(not(windows))]
    fn create_visible_pty(
        id: Uuid,
        cwd: &str,
        output_tx: mpsc::Sender<(Uuid, Vec<u8>)>,
    ) -> Result<Self> {
        warn!("Visible terminal mode is only supported on Windows, falling back to background mode");
        Self::create_background_pty(id, cwd, output_tx)
    }

    /// Spawn the reader task for background PTY
    fn spawn_reader_task(
        id: Uuid,
        reader: Arc<std::sync::Mutex<Box<dyn Read + Send>>>,
        output_tx: OutputSender,
        output_buffer: Arc<tokio::sync::Mutex<Vec<u8>>>,
        is_connected: Arc<AtomicBool>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                let reader_ref = Arc::clone(&reader);

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
                        Self::handle_output_data(
                            id,
                            data,
                            &output_tx,
                            &output_buffer,
                            &is_connected,
                        )
                        .await;
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
        })
    }

    /// Handle output data - send to channel or buffer
    async fn handle_output_data(
        id: Uuid,
        data: Vec<u8>,
        output_tx: &OutputSender,
        output_buffer: &Arc<tokio::sync::Mutex<Vec<u8>>>,
        is_connected: &Arc<AtomicBool>,
    ) {
        if is_connected.load(Ordering::SeqCst) {
            let tx = output_tx.lock().await;
            if tx.send((id, data)).await.is_err() {
                debug!("Output channel closed for instance {}, buffering", id);
                is_connected.store(false, Ordering::SeqCst);
            }
        } else {
            let mut buffer = output_buffer.lock().await;
            if buffer.len() + data.len() <= MAX_BUFFER_SIZE {
                buffer.extend(data);
            } else {
                let overflow = (buffer.len() + data.len()) - MAX_BUFFER_SIZE;
                buffer.drain(..overflow);
                buffer.extend(data);
                debug!("Output buffer overflow for instance {}, dropped {} bytes", id, overflow);
            }
        }
    }

    /// Spawn the process monitor task
    fn spawn_monitor_task(
        id: Uuid,
        child: Arc<std::sync::Mutex<Box<dyn Child + Send + Sync>>>,
        output_tx: OutputSender,
        is_connected: Arc<AtomicBool>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(500));
            loop {
                interval.tick().await;

                let exited = {
                    let mut child_guard = child.lock().unwrap();
                    match child_guard.try_wait() {
                        Ok(Some(status)) => {
                            info!("Process exited for instance {} with status {:?}", id, status);
                            true
                        }
                        Ok(None) => false,
                        Err(e) => {
                            warn!("Error checking process status for instance {}: {}", id, e);
                            true
                        }
                    }
                };

                if exited {
                    if is_connected.load(Ordering::SeqCst) {
                        let tx = output_tx.lock().await;
                        let _ = tx.send((id, vec![])).await;
                    }
                    break;
                }
            }
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

    /// Resize the PTY (only works in background mode)
    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        if let Some(ref master) = self.master {
            info!("Applying PTY resize for instance {}: {}x{}", self.id, cols, rows);
            master
                .resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|e| anyhow!("Failed to resize PTY: {}", e))?;
        } else {
            warn!("Resize not supported - no master handle for instance {}", self.id);
        }
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

        // Kill visible child process on Windows
        #[cfg(windows)]
        if let Some(ref mut child) = self.visible_child {
            let _ = child.kill();
        }
    }
}
