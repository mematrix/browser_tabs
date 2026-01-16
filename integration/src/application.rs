/// Main application module
///
/// Provides high-level Application API

use crate::{AppContext, AppConfig, UnifiedLogger};
use web_page_manager_core::errors::Result;
use web_page_manager_core::types::*;
use std::sync::Arc;
use tracing::info;

/// Main application
pub struct Application {
    /// Application context
    context: Arc<AppContext>,
}

impl Application {
    /// Create and initialize a new application
    pub async fn new(config: AppConfig) -> Result<Self> {
        // Initialize logging
        UnifiedLogger::init_default()
            .map_err(|e| web_page_manager_core::errors::SystemError::Configuration {
                details: e.to_string()
            })?;

        info!("Starting Webpage Manager Application");

        // Create application context
        let context = Arc::new(AppContext::new(config).await?);

        // Auto-connect to browsers if enabled
        let config_guard = context.config.read().await;
        if config_guard.auto_connect_browsers {
            drop(config_guard);
            context.connect_browsers().await;
        }

        info!("Application initialized successfully");

        Ok(Self { context })
    }

    /// Run the application
    pub async fn run(&self) -> Result<()> {
        info!("Running application");

        // Initialize UI
        let ui_manager = self.context.ui_manager.read().await;

        // Show main window
        ui_manager.show_main_window().await?;

        Ok(())
    }

    /// Shutdown the application
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down application");
        self.context.shutdown().await?;
        info!("Application shutdown complete");
        Ok(())
    }

    /// Get application context
    pub fn context(&self) -> &Arc<AppContext> {
        &self.context
    }

    // High-level API methods

    /// Search across all data sources
    pub async fn search(&self, query: &str) -> Result<Vec<UnifiedPageInfo>> {
        Ok(self.context.search(query).await)
    }

    /// Get application statistics
    pub async fn get_stats(&self) -> Result<crate::AppStatistics> {
        Ok(self.context.get_stats().await)
    }

    /// Connect to a browser
    pub async fn connect_browser(&self, browser_type: &BrowserType) -> Result<()> {
        self.context.browser_manager.connect(browser_type.clone()).await?;
        info!("Connected to {:?}", browser_type);
        Ok(())
    }

    /// Disconnect from a browser
    pub async fn disconnect_browser(&self, browser_type: &BrowserType) -> Result<()> {
        self.context.browser_manager.disconnect(browser_type.clone()).await?;
        info!("Disconnected from {:?}", browser_type);
        Ok(())
    }

    /// Get all pages
    pub async fn get_all_pages(&self) -> Result<Vec<UnifiedPageInfo>> {
        Ok(self.context.get_all_pages().await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_application_creation() {
        let config = AppConfig::default();
        let app = Application::new(config).await;
        assert!(app.is_ok());
    }

    #[tokio::test]
    async fn test_application_lifecycle() {
        let config = AppConfig::default();
        let app = Application::new(config).await.unwrap();

        // Test shutdown
        let result = app.shutdown().await;
        assert!(result.is_ok());
    }
}
