/// Unified logger for centralized logging configuration

use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use std::path::PathBuf;

/// Logger configuration
#[derive(Debug, Clone)]
pub struct LoggerConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,

    /// Log to file
    pub log_to_file: bool,

    /// Log file path
    pub log_file_path: Option<PathBuf>,

    /// Include timestamps
    pub include_timestamps: bool,

    /// Include thread IDs
    pub include_thread_ids: bool,

    /// Include target module paths
    pub include_targets: bool,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            log_to_file: false,
            log_file_path: None,
            include_timestamps: true,
            include_thread_ids: false,
            include_targets: true,
        }
    }
}

/// Unified logger
pub struct UnifiedLogger;

impl UnifiedLogger {
    /// Initialize the global logger
    pub fn init(config: LoggerConfig) -> std::result::Result<(), Box<dyn std::error::Error>> {
        // Create filter
        let filter = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(&config.level))?;

        // Create console layer
        let console_layer = fmt::layer()
            .with_target(config.include_targets)
            .with_thread_ids(config.include_thread_ids)
            .with_ansi(true);

        // Build subscriber
        let subscriber = tracing_subscriber::registry()
            .with(filter)
            .with(console_layer);

        // Set as global default
        tracing::subscriber::set_global_default(subscriber)?;

        tracing::info!("Logging initialized with level: {}", config.level);

        Ok(())
    }

    /// Initialize with default configuration
    pub fn init_default() -> std::result::Result<(), Box<dyn std::error::Error>> {
        Self::init(LoggerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_config_default() {
        let config = LoggerConfig::default();
        assert_eq!(config.level, "info");
        assert!(!config.log_to_file);
    }
}
