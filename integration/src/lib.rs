/// Integration module for webpage-manager
///
/// Task 11.1: Component Integration and Data Flow
/// This module provides orchestration of all system components

use web_page_manager_core::errors::Result;
use web_page_manager_core::types::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

pub mod application;
pub mod error_handler;
pub mod logger;

pub use application::Application;
pub use error_handler::{UnifiedErrorHandler, ErrorSeverity, ErrorStatistics};
pub use logger::{UnifiedLogger, LoggerConfig};

/// Application configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppConfig {
    /// Database path
    pub database_path: Option<std::path::PathBuf>,

    /// Enable AI processing
    pub enable_ai: bool,

    /// Enable browser auto-connection
    pub auto_connect_browsers: bool,

    /// Cache size limit in MB
    pub cache_size_mb: usize,

    /// History retention days
    pub history_retention_days: u32,

    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,

    /// Log level
    pub log_level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database_path: None,
            enable_ai: true,
            auto_connect_browsers: true,
            cache_size_mb: 100,
            history_retention_days: 30,
            enable_performance_monitoring: true,
            log_level: "info".to_string(),
        }
    }
}

/// Application context that holds all initialized components
pub struct AppContext {
    /// Database manager for data persistence
    pub database: Arc<data_access::DatabaseManager>,

    /// Browser connector manager for multi-browser support
    pub browser_manager: Arc<browser_connector::BrowserConnectorManager>,

    /// Page manager for unified page operations
    pub page_manager: Arc<page_manager::PageUnifiedManager>,

    /// UI manager for framework-specific operations
    pub ui_manager: Arc<RwLock<Box<dyn ui_manager::UIManager>>>,

    /// Unified error handler
    pub error_handler: Arc<UnifiedErrorHandler>,

    /// Application configuration
    pub config: Arc<RwLock<AppConfig>>,
}

impl AppContext {
    /// Create a new application context with all components initialized
    pub async fn new(config: AppConfig) -> Result<Self> {
        info!("Initializing application context");

        // Initialize database
        let database = if let Some(path) = &config.database_path {
            Arc::new(data_access::DatabaseManager::new(path).await?)
        } else {
            Arc::new(data_access::DatabaseManager::in_memory().await?)
        };
        info!("Database initialized");

        // Initialize browser connector manager
        let browser_manager = Arc::new(browser_connector::BrowserConnectorManager::new());
        info!("Browser connector manager initialized");

        // Initialize page manager
        let page_manager = Arc::new(page_manager::PageUnifiedManager::new());
        info!("Page manager initialized");

        // Initialize UI manager
        let ui_manager = Arc::new(RwLock::new(
            ui_manager::UIManagerFactory::create()
        ));
        info!("UI manager initialized");

        // Initialize error handler
        let error_handler = Arc::new(UnifiedErrorHandler::new());

        let config = Arc::new(RwLock::new(config));

        info!("Application context initialized successfully");

        Ok(Self {
            database,
            browser_manager,
            page_manager,
            ui_manager,
            error_handler,
            config,
        })
    }

    /// Shutdown all components gracefully
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down application context");

        // Disconnect all browsers
        if let Err(e) = self.browser_manager.disconnect_all().await {
            warn!("Error disconnecting browsers: {}", e);
        }
        info!("Browser connections closed");

        // Clear caches
        self.database.clear_cache().await;
        info!("Caches cleared");

        info!("Application context shutdown complete");
        Ok(())
    }

    /// Connect to all available browsers
    pub async fn connect_browsers(&self) -> Vec<BrowserType> {
        info!("Connecting to browsers");
        let connected = self.browser_manager.connect_all().await;
        info!("Connected to {} browser(s)", connected.len());
        connected
    }

    /// Get all unified pages
    pub async fn get_all_pages(&self) -> Vec<UnifiedPageInfo> {
        self.page_manager.get_unified_pages().await
    }

    /// Search across all data
    pub async fn search(&self, query: &str) -> Vec<UnifiedPageInfo> {
        self.page_manager.search_pages(query).await
    }

    /// Get application statistics
    pub async fn get_stats(&self) -> AppStatistics {
        let page_stats = self.page_manager.get_stats().await;
        let connected_browsers = self.browser_manager.connected_count().await;

        AppStatistics {
            total_pages: page_stats.total_pages,
            total_tabs: page_stats.active_tab_pages,
            total_bookmarks: page_stats.bookmark_only_pages,
            connected_browsers,
        }
    }
}

/// Application statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppStatistics {
    pub total_pages: usize,
    pub total_tabs: usize,
    pub total_bookmarks: usize,
    pub connected_browsers: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_app_context_creation() {
        let config = AppConfig::default();
        let context = AppContext::new(config).await;
        assert!(context.is_ok());
    }

    #[tokio::test]
    async fn test_app_context_shutdown() {
        let config = AppConfig::default();
        let context = AppContext::new(config).await.unwrap();
        let result = context.shutdown().await;
        assert!(result.is_ok());
    }
}
