//! Logging system with daily file rotation

use std::path::Path;
use common::LoggingConfig;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::prelude::*;

/// Guard that keeps the file appender alive
pub struct LogGuard {
    _guard: Option<tracing_appender::non_blocking::WorkerGuard>,
}

/// Initialize logging system with optional file rotation
pub fn init_logging(config: &LoggingConfig) -> LogGuard {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.level));

    // Console layer (always enabled)
    let console_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);

    // File layer (optional)
    if let Some(ref file_path) = config.file {
        // Ensure log directory exists
        if let Some(parent) = Path::new(file_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Parse rotation policy
        let rotation = match config.rotation.to_lowercase().as_str() {
            "hourly" => Rotation::HOURLY,
            _ => Rotation::DAILY, // Default to daily for "daily" and any other value
        };

        // Get directory and file prefix from path
        let (dir, file_prefix) = if let Some(parent) = file_path.parent() {
            let prefix = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("agent");
            (parent.to_path_buf(), prefix.to_string())
        } else {
            (std::path::PathBuf::from("."), "agent".to_string())
        };

        // Create rolling file appender
        let file_appender = RollingFileAppender::new(rotation, dir, file_prefix);
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        // File layer with detailed format
        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true);

        tracing_subscriber::registry()
            .with(filter)
            .with(console_layer)
            .with(file_layer)
            .init();

        LogGuard {
            _guard: Some(guard),
        }
    } else {
        // Console only
        tracing_subscriber::registry()
            .with(filter)
            .with(console_layer)
            .init();

        LogGuard { _guard: None }
    }
}
