//! Browser Connector module for Web Page Manager
//! 
//! This module provides functionality to connect to and communicate with
//! multiple browsers using their respective APIs (CDP for Chrome/Edge,
//! WebExtensions for Firefox).

pub mod traits;
pub mod cdp;
pub mod firefox;
pub mod privacy_filter;

pub use traits::*;

use web_page_manager_core::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Browser connector manager that handles multiple browser connections
pub struct BrowserConnectorManager {
    connections: Arc<RwLock<HashMap<BrowserType, Box<dyn BrowserConnector>>>>,
    privacy_filter: privacy_filter::PrivacyModeFilter,
}

impl BrowserConnectorManager {
    /// Create a new browser connector manager
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            privacy_filter: privacy_filter::PrivacyModeFilter::new(),
        }
    }

    /// Detect all running browsers
    pub async fn detect_browsers(&self) -> Result<Vec<BrowserInstance>> {
        let mut browsers = Vec::new();
        
        // Detect Chrome
        if let Ok(chrome) = cdp::ChromeConnector::detect().await {
            browsers.push(chrome);
        }
        
        // Detect Edge
        if let Ok(edge) = cdp::EdgeConnector::detect().await {
            browsers.push(edge);
        }
        
        // Detect Firefox
        if let Ok(firefox) = firefox::FirefoxConnector::detect().await {
            browsers.push(firefox);
        }
        
        Ok(browsers)
    }

    /// Connect to a specific browser
    pub async fn connect(&self, browser_type: BrowserType) -> Result<()> {
        let connector: Box<dyn BrowserConnector> = match browser_type {
            BrowserType::Chrome => Box::new(cdp::ChromeConnector::new()),
            BrowserType::Edge => Box::new(cdp::EdgeConnector::new()),
            BrowserType::Firefox => Box::new(firefox::FirefoxConnector::new()),
            BrowserType::Safari => {
                return Err(WebPageManagerError::BrowserConnection {
                    source: BrowserConnectionError::BrowserNotRunning {
                        browser: BrowserType::Safari,
                    },
                });
            }
        };
        
        connector.connect().await?;
        
        let mut connections = self.connections.write().await;
        connections.insert(browser_type, connector);
        
        Ok(())
    }

    /// Get tabs from a connected browser (filtered for privacy mode)
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

    /// Disconnect from a browser
    pub async fn disconnect(&self, browser_type: BrowserType) -> Result<()> {
        let mut connections = self.connections.write().await;
        
        if let Some(connector) = connections.remove(&browser_type) {
            connector.disconnect().await?;
        }
        
        Ok(())
    }

    /// Disconnect from all browsers
    pub async fn disconnect_all(&self) -> Result<()> {
        let mut connections = self.connections.write().await;
        
        for (_, connector) in connections.drain() {
            let _ = connector.disconnect().await;
        }
        
        Ok(())
    }
}

impl Default for BrowserConnectorManager {
    fn default() -> Self {
        Self::new()
    }
}
