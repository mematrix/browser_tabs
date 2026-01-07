//! Browser connector traits

use web_page_manager_core::*;
use async_trait::async_trait;

/// Trait for browser connectors
#[async_trait]
pub trait BrowserConnector: Send + Sync {
    /// Get the browser type
    fn browser_type(&self) -> BrowserType;
    
    /// Connect to the browser
    async fn connect(&self) -> Result<()>;
    
    /// Disconnect from the browser
    async fn disconnect(&self) -> Result<()>;
    
    /// Check if connected
    fn is_connected(&self) -> bool;
    
    /// Get all tabs (including private tabs - filtering happens at manager level)
    async fn get_tabs(&self) -> Result<Vec<TabInfo>>;
    
    /// Get all bookmarks
    async fn get_bookmarks(&self) -> Result<Vec<BookmarkInfo>>;
    
    /// Fetch page content from URL
    async fn fetch_page_content(&self, url: &str) -> Result<PageContent>;
    
    /// Close a tab
    async fn close_tab(&self, tab_id: &TabId) -> Result<()>;
    
    /// Activate a tab
    async fn activate_tab(&self, tab_id: &TabId) -> Result<()>;
    
    /// Create a new tab
    async fn create_tab(&self, url: &str) -> Result<TabId>;
}
