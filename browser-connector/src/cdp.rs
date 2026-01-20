//! Chrome DevTools Protocol (CDP) connector for Chrome and Edge browsers
//!
//! This module implements browser detection and connection functionality for
//! Chromium-based browsers (Chrome and Edge) using the Chrome DevTools Protocol.

use crate::traits::BrowserConnector;
use web_page_manager_core::*;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
// WebSocket imports reserved for future CDP WebSocket implementation
// use futures_util::{SinkExt, StreamExt};
// use tokio_tungstenite::{connect_async, tungstenite::Message};

/// CDP target information returned by the browser
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CdpTarget {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub target_type: String,
    pub title: String,
    pub url: String,
    pub web_socket_debugger_url: Option<String>,
    #[serde(default)]
    pub favicon_url: Option<String>,
    #[serde(default)]
    pub description: String,
}

/// CDP browser version information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CdpVersion {
    #[serde(rename = "Browser")]
    pub browser: String,
    #[serde(rename = "Protocol-Version")]
    pub protocol_version: String,
    #[serde(rename = "User-Agent")]
    pub user_agent: String,
    #[serde(rename = "V8-Version")]
    pub v8_version: Option<String>,
    #[serde(rename = "WebKit-Version")]
    pub webkit_version: Option<String>,
    #[serde(rename = "webSocketDebuggerUrl")]
    pub web_socket_debugger_url: Option<String>,
}

/// CDP command message (reserved for WebSocket-based CDP communication)
#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct CdpCommand {
    id: u64,
    method: String,
    params: serde_json::Value,
}

/// CDP response message (reserved for WebSocket-based CDP communication)
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CdpResponse {
    id: Option<u64>,
    result: Option<serde_json::Value>,
    error: Option<CdpError>,
}

/// CDP error information
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CdpError {
    code: i64,
    message: String,
}

/// Internal connection state for CDP
struct CdpConnectionState {
    connected: bool,
    targets: Vec<CdpTarget>,
    version: Option<CdpVersion>,
    ws_url: Option<String>,
}

impl Default for CdpConnectionState {
    fn default() -> Self {
        Self {
            connected: false,
            targets: Vec::new(),
            version: None,
            ws_url: None,
        }
    }
}

/// Chrome browser connector using CDP
pub struct ChromeConnector {
    state: Arc<RwLock<CdpConnectionState>>,
    debug_port: u16,
}

impl ChromeConnector {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(CdpConnectionState::default())),
            debug_port: 9222,
        }
    }

    pub fn with_port(port: u16) -> Self {
        Self {
            state: Arc::new(RwLock::new(CdpConnectionState::default())),
            debug_port: port,
        }
    }

    /// Detect running Chrome instance by checking the debug port
    pub async fn detect() -> Result<BrowserInstance> {
        Self::detect_on_port(9222).await
    }

    /// Detect Chrome on a specific port
    pub async fn detect_on_port(port: u16) -> Result<BrowserInstance> {
        let url = format!("http://localhost:{}/json/version", port);
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Network { details: e.to_string() },
            })?;
        
        let response = client.get(&url).send().await.map_err(|_e| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Chrome,
                },
            }
        })?;
        
        if !response.status().is_success() {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Chrome,
                },
            });
        }
        
        let version: CdpVersion = response.json().await.map_err(|_e| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::InvalidResponse {
                    browser: BrowserType::Chrome,
                },
            }
        })?;
        
        // Extract version number from browser string (e.g., "Chrome/120.0.6099.109")
        let browser_version = version.browser
            .split('/')
            .nth(1)
            .unwrap_or("unknown")
            .to_string();
        
        Ok(BrowserInstance {
            browser_type: BrowserType::Chrome,
            version: browser_version,
            process_id: 0, // Would need platform-specific code to get actual PID
            debug_port: Some(port),
            profile_path: None,
        })
    }

    /// Fetch all CDP targets (tabs, extensions, etc.)
    async fn fetch_targets(&self) -> Result<Vec<CdpTarget>> {
        let url = format!("http://localhost:{}/json/list", self.debug_port);
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Network { details: e.to_string() },
            })?;
        
        let response = client.get(&url).send().await.map_err(|_e| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::ConnectionTimeout {
                    browser: BrowserType::Chrome,
                },
            }
        })?;
        
        if !response.status().is_success() {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::InvalidResponse {
                    browser: BrowserType::Chrome,
                },
            });
        }
        
        let targets: Vec<CdpTarget> = response.json().await.map_err(|_e| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::InvalidResponse {
                    browser: BrowserType::Chrome,
                },
            }
        })?;
        
        Ok(targets)
    }

    /// Convert CDP target to TabInfo
    fn target_to_tab_info(&self, target: &CdpTarget, is_private: bool) -> TabInfo {
        TabInfo {
            id: TabId(target.id.clone()),
            url: target.url.clone(),
            title: target.title.clone(),
            favicon_url: target.favicon_url.clone(),
            browser_type: BrowserType::Chrome,
            is_private,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
        }
    }
}

impl Default for ChromeConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BrowserConnector for ChromeConnector {
    fn browser_type(&self) -> BrowserType {
        BrowserType::Chrome
    }

    async fn connect(&self) -> Result<()> {
        tracing::info!("Connecting to Chrome on port {}", self.debug_port);
        
        // Verify browser is running and get version info
        let version_url = format!("http://localhost:{}/json/version", self.debug_port);
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Network { details: e.to_string() },
            })?;
        
        let response = client.get(&version_url).send().await.map_err(|_| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Chrome,
                },
            }
        })?;
        
        if !response.status().is_success() {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Chrome,
                },
            });
        }
        
        let version: CdpVersion = response.json().await.map_err(|_| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::InvalidResponse {
                    browser: BrowserType::Chrome,
                },
            }
        })?;
        
        // Fetch initial targets
        let targets = self.fetch_targets().await?;
        
        // Update state
        let mut state = self.state.write().await;
        state.connected = true;
        state.version = Some(version.clone());
        state.ws_url = version.web_socket_debugger_url;
        state.targets = targets;
        
        tracing::info!("Connected to Chrome: {}", version.browser);
        Ok(())
    }

    async fn disconnect(&self) -> Result<()> {
        tracing::info!("Disconnecting from Chrome");
        
        let mut state = self.state.write().await;
        state.connected = false;
        state.targets.clear();
        state.version = None;
        state.ws_url = None;
        
        Ok(())
    }

    fn is_connected(&self) -> bool {
        // Use try_read to avoid blocking
        if let Ok(state) = self.state.try_read() {
            state.connected
        } else {
            false
        }
    }

    async fn get_tabs(&self) -> Result<Vec<TabInfo>> {
        let state = self.state.read().await;
        if !state.connected {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Chrome,
                },
            });
        }
        drop(state);

        // Refresh targets list
        let targets = self.fetch_targets().await?;
        
        // Update cached targets
        {
            let mut state = self.state.write().await;
            state.targets = targets.clone();
        }
        
        // Filter to only page targets and convert to TabInfo
        // Note: CDP doesn't directly expose incognito status, we detect it via URL patterns
        // or by checking if the target belongs to an incognito window (requires additional CDP calls)
        let tabs: Vec<TabInfo> = targets
            .iter()
            .filter(|t| t.target_type == "page")
            .map(|t| {
                // For now, assume all tabs are non-private
                // Full incognito detection requires Target.getTargetInfo with additional context
                self.target_to_tab_info(t, false)
            })
            .collect();
        
        Ok(tabs)
    }

    async fn get_bookmarks(&self) -> Result<Vec<BookmarkInfo>> {
        let state = self.state.read().await;
        if !state.connected {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Chrome,
                },
            });
        }
        drop(state);

        // Chrome bookmarks are stored in a JSON file in the profile directory
        // This requires knowing the profile path, which we can get from the browser
        // For now, return empty - full implementation would read from:
        // Windows: %LOCALAPPDATA%\Google\Chrome\User Data\Default\Bookmarks
        // Linux: ~/.config/google-chrome/Default/Bookmarks
        // macOS: ~/Library/Application Support/Google/Chrome/Default/Bookmarks
        
        Ok(Vec::new())
    }

    async fn fetch_page_content(&self, url: &str) -> Result<PageContent> {
        tracing::info!("Fetching page content: {}", url);
        
        let state = self.state.read().await;
        if !state.connected {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Chrome,
                },
            });
        }
        drop(state);
        
        // For page content fetching, we use a simple HTTP request
        // Full CDP implementation would use Page.navigate() and DOM.getDocument()
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Network { details: e.to_string() },
            })?;
        
        let response = client.get(url).send().await.map_err(|_e| {
            WebPageManagerError::AIProcessing {
                source: AIProcessingError::ContentFetchFailed { url: url.to_string() },
            }
        })?;
        
        let html = response.text().await.map_err(|_e| {
            WebPageManagerError::AIProcessing {
                source: AIProcessingError::ContentFetchFailed { url: url.to_string() },
            }
        })?;
        
        // Basic content extraction (full implementation would use proper HTML parsing)
        let title = extract_title(&html).unwrap_or_default();
        let description = extract_meta_description(&html);
        let text = extract_text_content(&html);
        
        Ok(PageContent {
            html,
            text,
            title,
            description,
            keywords: Vec::new(),
            images: Vec::new(),
            links: Vec::new(),
            extracted_at: Utc::now(),
        })
    }

    async fn close_tab(&self, tab_id: &TabId) -> Result<()> {
        tracing::info!("Closing Chrome tab: {:?}", tab_id);
        
        let state = self.state.read().await;
        if !state.connected {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Chrome,
                },
            });
        }
        drop(state);
        
        // Use CDP HTTP endpoint to close target
        let url = format!("http://localhost:{}/json/close/{}", self.debug_port, tab_id.0);
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Network { details: e.to_string() },
            })?;
        
        let response = client.get(&url).send().await.map_err(|_| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::ConnectionTimeout {
                    browser: BrowserType::Chrome,
                },
            }
        })?;
        
        if !response.status().is_success() {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::InvalidResponse {
                    browser: BrowserType::Chrome,
                },
            });
        }
        
        Ok(())
    }

    async fn activate_tab(&self, tab_id: &TabId) -> Result<()> {
        tracing::info!("Activating Chrome tab: {:?}", tab_id);
        
        let state = self.state.read().await;
        if !state.connected {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Chrome,
                },
            });
        }
        drop(state);
        
        // Use CDP HTTP endpoint to activate target
        let url = format!("http://localhost:{}/json/activate/{}", self.debug_port, tab_id.0);
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Network { details: e.to_string() },
            })?;
        
        let response = client.get(&url).send().await.map_err(|_| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::ConnectionTimeout {
                    browser: BrowserType::Chrome,
                },
            }
        })?;
        
        if !response.status().is_success() {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::InvalidResponse {
                    browser: BrowserType::Chrome,
                },
            });
        }
        
        Ok(())
    }

    async fn create_tab(&self, url: &str) -> Result<TabId> {
        tracing::info!("Creating Chrome tab: {}", url);
        
        let state = self.state.read().await;
        if !state.connected {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Chrome,
                },
            });
        }
        drop(state);
        
        // Use CDP HTTP endpoint to create new target
        let encoded_url = urlencoding::encode(url);
        let api_url = format!("http://localhost:{}/json/new?{}", self.debug_port, encoded_url);
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Network { details: e.to_string() },
            })?;
        
        let response = client.get(&api_url).send().await.map_err(|_| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::ConnectionTimeout {
                    browser: BrowserType::Chrome,
                },
            }
        })?;
        
        if !response.status().is_success() {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::InvalidResponse {
                    browser: BrowserType::Chrome,
                },
            });
        }
        
        let target: CdpTarget = response.json().await.map_err(|_| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::InvalidResponse {
                    browser: BrowserType::Chrome,
                },
            }
        })?;
        
        Ok(TabId(target.id))
    }
}

/// Edge browser connector using CDP (Edge is Chromium-based)
pub struct EdgeConnector {
    state: Arc<RwLock<CdpConnectionState>>,
    debug_port: u16,
}

impl EdgeConnector {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(CdpConnectionState::default())),
            debug_port: 9223, // Different default port from Chrome
        }
    }

    pub fn with_port(port: u16) -> Self {
        Self {
            state: Arc::new(RwLock::new(CdpConnectionState::default())),
            debug_port: port,
        }
    }

    /// Detect running Edge instance
    pub async fn detect() -> Result<BrowserInstance> {
        Self::detect_on_port(9223).await
    }

    /// Detect Edge on a specific port
    pub async fn detect_on_port(port: u16) -> Result<BrowserInstance> {
        let url = format!("http://localhost:{}/json/version", port);
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Network { details: e.to_string() },
            })?;
        
        let response = client.get(&url).send().await.map_err(|_| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Edge,
                },
            }
        })?;
        
        if !response.status().is_success() {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Edge,
                },
            });
        }
        
        let version: CdpVersion = response.json().await.map_err(|_| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::InvalidResponse {
                    browser: BrowserType::Edge,
                },
            }
        })?;
        
        // Extract version number from browser string (e.g., "Edg/120.0.2210.91")
        let browser_version = version.browser
            .split('/')
            .nth(1)
            .unwrap_or("unknown")
            .to_string();
        
        Ok(BrowserInstance {
            browser_type: BrowserType::Edge,
            version: browser_version,
            process_id: 0,
            debug_port: Some(port),
            profile_path: None,
        })
    }

    /// Fetch all CDP targets
    async fn fetch_targets(&self) -> Result<Vec<CdpTarget>> {
        let url = format!("http://localhost:{}/json/list", self.debug_port);
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Network { details: e.to_string() },
            })?;
        
        let response = client.get(&url).send().await.map_err(|_| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::ConnectionTimeout {
                    browser: BrowserType::Edge,
                },
            }
        })?;
        
        if !response.status().is_success() {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::InvalidResponse {
                    browser: BrowserType::Edge,
                },
            });
        }
        
        let targets: Vec<CdpTarget> = response.json().await.map_err(|_| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::InvalidResponse {
                    browser: BrowserType::Edge,
                },
            }
        })?;
        
        Ok(targets)
    }

    /// Convert CDP target to TabInfo
    fn target_to_tab_info(&self, target: &CdpTarget, is_private: bool) -> TabInfo {
        TabInfo {
            id: TabId(target.id.clone()),
            url: target.url.clone(),
            title: target.title.clone(),
            favicon_url: target.favicon_url.clone(),
            browser_type: BrowserType::Edge,
            is_private,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
        }
    }
}

impl Default for EdgeConnector {
    fn default() -> Self {
        Self::new()
    }
}


#[async_trait]
impl BrowserConnector for EdgeConnector {
    fn browser_type(&self) -> BrowserType {
        BrowserType::Edge
    }

    async fn connect(&self) -> Result<()> {
        tracing::info!("Connecting to Edge on port {}", self.debug_port);
        
        let version_url = format!("http://localhost:{}/json/version", self.debug_port);
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Network { details: e.to_string() },
            })?;
        
        let response = client.get(&version_url).send().await.map_err(|_| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Edge,
                },
            }
        })?;
        
        if !response.status().is_success() {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Edge,
                },
            });
        }
        
        let version: CdpVersion = response.json().await.map_err(|_| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::InvalidResponse {
                    browser: BrowserType::Edge,
                },
            }
        })?;
        
        let targets = self.fetch_targets().await?;
        
        let mut state = self.state.write().await;
        state.connected = true;
        state.version = Some(version.clone());
        state.ws_url = version.web_socket_debugger_url;
        state.targets = targets;
        
        tracing::info!("Connected to Edge: {}", version.browser);
        Ok(())
    }

    async fn disconnect(&self) -> Result<()> {
        tracing::info!("Disconnecting from Edge");
        
        let mut state = self.state.write().await;
        state.connected = false;
        state.targets.clear();
        state.version = None;
        state.ws_url = None;
        
        Ok(())
    }

    fn is_connected(&self) -> bool {
        if let Ok(state) = self.state.try_read() {
            state.connected
        } else {
            false
        }
    }

    async fn get_tabs(&self) -> Result<Vec<TabInfo>> {
        let state = self.state.read().await;
        if !state.connected {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Edge,
                },
            });
        }
        drop(state);

        let targets = self.fetch_targets().await?;
        
        {
            let mut state = self.state.write().await;
            state.targets = targets.clone();
        }
        
        let tabs: Vec<TabInfo> = targets
            .iter()
            .filter(|t| t.target_type == "page")
            .map(|t| self.target_to_tab_info(t, false))
            .collect();
        
        Ok(tabs)
    }

    async fn get_bookmarks(&self) -> Result<Vec<BookmarkInfo>> {
        let state = self.state.read().await;
        if !state.connected {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Edge,
                },
            });
        }
        drop(state);

        // Edge bookmarks are stored similarly to Chrome
        // Windows: %LOCALAPPDATA%\Microsoft\Edge\User Data\Default\Bookmarks
        Ok(Vec::new())
    }

    async fn fetch_page_content(&self, url: &str) -> Result<PageContent> {
        tracing::info!("Fetching page content from Edge: {}", url);
        
        let state = self.state.read().await;
        if !state.connected {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Edge,
                },
            });
        }
        drop(state);
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Network { details: e.to_string() },
            })?;
        
        let response = client.get(url).send().await.map_err(|_| {
            WebPageManagerError::AIProcessing {
                source: AIProcessingError::ContentFetchFailed { url: url.to_string() },
            }
        })?;
        
        let html = response.text().await.map_err(|_| {
            WebPageManagerError::AIProcessing {
                source: AIProcessingError::ContentFetchFailed { url: url.to_string() },
            }
        })?;
        
        let title = extract_title(&html).unwrap_or_default();
        let description = extract_meta_description(&html);
        let text = extract_text_content(&html);
        
        Ok(PageContent {
            html,
            text,
            title,
            description,
            keywords: Vec::new(),
            images: Vec::new(),
            links: Vec::new(),
            extracted_at: Utc::now(),
        })
    }

    async fn close_tab(&self, tab_id: &TabId) -> Result<()> {
        tracing::info!("Closing Edge tab: {:?}", tab_id);
        
        let state = self.state.read().await;
        if !state.connected {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Edge,
                },
            });
        }
        drop(state);
        
        let url = format!("http://localhost:{}/json/close/{}", self.debug_port, tab_id.0);
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Network { details: e.to_string() },
            })?;
        
        let response = client.get(&url).send().await.map_err(|_| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::ConnectionTimeout {
                    browser: BrowserType::Edge,
                },
            }
        })?;
        
        if !response.status().is_success() {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::InvalidResponse {
                    browser: BrowserType::Edge,
                },
            });
        }
        
        Ok(())
    }

    async fn activate_tab(&self, tab_id: &TabId) -> Result<()> {
        tracing::info!("Activating Edge tab: {:?}", tab_id);
        
        let state = self.state.read().await;
        if !state.connected {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Edge,
                },
            });
        }
        drop(state);
        
        let url = format!("http://localhost:{}/json/activate/{}", self.debug_port, tab_id.0);
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Network { details: e.to_string() },
            })?;
        
        let response = client.get(&url).send().await.map_err(|_| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::ConnectionTimeout {
                    browser: BrowserType::Edge,
                },
            }
        })?;
        
        if !response.status().is_success() {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::InvalidResponse {
                    browser: BrowserType::Edge,
                },
            });
        }
        
        Ok(())
    }

    async fn create_tab(&self, url: &str) -> Result<TabId> {
        tracing::info!("Creating Edge tab: {}", url);
        
        let state = self.state.read().await;
        if !state.connected {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Edge,
                },
            });
        }
        drop(state);
        
        let encoded_url = urlencoding::encode(url);
        let api_url = format!("http://localhost:{}/json/new?{}", self.debug_port, encoded_url);
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| WebPageManagerError::System {
                source: SystemError::Network { details: e.to_string() },
            })?;
        
        let response = client.get(&api_url).send().await.map_err(|_| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::ConnectionTimeout {
                    browser: BrowserType::Edge,
                },
            }
        })?;
        
        if !response.status().is_success() {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::InvalidResponse {
                    browser: BrowserType::Edge,
                },
            });
        }
        
        let target: CdpTarget = response.json().await.map_err(|_| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::InvalidResponse {
                    browser: BrowserType::Edge,
                },
            }
        })?;
        
        Ok(TabId(target.id))
    }
}

// Helper functions for basic HTML content extraction

/// Extract title from HTML
fn extract_title(html: &str) -> Option<String> {
    let title_start = html.find("<title>")?;
    let title_end = html.find("</title>")?;
    
    if title_start < title_end {
        let title = &html[title_start + 7..title_end];
        Some(title.trim().to_string())
    } else {
        None
    }
}

/// Extract meta description from HTML
fn extract_meta_description(html: &str) -> Option<String> {
    // Look for <meta name="description" content="...">
    let lower_html = html.to_lowercase();
    
    if let Some(meta_pos) = lower_html.find("name=\"description\"") {
        // Find the content attribute
        let search_start = meta_pos.saturating_sub(100);
        let search_end = (meta_pos + 200).min(html.len());
        let search_area = &html[search_start..search_end];
        
        if let Some(content_pos) = search_area.to_lowercase().find("content=\"") {
            let content_start = search_start + content_pos + 9;
            if let Some(content_end) = html[content_start..].find('"') {
                return Some(html[content_start..content_start + content_end].to_string());
            }
        }
    }
    
    None
}

/// Extract plain text content from HTML (basic implementation)
fn extract_text_content(html: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    
    let lower_html = html.to_lowercase();
    let chars: Vec<char> = html.chars().collect();
    let lower_chars: Vec<char> = lower_html.chars().collect();
    
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        
        if c == '<' {
            in_tag = true;
            
            // Check for script/style tags
            let remaining: String = lower_chars[i..].iter().take(10).collect();
            if remaining.starts_with("<script") {
                in_script = true;
            } else if remaining.starts_with("</script") {
                in_script = false;
            } else if remaining.starts_with("<style") {
                in_style = true;
            } else if remaining.starts_with("</style") {
                in_style = false;
            }
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag && !in_script && !in_style {
            if c.is_whitespace() {
                if !text.ends_with(' ') && !text.is_empty() {
                    text.push(' ');
                }
            } else {
                text.push(c);
            }
        }
        
        i += 1;
    }
    
    text.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title() {
        let html = "<html><head><title>Test Page</title></head></html>";
        assert_eq!(extract_title(html), Some("Test Page".to_string()));
        
        let html_no_title = "<html><head></head></html>";
        assert_eq!(extract_title(html_no_title), None);
    }

    #[test]
    fn test_extract_meta_description() {
        let html = r#"<html><head><meta name="description" content="This is a test description"></head></html>"#;
        assert_eq!(extract_meta_description(html), Some("This is a test description".to_string()));
    }

    #[test]
    fn test_extract_text_content() {
        let html = "<html><body><p>Hello World</p><script>var x = 1;</script></body></html>";
        let text = extract_text_content(html);
        assert!(text.contains("Hello World"));
        assert!(!text.contains("var x"));
    }
}
