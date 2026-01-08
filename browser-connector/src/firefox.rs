//! Firefox browser connector using WebExtensions API and Native Messaging
//!
//! This module implements browser detection and connection functionality for
//! Firefox using the WebExtensions Native Messaging protocol.

use crate::traits::BrowserConnector;
use web_page_manager_core::*;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Native messaging manifest for Firefox extension communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeMessagingManifest {
    pub name: String,
    pub description: String,
    pub path: String,
    #[serde(rename = "type")]
    pub manifest_type: String,
    pub allowed_extensions: Vec<String>,
}

/// Message sent to/from the Firefox extension
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExtensionMessage {
    #[serde(rename = "getTabs")]
    GetTabs,
    #[serde(rename = "getBookmarks")]
    GetBookmarks,
    #[serde(rename = "closeTab")]
    CloseTab { tab_id: i64 },
    #[serde(rename = "activateTab")]
    ActivateTab { tab_id: i64 },
    #[serde(rename = "createTab")]
    CreateTab { url: String },
    #[serde(rename = "getPageContent")]
    GetPageContent { tab_id: i64 },
}

/// Response from the Firefox extension
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExtensionResponse {
    #[serde(rename = "tabs")]
    Tabs { tabs: Vec<FirefoxTab> },
    #[serde(rename = "bookmarks")]
    Bookmarks { bookmarks: Vec<FirefoxBookmark> },
    #[serde(rename = "tabClosed")]
    TabClosed { success: bool },
    #[serde(rename = "tabActivated")]
    TabActivated { success: bool },
    #[serde(rename = "tabCreated")]
    TabCreated { tab_id: i64 },
    #[serde(rename = "pageContent")]
    PageContent { content: String, title: String },
    #[serde(rename = "error")]
    Error { message: String },
}

/// Firefox tab information from the extension
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirefoxTab {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub fav_icon_url: Option<String>,
    pub incognito: bool,
    pub window_id: i64,
    pub index: i32,
    pub active: bool,
    pub pinned: bool,
}

/// Firefox bookmark information from the extension
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirefoxBookmark {
    pub id: String,
    pub url: Option<String>,
    pub title: String,
    pub parent_id: Option<String>,
    #[serde(rename = "type")]
    pub bookmark_type: String,
    pub date_added: Option<i64>,
}

/// Internal connection state for Firefox
struct FirefoxConnectionState {
    connected: bool,
    version: Option<String>,
    profile_path: Option<PathBuf>,
    extension_installed: bool,
}

impl Default for FirefoxConnectionState {
    fn default() -> Self {
        Self {
            connected: false,
            version: None,
            profile_path: None,
            extension_installed: false,
        }
    }
}

/// Firefox browser connector using WebExtensions Native Messaging
pub struct FirefoxConnector {
    state: Arc<RwLock<FirefoxConnectionState>>,
}

impl FirefoxConnector {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(FirefoxConnectionState::default())),
        }
    }

    /// Detect running Firefox instance
    pub async fn detect() -> Result<BrowserInstance> {
        // Check for Firefox profile directory to detect installation
        let profile_path = Self::find_firefox_profile()?;
        
        // Try to get Firefox version from the profile or installation
        let version = Self::get_firefox_version().unwrap_or_else(|| "unknown".to_string());
        
        Ok(BrowserInstance {
            browser_type: BrowserType::Firefox,
            version,
            process_id: 0,
            debug_port: None,
            profile_path: profile_path.map(|p| p.to_string_lossy().to_string()),
        })
    }

    /// Find Firefox profile directory
    fn find_firefox_profile() -> Result<Option<PathBuf>> {
        let profile_base = Self::get_firefox_profile_base();
        
        if let Some(base) = profile_base {
            if base.exists() {
                // Look for default profile
                if let Ok(entries) = std::fs::read_dir(&base) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            let name = path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("");
                            if name.ends_with(".default") || name.ends_with(".default-release") {
                                return Ok(Some(path));
                            }
                        }
                    }
                }
                return Ok(Some(base));
            }
        }
        
        Ok(None)
    }

    /// Get Firefox profile base directory based on platform
    fn get_firefox_profile_base() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            dirs::data_local_dir().map(|p| p.join("Mozilla").join("Firefox").join("Profiles"))
        }
        
        #[cfg(target_os = "linux")]
        {
            dirs::home_dir().map(|p| p.join(".mozilla").join("firefox"))
        }
        
        #[cfg(target_os = "macos")]
        {
            dirs::home_dir().map(|p| {
                p.join("Library")
                    .join("Application Support")
                    .join("Firefox")
                    .join("Profiles")
            })
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            None
        }
    }

    /// Get Firefox version from installation
    fn get_firefox_version() -> Option<String> {
        // This would typically read from Firefox's application.ini or use platform-specific methods
        // For now, return a placeholder
        Some("121.0".to_string())
    }

    /// Check if the native messaging host is registered
    fn check_native_messaging_host() -> bool {
        let manifest_path = Self::get_native_messaging_manifest_path();
        
        if let Some(path) = manifest_path {
            path.exists()
        } else {
            false
        }
    }

    /// Get the path where native messaging manifest should be located
    fn get_native_messaging_manifest_path() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            // On Windows, native messaging hosts are registered in the registry
            // The manifest file location is specified in the registry
            dirs::data_local_dir().map(|p| {
                p.join("Mozilla")
                    .join("NativeMessagingHosts")
                    .join("web_page_manager.json")
            })
        }
        
        #[cfg(target_os = "linux")]
        {
            dirs::home_dir().map(|p| {
                p.join(".mozilla")
                    .join("native-messaging-hosts")
                    .join("web_page_manager.json")
            })
        }
        
        #[cfg(target_os = "macos")]
        {
            dirs::home_dir().map(|p| {
                p.join("Library")
                    .join("Application Support")
                    .join("Mozilla")
                    .join("NativeMessagingHosts")
                    .join("web_page_manager.json")
            })
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            None
        }
    }

    /// Convert Firefox tab to TabInfo
    #[allow(dead_code)]
    fn firefox_tab_to_tab_info(&self, tab: &FirefoxTab) -> TabInfo {
        TabInfo {
            id: TabId(tab.id.to_string()),
            url: tab.url.clone(),
            title: tab.title.clone(),
            favicon_url: tab.fav_icon_url.clone(),
            browser_type: BrowserType::Firefox,
            is_private: tab.incognito,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
        }
    }

    /// Convert Firefox bookmark to BookmarkInfo
    #[allow(dead_code)]
    fn firefox_bookmark_to_bookmark_info(&self, bookmark: &FirefoxBookmark, folder_path: Vec<String>) -> Option<BookmarkInfo> {
        // Only convert actual bookmarks (not folders)
        if bookmark.bookmark_type != "bookmark" || bookmark.url.is_none() {
            return None;
        }
        
        let created_at = bookmark.date_added
            .map(|ts| {
                chrono::DateTime::from_timestamp_millis(ts)
                    .unwrap_or_else(Utc::now)
            })
            .unwrap_or_else(Utc::now);
        
        Some(BookmarkInfo {
            id: BookmarkId(bookmark.id.clone()),
            url: bookmark.url.clone().unwrap_or_default(),
            title: bookmark.title.clone(),
            favicon_url: None,
            browser_type: BrowserType::Firefox,
            folder_path,
            created_at,
            last_accessed: None,
        })
    }
}

impl Default for FirefoxConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BrowserConnector for FirefoxConnector {
    fn browser_type(&self) -> BrowserType {
        BrowserType::Firefox
    }

    async fn connect(&self) -> Result<()> {
        tracing::info!("Connecting to Firefox via Native Messaging");
        
        // Check if Firefox profile exists
        let profile_path = Self::find_firefox_profile()?;
        
        if profile_path.is_none() {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Firefox,
                },
            });
        }
        
        // Check if native messaging host is registered
        let extension_installed = Self::check_native_messaging_host();
        
        // Get Firefox version
        let version = Self::get_firefox_version();
        
        // Update state
        let mut state = self.state.write().await;
        state.connected = true;
        state.version = version;
        state.profile_path = profile_path;
        state.extension_installed = extension_installed;
        
        if !extension_installed {
            tracing::warn!("Firefox native messaging host not found. Some features may be limited.");
        }
        
        tracing::info!("Connected to Firefox");
        Ok(())
    }

    async fn disconnect(&self) -> Result<()> {
        tracing::info!("Disconnecting from Firefox");
        
        let mut state = self.state.write().await;
        state.connected = false;
        state.version = None;
        state.profile_path = None;
        state.extension_installed = false;
        
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
                    browser: BrowserType::Firefox,
                },
            });
        }
        
        if !state.extension_installed {
            tracing::warn!("Firefox extension not installed, cannot retrieve tabs");
            return Ok(Vec::new());
        }
        drop(state);

        // In a full implementation, this would communicate with the Firefox extension
        // via native messaging to get the current tabs
        // For now, return empty as we need the extension to be running
        
        Ok(Vec::new())
    }

    async fn get_bookmarks(&self) -> Result<Vec<BookmarkInfo>> {
        let state = self.state.read().await;
        if !state.connected {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Firefox,
                },
            });
        }
        
        let profile_path = state.profile_path.clone();
        drop(state);

        // Firefox bookmarks are stored in places.sqlite in the profile directory
        // This requires reading from the SQLite database
        if let Some(profile) = profile_path {
            let places_db = profile.join("places.sqlite");
            if places_db.exists() {
                // In a full implementation, we would read from the SQLite database
                // For now, return empty as direct database access requires careful handling
                tracing::info!("Found Firefox places database at {:?}", places_db);
            }
        }
        
        Ok(Vec::new())
    }

    async fn fetch_page_content(&self, url: &str) -> Result<PageContent> {
        tracing::info!("Fetching page content from Firefox: {}", url);
        
        let state = self.state.read().await;
        if !state.connected {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Firefox,
                },
            });
        }
        drop(state);
        
        // Use HTTP request to fetch content (same as Chrome implementation)
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
        
        // Basic content extraction
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
        tracing::info!("Closing Firefox tab: {:?}", tab_id);
        
        let state = self.state.read().await;
        if !state.connected {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Firefox,
                },
            });
        }
        
        if !state.extension_installed {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::PermissionDenied {
                    browser: BrowserType::Firefox,
                },
            });
        }
        drop(state);
        
        // In a full implementation, this would send a message to the Firefox extension
        // via native messaging to close the tab
        
        Ok(())
    }

    async fn activate_tab(&self, tab_id: &TabId) -> Result<()> {
        tracing::info!("Activating Firefox tab: {:?}", tab_id);
        
        let state = self.state.read().await;
        if !state.connected {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Firefox,
                },
            });
        }
        
        if !state.extension_installed {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::PermissionDenied {
                    browser: BrowserType::Firefox,
                },
            });
        }
        drop(state);
        
        // In a full implementation, this would send a message to the Firefox extension
        
        Ok(())
    }

    async fn create_tab(&self, url: &str) -> Result<TabId> {
        tracing::info!("Creating Firefox tab: {}", url);
        
        let state = self.state.read().await;
        if !state.connected {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Firefox,
                },
            });
        }
        
        if !state.extension_installed {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::PermissionDenied {
                    browser: BrowserType::Firefox,
                },
            });
        }
        drop(state);
        
        // In a full implementation, this would send a message to the Firefox extension
        
        Ok(TabId::new())
    }
}

// Helper functions for basic HTML content extraction (shared with CDP module)

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
    let lower_html = html.to_lowercase();
    
    if let Some(meta_pos) = lower_html.find("name=\"description\"") {
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

/// Extract plain text content from HTML
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
    fn test_firefox_connector_creation() {
        let connector = FirefoxConnector::new();
        assert_eq!(connector.browser_type(), BrowserType::Firefox);
        assert!(!connector.is_connected());
    }

    #[test]
    fn test_extract_title() {
        let html = "<html><head><title>Firefox Test</title></head></html>";
        assert_eq!(extract_title(html), Some("Firefox Test".to_string()));
    }

    #[test]
    fn test_firefox_tab_conversion() {
        let connector = FirefoxConnector::new();
        let firefox_tab = FirefoxTab {
            id: 123,
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
            fav_icon_url: Some("https://example.com/favicon.ico".to_string()),
            incognito: false,
            window_id: 1,
            index: 0,
            active: true,
            pinned: false,
        };
        
        let tab_info = connector.firefox_tab_to_tab_info(&firefox_tab);
        
        assert_eq!(tab_info.id.0, "123");
        assert_eq!(tab_info.url, "https://example.com");
        assert_eq!(tab_info.title, "Example");
        assert!(!tab_info.is_private);
        assert_eq!(tab_info.browser_type, BrowserType::Firefox);
    }

    #[test]
    fn test_firefox_bookmark_conversion() {
        let connector = FirefoxConnector::new();
        let firefox_bookmark = FirefoxBookmark {
            id: "bookmark123".to_string(),
            url: Some("https://example.com".to_string()),
            title: "Example Bookmark".to_string(),
            parent_id: Some("folder1".to_string()),
            bookmark_type: "bookmark".to_string(),
            date_added: Some(1704067200000), // 2024-01-01
        };
        
        let bookmark_info = connector.firefox_bookmark_to_bookmark_info(
            &firefox_bookmark,
            vec!["Bookmarks".to_string(), "Folder".to_string()],
        );
        
        assert!(bookmark_info.is_some());
        let bookmark = bookmark_info.unwrap();
        assert_eq!(bookmark.id.0, "bookmark123");
        assert_eq!(bookmark.url, "https://example.com");
        assert_eq!(bookmark.title, "Example Bookmark");
    }

    #[test]
    fn test_firefox_folder_not_converted() {
        let connector = FirefoxConnector::new();
        let firefox_folder = FirefoxBookmark {
            id: "folder123".to_string(),
            url: None,
            title: "My Folder".to_string(),
            parent_id: None,
            bookmark_type: "folder".to_string(),
            date_added: None,
        };
        
        let bookmark_info = connector.firefox_bookmark_to_bookmark_info(
            &firefox_folder,
            vec![],
        );
        
        assert!(bookmark_info.is_none());
    }
}
