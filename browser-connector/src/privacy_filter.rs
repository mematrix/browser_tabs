//! Privacy mode filter for excluding incognito/private tabs
//!
//! This module provides comprehensive privacy filtering for browser tabs,
//! including detection of private/incognito mode and filtering of sensitive URLs.

use web_page_manager_core::TabInfo;

/// URL patterns that indicate privacy-sensitive content
const PRIVACY_URL_PATTERNS: &[&str] = &[
    "chrome://settings/privacy",
    "chrome://settings/passwords",
    "chrome://settings/security",
    "chrome://history",
    "edge://settings/privacy",
    "edge://settings/passwords",
    "edge://history",
    "about:preferences#privacy",
    "about:logins",
    "about:privatebrowsing",
];

/// URL schemes that should be filtered out
const FILTERED_SCHEMES: &[&str] = &[
    "chrome-extension://",
    "moz-extension://",
    "edge://",
    "chrome://",
    "about:",
    "file://",
];

/// Filter configuration for privacy mode filtering
#[derive(Debug, Clone)]
pub struct PrivacyFilterConfig {
    /// Whether to filter out browser internal pages
    pub filter_internal_pages: bool,
    /// Whether to filter out extension pages
    pub filter_extension_pages: bool,
    /// Whether to filter out file:// URLs
    pub filter_file_urls: bool,
    /// Custom URL patterns to filter
    pub custom_filter_patterns: Vec<String>,
}

impl Default for PrivacyFilterConfig {
    fn default() -> Self {
        Self {
            filter_internal_pages: true,
            filter_extension_pages: true,
            filter_file_urls: true,
            custom_filter_patterns: Vec::new(),
        }
    }
}

/// Filter for excluding private/incognito mode tabs
/// 
/// This filter provides multiple layers of privacy protection:
/// 1. Filters tabs marked as private/incognito by the browser
/// 2. Filters tabs with privacy-sensitive URLs
/// 3. Filters browser internal pages and extension pages
pub struct PrivacyModeFilter {
    config: PrivacyFilterConfig,
}

impl PrivacyModeFilter {
    /// Create a new privacy filter with default configuration
    pub fn new() -> Self {
        Self {
            config: PrivacyFilterConfig::default(),
        }
    }

    /// Create a new privacy filter with custom configuration
    pub fn with_config(config: PrivacyFilterConfig) -> Self {
        Self { config }
    }

    /// Filter out private/incognito tabs from the list
    /// 
    /// This method applies all configured filters:
    /// - Removes tabs marked as private/incognito
    /// - Removes tabs with privacy-sensitive URLs
    /// - Removes browser internal pages (if configured)
    /// - Removes extension pages (if configured)
    pub fn filter_tabs(&self, tabs: Vec<TabInfo>) -> Vec<TabInfo> {
        tabs.into_iter()
            .filter(|tab| !self.should_filter(tab))
            .collect()
    }

    /// Check if a single tab should be filtered out
    pub fn should_filter(&self, tab: &TabInfo) -> bool {
        // Check if tab is marked as private
        if tab.is_private {
            return true;
        }

        // Check for privacy-sensitive URLs
        if self.is_privacy_sensitive_url(&tab.url) {
            return true;
        }

        // Check for filtered URL schemes
        if self.should_filter_by_scheme(&tab.url) {
            return true;
        }

        // Check custom filter patterns
        if self.matches_custom_pattern(&tab.url) {
            return true;
        }

        false
    }

    /// Check if a single tab is in private mode
    pub fn is_private(&self, tab: &TabInfo) -> bool {
        tab.is_private
    }

    /// Check if a URL is privacy-sensitive
    pub fn is_privacy_sensitive_url(&self, url: &str) -> bool {
        let lower_url = url.to_lowercase();
        PRIVACY_URL_PATTERNS.iter().any(|pattern| lower_url.contains(pattern))
    }

    /// Check if a URL should be filtered based on its scheme
    fn should_filter_by_scheme(&self, url: &str) -> bool {
        let lower_url = url.to_lowercase();

        for scheme in FILTERED_SCHEMES {
            if lower_url.starts_with(scheme) {
                // Check configuration for specific schemes
                if scheme.starts_with("chrome://") || scheme.starts_with("edge://") || scheme.starts_with("about:") {
                    if self.config.filter_internal_pages {
                        return true;
                    }
                } else if scheme.contains("-extension://") {
                    if self.config.filter_extension_pages {
                        return true;
                    }
                } else if *scheme == "file://" {
                    if self.config.filter_file_urls {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check if URL matches any custom filter pattern
    fn matches_custom_pattern(&self, url: &str) -> bool {
        let lower_url = url.to_lowercase();
        self.config.custom_filter_patterns.iter()
            .any(|pattern| lower_url.contains(&pattern.to_lowercase()))
    }

    /// Get statistics about filtered tabs
    pub fn get_filter_stats(&self, tabs: &[TabInfo]) -> FilterStats {
        let mut stats = FilterStats::default();
        
        for tab in tabs {
            if tab.is_private {
                stats.private_tabs += 1;
            }
            if self.is_privacy_sensitive_url(&tab.url) {
                stats.privacy_sensitive_urls += 1;
            }
            if self.should_filter_by_scheme(&tab.url) {
                stats.internal_pages += 1;
            }
            if self.matches_custom_pattern(&tab.url) {
                stats.custom_filtered += 1;
            }
        }
        
        stats.total_tabs = tabs.len();
        stats.filtered_tabs = tabs.iter().filter(|t| self.should_filter(t)).count();
        stats.passed_tabs = stats.total_tabs - stats.filtered_tabs;
        
        stats
    }
}

impl Default for PrivacyModeFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about filtered tabs
#[derive(Debug, Clone, Default)]
pub struct FilterStats {
    /// Total number of tabs before filtering
    pub total_tabs: usize,
    /// Number of tabs that passed the filter
    pub passed_tabs: usize,
    /// Number of tabs that were filtered out
    pub filtered_tabs: usize,
    /// Number of private/incognito tabs
    pub private_tabs: usize,
    /// Number of tabs with privacy-sensitive URLs
    pub privacy_sensitive_urls: usize,
    /// Number of browser internal pages
    pub internal_pages: usize,
    /// Number of tabs filtered by custom patterns
    pub custom_filtered: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use web_page_manager_core::{BrowserType, TabId, Utc};

    fn create_test_tab(url: &str, is_private: bool) -> TabInfo {
        TabInfo {
            id: TabId::new(),
            url: url.to_string(),
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
            create_test_tab("https://example.com", false),
            create_test_tab("https://example.org", true),
            create_test_tab("https://example.net", false),
            create_test_tab("https://example.io", true),
        ];
        
        let filtered = filter.filter_tabs(tabs);
        
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|t| !t.is_private));
    }

    #[test]
    fn test_filter_keeps_all_normal_tabs() {
        let filter = PrivacyModeFilter::new();
        
        let tabs = vec![
            create_test_tab("https://example.com", false),
            create_test_tab("https://example.org", false),
            create_test_tab("https://example.net", false),
        ];
        
        let filtered = filter.filter_tabs(tabs);
        
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn test_is_private() {
        let filter = PrivacyModeFilter::new();
        
        let private_tab = create_test_tab("https://example.com", true);
        let normal_tab = create_test_tab("https://example.com", false);
        
        assert!(filter.is_private(&private_tab));
        assert!(!filter.is_private(&normal_tab));
    }

    #[test]
    fn test_filter_privacy_sensitive_urls() {
        let filter = PrivacyModeFilter::new();
        
        let tabs = vec![
            create_test_tab("https://example.com", false),
            create_test_tab("chrome://settings/privacy", false),
            create_test_tab("https://google.com", false),
        ];
        
        let filtered = filter.filter_tabs(tabs);
        
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|t| !t.url.contains("privacy")));
    }

    #[test]
    fn test_filter_internal_pages() {
        let filter = PrivacyModeFilter::new();
        
        let tabs = vec![
            create_test_tab("https://example.com", false),
            create_test_tab("chrome://newtab", false),
            create_test_tab("edge://settings", false),
            create_test_tab("about:blank", false),
        ];
        
        let filtered = filter.filter_tabs(tabs);
        
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].url, "https://example.com");
    }

    #[test]
    fn test_filter_extension_pages() {
        let filter = PrivacyModeFilter::new();
        
        let tabs = vec![
            create_test_tab("https://example.com", false),
            create_test_tab("chrome-extension://abc123/popup.html", false),
            create_test_tab("moz-extension://def456/options.html", false),
        ];
        
        let filtered = filter.filter_tabs(tabs);
        
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].url, "https://example.com");
    }

    #[test]
    fn test_custom_filter_patterns() {
        let config = PrivacyFilterConfig {
            custom_filter_patterns: vec!["secret".to_string(), "private-data".to_string()],
            ..Default::default()
        };
        let filter = PrivacyModeFilter::with_config(config);
        
        let tabs = vec![
            create_test_tab("https://example.com", false),
            create_test_tab("https://example.com/secret/page", false),
            create_test_tab("https://example.com/private-data/info", false),
        ];
        
        let filtered = filter.filter_tabs(tabs);
        
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].url, "https://example.com");
    }

    #[test]
    fn test_filter_stats() {
        let filter = PrivacyModeFilter::new();
        
        let tabs = vec![
            create_test_tab("https://example.com", false),
            create_test_tab("https://example.org", true),
            create_test_tab("chrome://settings/privacy", false),
            create_test_tab("chrome://newtab", false),
        ];
        
        let stats = filter.get_filter_stats(&tabs);
        
        assert_eq!(stats.total_tabs, 4);
        assert_eq!(stats.private_tabs, 1);
        assert_eq!(stats.passed_tabs, 1);
        assert_eq!(stats.filtered_tabs, 3);
    }

    #[test]
    fn test_config_disable_internal_page_filter() {
        let config = PrivacyFilterConfig {
            filter_internal_pages: false,
            ..Default::default()
        };
        let filter = PrivacyModeFilter::with_config(config);
        
        let tabs = vec![
            create_test_tab("https://example.com", false),
            create_test_tab("chrome://newtab", false),
        ];
        
        let filtered = filter.filter_tabs(tabs);
        
        // chrome://newtab should pass since internal page filtering is disabled
        assert_eq!(filtered.len(), 2);
    }
}
