//! Browser Connector module for Web Page Manager
//! 
//! This module provides functionality to connect to and communicate with
//! multiple browsers using their respective APIs (CDP for Chrome/Edge,
//! WebExtensions for Firefox).
//!
//! # Features
//! - Automatic browser detection for Chrome, Edge, and Firefox
//! - CDP (Chrome DevTools Protocol) support for Chromium-based browsers
//! - WebExtensions Native Messaging support for Firefox
//! - Privacy mode filtering to exclude incognito/private tabs
//! - Browser instance lifecycle management
//! - Tab state monitoring and change detection
//! - Enhanced tab information extraction and categorization
//! - Bookmark import from multiple browsers with validation

pub mod traits;
pub mod cdp;
pub mod firefox;
pub mod privacy_filter;
pub mod tab_monitor;
pub mod tab_extractor;
pub mod bookmark_import;
pub mod bookmark_content_analyzer;

pub use traits::*;
pub use cdp::{ChromeConnector, EdgeConnector, CdpTarget, CdpVersion};
pub use firefox::FirefoxConnector;
pub use privacy_filter::{PrivacyModeFilter, PrivacyFilterConfig, FilterStats};
pub use tab_monitor::{TabMonitor, TabMonitorConfig, TabEvent, TabMonitorStats};
pub use tab_extractor::{TabExtractor, ExtendedTabInfo, TabCategory, TabStats};
pub use bookmark_import::{
    BookmarkImporter, BookmarkValidator, BookmarkSource, ImportProgress, ImportStatus,
    BookmarkValidationResult, ValidationReport, ChromeBookmarks, ChromeBookmarkNode,
};
pub use bookmark_content_analyzer::{
    BookmarkContentAnalyzer, BookmarkContentAnalyzerConfig, BookmarkContentResult,
    BatchAnalysisResult, BatchBookmarkProcessor, BatchAnalysisConfig, BatchBookmarkAnalysis,
    MergeSuggestion, MergedBookmarkMetadata,
};

use web_page_manager_core::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Browser connection status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Not connected to the browser
    Disconnected,
    /// Currently connecting
    Connecting,
    /// Successfully connected
    Connected,
    /// Connection failed with error
    Failed(String),
}

/// Browser instance with connection state
#[derive(Debug)]
pub struct ManagedBrowserInstance {
    pub instance: BrowserInstance,
    pub status: ConnectionStatus,
    pub last_error: Option<String>,
    pub connected_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Browser connector manager that handles multiple browser connections
/// 
/// This is the main entry point for browser connectivity. It manages
/// connections to multiple browsers simultaneously and provides a unified
/// interface for browser operations.
pub struct BrowserConnectorManager {
    connections: Arc<RwLock<HashMap<BrowserType, Box<dyn BrowserConnector>>>>,
    instances: Arc<RwLock<HashMap<BrowserType, ManagedBrowserInstance>>>,
    privacy_filter: PrivacyModeFilter,
    tab_monitor: Arc<TabMonitor>,
    tab_extractor: TabExtractor,
}

impl BrowserConnectorManager {
    /// Create a new browser connector manager
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            instances: Arc::new(RwLock::new(HashMap::new())),
            privacy_filter: PrivacyModeFilter::new(),
            tab_monitor: Arc::new(TabMonitor::new()),
            tab_extractor: TabExtractor::new(),
        }
    }

    /// Create a new browser connector manager with custom configuration
    pub fn with_config(
        privacy_config: PrivacyFilterConfig,
        monitor_config: TabMonitorConfig,
    ) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            instances: Arc::new(RwLock::new(HashMap::new())),
            privacy_filter: PrivacyModeFilter::with_config(privacy_config),
            tab_monitor: Arc::new(TabMonitor::with_config(monitor_config)),
            tab_extractor: TabExtractor::new(),
        }
    }

    /// Get a reference to the tab monitor
    pub fn tab_monitor(&self) -> &Arc<TabMonitor> {
        &self.tab_monitor
    }

    /// Get a reference to the tab extractor
    pub fn tab_extractor(&self) -> &TabExtractor {
        &self.tab_extractor
    }

    /// Get a reference to the privacy filter
    pub fn privacy_filter(&self) -> &PrivacyModeFilter {
        &self.privacy_filter
    }

    /// Detect all running browsers that support remote debugging
    /// 
    /// This method checks for:
    /// - Chrome with remote debugging enabled (port 9222)
    /// - Edge with remote debugging enabled (port 9223)
    /// - Firefox with profile directory present
    pub async fn detect_browsers(&self) -> Result<Vec<BrowserInstance>> {
        let mut browsers = Vec::new();
        
        // Detect Chrome on default and common ports
        for port in [9222, 9229] {
            if let Ok(chrome) = ChromeConnector::detect_on_port(port).await {
                browsers.push(chrome);
                break;
            }
        }
        
        // Detect Edge on default and common ports
        for port in [9223, 9224] {
            if let Ok(edge) = EdgeConnector::detect_on_port(port).await {
                browsers.push(edge);
                break;
            }
        }
        
        // Detect Firefox
        if let Ok(firefox) = FirefoxConnector::detect().await {
            browsers.push(firefox);
        }
        
        // Update instances cache
        let mut instances = self.instances.write().await;
        for browser in &browsers {
            instances.insert(browser.browser_type, ManagedBrowserInstance {
                instance: browser.clone(),
                status: ConnectionStatus::Disconnected,
                last_error: None,
                connected_at: None,
            });
        }
        
        Ok(browsers)
    }

    /// Get the list of detected browser instances
    pub async fn get_detected_instances(&self) -> Vec<ManagedBrowserInstance> {
        let instances = self.instances.read().await;
        instances.values()
            .map(|i| ManagedBrowserInstance {
                instance: i.instance.clone(),
                status: i.status.clone(),
                last_error: i.last_error.clone(),
                connected_at: i.connected_at,
            })
            .collect()
    }

    /// Get connection status for a specific browser
    pub async fn get_connection_status(&self, browser_type: BrowserType) -> ConnectionStatus {
        let instances = self.instances.read().await;
        instances.get(&browser_type)
            .map(|i| i.status.clone())
            .unwrap_or(ConnectionStatus::Disconnected)
    }

    /// Connect to a specific browser
    /// 
    /// # Arguments
    /// * `browser_type` - The type of browser to connect to
    /// 
    /// # Returns
    /// * `Ok(())` if connection successful
    /// * `Err` if browser is not running or connection fails
    pub async fn connect(&self, browser_type: BrowserType) -> Result<()> {
        // Update status to connecting
        {
            let mut instances = self.instances.write().await;
            if let Some(instance) = instances.get_mut(&browser_type) {
                instance.status = ConnectionStatus::Connecting;
            }
        }
        
        let connector: Box<dyn BrowserConnector> = match browser_type {
            BrowserType::Chrome => {
                let mut connector = ChromeConnector::new();
                // Check if we have a detected instance with a specific port
                let instances = self.instances.read().await;
                if let Some(instance) = instances.get(&browser_type) {
                    if let Some(port) = instance.instance.debug_port {
                        connector = ChromeConnector::with_port(port);
                    }
                }
                drop(instances);
                Box::new(connector)
            }
            BrowserType::Edge => {
                let mut connector = EdgeConnector::new();
                let instances = self.instances.read().await;
                if let Some(instance) = instances.get(&browser_type) {
                    if let Some(port) = instance.instance.debug_port {
                        connector = EdgeConnector::with_port(port);
                    }
                }
                drop(instances);
                Box::new(connector)
            }
            BrowserType::Firefox => Box::new(FirefoxConnector::new()),
            BrowserType::Safari => {
                // Update status to failed
                let mut instances = self.instances.write().await;
                if let Some(instance) = instances.get_mut(&browser_type) {
                    instance.status = ConnectionStatus::Failed("Safari not supported".to_string());
                    instance.last_error = Some("Safari not supported".to_string());
                }
                
                return Err(WebPageManagerError::BrowserConnection {
                    source: BrowserConnectionError::BrowserNotRunning {
                        browser: BrowserType::Safari,
                    },
                });
            }
        };
        
        // Attempt connection
        match connector.connect().await {
            Ok(()) => {
                let mut connections = self.connections.write().await;
                connections.insert(browser_type, connector);
                
                let mut instances = self.instances.write().await;
                if let Some(instance) = instances.get_mut(&browser_type) {
                    instance.status = ConnectionStatus::Connected;
                    instance.last_error = None;
                    instance.connected_at = Some(chrono::Utc::now());
                }
                
                tracing::info!("Successfully connected to {:?}", browser_type);
                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                
                let mut instances = self.instances.write().await;
                if let Some(instance) = instances.get_mut(&browser_type) {
                    instance.status = ConnectionStatus::Failed(error_msg.clone());
                    instance.last_error = Some(error_msg);
                }
                
                Err(e)
            }
        }
    }

    /// Connect to all detected browsers
    /// 
    /// Returns a list of browser types that were successfully connected
    pub async fn connect_all(&self) -> Vec<BrowserType> {
        let mut connected = Vec::new();
        
        // First detect browsers
        let _ = self.detect_browsers().await;
        
        // Get list of detected browsers
        let instances = self.instances.read().await;
        let browser_types: Vec<BrowserType> = instances.keys().copied().collect();
        drop(instances);
        
        // Try to connect to each
        for browser_type in browser_types {
            if self.connect(browser_type).await.is_ok() {
                connected.push(browser_type);
            }
        }
        
        connected
    }

    /// Get tabs from a connected browser (filtered for privacy mode)
    /// 
    /// # Arguments
    /// * `browser_type` - The browser to get tabs from
    /// 
    /// # Returns
    /// * List of tabs, excluding private/incognito tabs
    pub async fn get_tabs(&self, browser_type: BrowserType) -> Result<Vec<TabInfo>> {
        let connections = self.connections.read().await;
        
        let connector = connections.get(&browser_type).ok_or_else(|| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: browser_type,
                },
            }
        })?;
        
        let all_tabs = connector.get_tabs().await?;
        
        // Filter out private/incognito tabs
        let filtered_tabs = self.privacy_filter.filter_tabs(all_tabs);
        
        Ok(filtered_tabs)
    }

    /// Get tabs from all connected browsers
    /// 
    /// Returns a map of browser type to tabs, with private tabs filtered out
    pub async fn get_all_tabs(&self) -> HashMap<BrowserType, Vec<TabInfo>> {
        let mut all_tabs = HashMap::new();
        
        let connections = self.connections.read().await;
        for (browser_type, connector) in connections.iter() {
            if let Ok(tabs) = connector.get_tabs().await {
                let filtered = self.privacy_filter.filter_tabs(tabs);
                all_tabs.insert(*browser_type, filtered);
            }
        }
        
        all_tabs
    }

    /// Get bookmarks from a connected browser
    pub async fn get_bookmarks(&self, browser_type: BrowserType) -> Result<Vec<BookmarkInfo>> {
        let connections = self.connections.read().await;
        
        let connector = connections.get(&browser_type).ok_or_else(|| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: browser_type,
                },
            }
        })?;
        
        connector.get_bookmarks().await
    }

    /// Get bookmarks from all connected browsers
    pub async fn get_all_bookmarks(&self) -> HashMap<BrowserType, Vec<BookmarkInfo>> {
        let mut all_bookmarks = HashMap::new();
        
        let connections = self.connections.read().await;
        for (browser_type, connector) in connections.iter() {
            if let Ok(bookmarks) = connector.get_bookmarks().await {
                all_bookmarks.insert(*browser_type, bookmarks);
            }
        }
        
        all_bookmarks
    }

    /// Fetch page content from a URL using a specific browser
    pub async fn fetch_page_content(&self, browser_type: BrowserType, url: &str) -> Result<PageContent> {
        let connections = self.connections.read().await;
        
        let connector = connections.get(&browser_type).ok_or_else(|| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: browser_type,
                },
            }
        })?;
        
        connector.fetch_page_content(url).await
    }

    /// Close a tab in a specific browser
    pub async fn close_tab(&self, browser_type: BrowserType, tab_id: &TabId) -> Result<()> {
        let connections = self.connections.read().await;
        
        let connector = connections.get(&browser_type).ok_or_else(|| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: browser_type,
                },
            }
        })?;
        
        connector.close_tab(tab_id).await
    }

    /// Activate a tab in a specific browser
    pub async fn activate_tab(&self, browser_type: BrowserType, tab_id: &TabId) -> Result<()> {
        let connections = self.connections.read().await;
        
        let connector = connections.get(&browser_type).ok_or_else(|| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: browser_type,
                },
            }
        })?;
        
        connector.activate_tab(tab_id).await
    }

    /// Create a new tab in a specific browser
    pub async fn create_tab(&self, browser_type: BrowserType, url: &str) -> Result<TabId> {
        let connections = self.connections.read().await;
        
        let connector = connections.get(&browser_type).ok_or_else(|| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: browser_type,
                },
            }
        })?;
        
        connector.create_tab(url).await
    }

    /// Disconnect from a specific browser
    pub async fn disconnect(&self, browser_type: BrowserType) -> Result<()> {
        let mut connections = self.connections.write().await;
        
        if let Some(connector) = connections.remove(&browser_type) {
            connector.disconnect().await?;
        }
        
        let mut instances = self.instances.write().await;
        if let Some(instance) = instances.get_mut(&browser_type) {
            instance.status = ConnectionStatus::Disconnected;
            instance.connected_at = None;
        }
        
        Ok(())
    }

    /// Disconnect from all browsers
    pub async fn disconnect_all(&self) -> Result<()> {
        let mut connections = self.connections.write().await;
        
        for (browser_type, connector) in connections.drain() {
            let _ = connector.disconnect().await;
            
            let mut instances = self.instances.write().await;
            if let Some(instance) = instances.get_mut(&browser_type) {
                instance.status = ConnectionStatus::Disconnected;
                instance.connected_at = None;
            }
        }
        
        Ok(())
    }

    /// Check if connected to a specific browser
    pub async fn is_connected(&self, browser_type: BrowserType) -> bool {
        let connections = self.connections.read().await;
        connections.get(&browser_type)
            .map(|c| c.is_connected())
            .unwrap_or(false)
    }

    /// Get list of currently connected browsers
    pub async fn get_connected_browsers(&self) -> Vec<BrowserType> {
        let connections = self.connections.read().await;
        connections.keys().copied().collect()
    }

    /// Get the number of connected browsers
    pub async fn connected_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    // ============================================================
    // Enhanced Tab Extraction and Monitoring Methods
    // ============================================================

    /// Get extended tab information from a connected browser
    /// 
    /// This method returns tabs with additional metadata including
    /// domain extraction, categorization, and URL analysis.
    pub async fn get_extended_tabs(&self, browser_type: BrowserType) -> Result<Vec<ExtendedTabInfo>> {
        let tabs = self.get_tabs(browser_type).await?;
        Ok(self.tab_extractor.extract_all(&tabs))
    }

    /// Get extended tab information from all connected browsers
    pub async fn get_all_extended_tabs(&self) -> HashMap<BrowserType, Vec<ExtendedTabInfo>> {
        let all_tabs = self.get_all_tabs().await;
        all_tabs.into_iter()
            .map(|(browser_type, tabs)| {
                (browser_type, self.tab_extractor.extract_all(&tabs))
            })
            .collect()
    }

    /// Get tabs grouped by domain from a specific browser
    pub async fn get_tabs_by_domain(&self, browser_type: BrowserType) -> Result<HashMap<String, Vec<TabInfo>>> {
        let tabs = self.get_tabs(browser_type).await?;
        Ok(self.tab_extractor.group_by_domain(&tabs))
    }

    /// Get tabs grouped by domain from all connected browsers
    pub async fn get_all_tabs_by_domain(&self) -> HashMap<String, Vec<TabInfo>> {
        let all_tabs = self.get_all_tabs().await;
        let mut grouped: HashMap<String, Vec<TabInfo>> = HashMap::new();
        
        for (_browser_type, tabs) in all_tabs {
            let domain_groups = self.tab_extractor.group_by_domain(&tabs);
            for (domain, domain_tabs) in domain_groups {
                grouped.entry(domain).or_default().extend(domain_tabs);
            }
        }
        
        grouped
    }

    /// Get tabs grouped by category from a specific browser
    pub async fn get_tabs_by_category(&self, browser_type: BrowserType) -> Result<HashMap<TabCategory, Vec<TabInfo>>> {
        let tabs = self.get_tabs(browser_type).await?;
        Ok(self.tab_extractor.group_by_category(&tabs))
    }

    /// Get tabs grouped by category from all connected browsers
    pub async fn get_all_tabs_by_category(&self) -> HashMap<TabCategory, Vec<TabInfo>> {
        let all_tabs = self.get_all_tabs().await;
        let mut grouped: HashMap<TabCategory, Vec<TabInfo>> = HashMap::new();
        
        for (_browser_type, tabs) in all_tabs {
            let category_groups = self.tab_extractor.group_by_category(&tabs);
            for (category, category_tabs) in category_groups {
                grouped.entry(category).or_default().extend(category_tabs);
            }
        }
        
        grouped
    }

    /// Get statistics about tabs from a specific browser
    pub async fn get_tab_stats(&self, browser_type: BrowserType) -> Result<TabStats> {
        let tabs = self.get_tabs(browser_type).await?;
        Ok(self.tab_extractor.get_tab_stats(&tabs))
    }

    /// Get statistics about tabs from all connected browsers
    pub async fn get_all_tab_stats(&self) -> TabStats {
        let all_tabs = self.get_all_tabs().await;
        let all_tabs_flat: Vec<TabInfo> = all_tabs.into_values().flatten().collect();
        self.tab_extractor.get_tab_stats(&all_tabs_flat)
    }

    /// Update the tab monitor with current tabs and detect changes
    /// 
    /// This method fetches tabs from all connected browsers and updates
    /// the tab monitor, returning any detected changes (new tabs, closed tabs,
    /// navigation events, etc.)
    pub async fn update_tab_monitor(&self) -> Vec<TabEvent> {
        let all_tabs = self.get_all_tabs().await;
        self.tab_monitor.update_tabs(all_tabs).await
    }

    /// Get filter statistics for tabs from a specific browser
    /// 
    /// This shows how many tabs were filtered out by the privacy filter
    /// and the reasons for filtering.
    pub async fn get_filter_stats(&self, browser_type: BrowserType) -> Result<FilterStats> {
        let connections = self.connections.read().await;
        
        let connector = connections.get(&browser_type).ok_or_else(|| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: browser_type,
                },
            }
        })?;
        
        let all_tabs = connector.get_tabs().await?;
        Ok(self.privacy_filter.get_filter_stats(&all_tabs))
    }

    /// Get recently closed tabs from the tab monitor
    /// 
    /// Returns tabs that were closed within the specified time window.
    pub async fn get_recently_closed_tabs(&self, within_minutes: i64) -> Vec<TabInfo> {
        self.tab_monitor.get_recently_closed_tabs(within_minutes).await
    }

    /// Get the current monitored tabs
    pub async fn get_monitored_tabs(&self) -> Vec<TabInfo> {
        self.tab_monitor.get_current_tabs().await
    }

    /// Get recent tab events from the monitor
    pub async fn get_recent_tab_events(&self, count: usize) -> Vec<TabEvent> {
        self.tab_monitor.get_recent_events(count).await
    }

    /// Get tab monitor statistics
    pub async fn get_monitor_stats(&self) -> TabMonitorStats {
        self.tab_monitor.get_stats().await
    }

    // ============================================================
    // Bookmark Import Methods
    // ============================================================

    /// Create a new bookmark importer for detecting and importing bookmarks
    /// 
    /// This implements Requirement 2.1: Auto-detect bookmarks from all installed browsers
    pub fn create_bookmark_importer(&self) -> BookmarkImporter {
        BookmarkImporter::new()
    }

    /// Create a new bookmark validator for checking bookmark accessibility
    /// 
    /// This implements Requirement 2.2: Validate bookmark accessibility
    pub fn create_bookmark_validator(&self) -> BookmarkValidator {
        BookmarkValidator::new()
    }

    /// Create a bookmark validator with custom timeout
    pub fn create_bookmark_validator_with_timeout(&self, timeout_secs: u64) -> BookmarkValidator {
        BookmarkValidator::with_timeout(timeout_secs)
    }

    /// Import bookmarks from all detected browser sources
    /// 
    /// This is a convenience method that creates an importer, detects sources,
    /// and imports all bookmarks in one call.
    pub async fn import_all_bookmarks(&self) -> Result<HashMap<BrowserType, Vec<BookmarkInfo>>> {
        let mut importer = BookmarkImporter::new();
        importer.detect_bookmark_sources().await?;
        importer.import_all().await
    }

    /// Validate a batch of bookmarks and generate a report
    /// 
    /// This implements Requirement 2.2: Generate status reports for bookmark validation
    pub async fn validate_bookmarks(&self, bookmarks: &[BookmarkInfo]) -> ValidationReport {
        let validator = BookmarkValidator::new();
        validator.validate_batch(bookmarks).await
    }

    // ============================================================
    // Bookmark Content Analysis Methods
    // ============================================================

    /// Create a new bookmark content analyzer for fetching and analyzing bookmark content
    /// 
    /// This implements Requirements 2.2 and 2.3:
    /// - Validate bookmark accessibility
    /// - Extract page content and metadata
    pub fn create_bookmark_content_analyzer(&self) -> BookmarkContentAnalyzer {
        BookmarkContentAnalyzer::new()
    }

    /// Create a bookmark content analyzer with custom configuration
    pub fn create_bookmark_content_analyzer_with_config(
        &self,
        config: BookmarkContentAnalyzerConfig,
    ) -> BookmarkContentAnalyzer {
        BookmarkContentAnalyzer::with_config(config)
    }

    /// Fetch content for a single bookmark
    /// 
    /// This method fetches the web page content, validates accessibility,
    /// and extracts metadata from the page.
    /// 
    /// Implements Requirements 2.2 and 2.3
    pub async fn fetch_bookmark_content(&self, bookmark: &BookmarkInfo) -> BookmarkContentResult {
        let analyzer = BookmarkContentAnalyzer::new();
        analyzer.fetch_bookmark_content(bookmark).await
    }

    /// Fetch content for multiple bookmarks in batch
    /// 
    /// This method processes bookmarks concurrently for efficient batch processing.
    /// 
    /// Implements Requirements 2.2 and 2.3
    pub async fn fetch_bookmark_content_batch(&self, bookmarks: &[BookmarkInfo]) -> BatchAnalysisResult {
        let analyzer = BookmarkContentAnalyzer::new();
        analyzer.fetch_batch(bookmarks).await
    }

    /// Validate bookmark accessibility without fetching full content
    /// 
    /// This is a lightweight check that only performs a HEAD request
    /// to verify the URL is accessible.
    /// 
    /// Implements Requirement 2.2
    pub async fn validate_bookmark_accessibility(&self, url: &str) -> (AccessibilityStatus, Option<String>) {
        let analyzer = BookmarkContentAnalyzer::new();
        analyzer.validate_accessibility(url).await
    }
}

impl Default for BrowserConnectorManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = BrowserConnectorManager::new();
        assert_eq!(manager.connected_count().await, 0);
    }

    #[tokio::test]
    async fn test_get_connected_browsers_empty() {
        let manager = BrowserConnectorManager::new();
        let connected = manager.get_connected_browsers().await;
        assert!(connected.is_empty());
    }

    #[tokio::test]
    async fn test_connection_status_default() {
        let manager = BrowserConnectorManager::new();
        let status = manager.get_connection_status(BrowserType::Chrome).await;
        assert_eq!(status, ConnectionStatus::Disconnected);
    }
}
