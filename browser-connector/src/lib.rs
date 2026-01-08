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

pub mod traits;
pub mod cdp;
pub mod firefox;
pub mod privacy_filter;

pub use traits::*;
pub use cdp::{ChromeConnector, EdgeConnector, CdpTarget, CdpVersion};
pub use firefox::FirefoxConnector;
pub use privacy_filter::PrivacyModeFilter;

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
}

impl BrowserConnectorManager {
    /// Create a new browser connector manager
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            instances: Arc::new(RwLock::new(HashMap::new())),
            privacy_filter: PrivacyModeFilter::new(),
        }
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
