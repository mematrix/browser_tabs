//! Privacy mode filter for excluding incognito/private tabs

use web_page_manager_core::TabInfo;

/// Filter for excluding private/incognito mode tabs
pub struct PrivacyModeFilter;

impl PrivacyModeFilter {
    pub fn new() -> Self {
        Self
    }

    /// Filter out private/incognito tabs from the list
    pub fn filter_tabs(&self, tabs: Vec<TabInfo>) -> Vec<TabInfo> {
        tabs.into_iter()
            .filter(|tab| !tab.is_private)
            .collect()
    }

    /// Check if a single tab is in private mode
    pub fn is_private(&self, tab: &TabInfo) -> bool {
        tab.is_private
    }
}

impl Default for PrivacyModeFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use web_page_manager_core::{BrowserType, TabId, Utc};

    fn create_test_tab(is_private: bool) -> TabInfo {
        TabInfo {
            id: TabId::new(),
            url: "https://example.com".to_string(),
            title: "Test Tab".to_string(),
            favicon_url: None,
            browser_type: BrowserType::Chrome,
            is_private,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
        }
    }

    #[test]
    fn test_filter_removes_private_tabs() {
        let filter = PrivacyModeFilter::new();
        
        let tabs = vec![
            create_test_tab(false),
            create_test_tab(true),
            create_test_tab(false),
            create_test_tab(true),
        ];
        
        let filtered = filter.filter_tabs(tabs);
        
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|t| !t.is_private));
    }

    #[test]
    fn test_filter_keeps_all_normal_tabs() {
        let filter = PrivacyModeFilter::new();
        
        let tabs = vec![
            create_test_tab(false),
            create_test_tab(false),
            create_test_tab(false),
        ];
        
        let filtered = filter.filter_tabs(tabs);
        
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn test_is_private() {
        let filter = PrivacyModeFilter::new();
        
        let private_tab = create_test_tab(true);
        let normal_tab = create_test_tab(false);
        
        assert!(filter.is_private(&private_tab));
        assert!(!filter.is_private(&normal_tab));
    }
}
