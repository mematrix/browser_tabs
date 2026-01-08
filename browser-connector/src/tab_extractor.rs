//! Tab information extraction module
//!
//! This module provides enhanced tab information extraction functionality,
//! including metadata extraction, domain analysis, and tab categorization.

use web_page_manager_core::{TabInfo, Utc};
use std::collections::HashMap;
use url::Url;

/// Extended tab information with additional metadata
#[derive(Debug, Clone)]
pub struct ExtendedTabInfo {
    /// Base tab information
    pub tab: TabInfo,
    /// Extracted domain from URL
    pub domain: Option<String>,
    /// Subdomain if present
    pub subdomain: Option<String>,
    /// URL path
    pub path: String,
    /// Query parameters
    pub query_params: HashMap<String, String>,
    /// Whether the URL uses HTTPS
    pub is_secure: bool,
    /// Detected content category
    pub category: Option<TabCategory>,
    /// Tab age in seconds
    pub age_seconds: i64,
}

/// Categories for tab content
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TabCategory {
    /// Search engine results
    Search,
    /// Social media
    SocialMedia,
    /// Video streaming
    Video,
    /// News and articles
    News,
    /// Shopping and e-commerce
    Shopping,
    /// Email and communication
    Communication,
    /// Development and coding
    Development,
    /// Documentation and reference
    Documentation,
    /// Entertainment
    Entertainment,
    /// Finance and banking
    Finance,
    /// Other/uncategorized
    Other,
}

/// Domain patterns for categorization
const SEARCH_DOMAINS: &[&str] = &["google.com", "bing.com", "duckduckgo.com", "yahoo.com", "baidu.com"];
const SOCIAL_DOMAINS: &[&str] = &["facebook.com", "twitter.com", "x.com", "instagram.com", "linkedin.com", "reddit.com", "tiktok.com"];
const VIDEO_DOMAINS: &[&str] = &["youtube.com", "vimeo.com", "twitch.tv", "netflix.com", "hulu.com", "bilibili.com"];
const NEWS_DOMAINS: &[&str] = &["cnn.com", "bbc.com", "nytimes.com", "reuters.com", "theguardian.com"];
const SHOPPING_DOMAINS: &[&str] = &["amazon.com", "ebay.com", "alibaba.com", "walmart.com", "etsy.com", "taobao.com"];
const COMMUNICATION_DOMAINS: &[&str] = &["gmail.com", "outlook.com", "mail.google.com", "slack.com", "discord.com", "teams.microsoft.com"];
const DEV_DOMAINS: &[&str] = &["github.com", "gitlab.com", "stackoverflow.com", "npmjs.com", "crates.io", "pypi.org"];
const DOC_DOMAINS: &[&str] = &["docs.rs", "developer.mozilla.org", "docs.microsoft.com", "docs.python.org", "rust-lang.org"];
const FINANCE_DOMAINS: &[&str] = &["paypal.com", "chase.com", "bankofamerica.com", "wellsfargo.com", "coinbase.com"];

/// Tab information extractor
pub struct TabExtractor {
    /// Custom domain categorizations
    custom_categories: HashMap<String, TabCategory>,
}

impl TabExtractor {
    /// Create a new tab extractor
    pub fn new() -> Self {
        Self {
            custom_categories: HashMap::new(),
        }
    }

    /// Add a custom domain categorization
    pub fn add_custom_category(&mut self, domain: &str, category: TabCategory) {
        self.custom_categories.insert(domain.to_lowercase(), category);
    }

    /// Extract extended information from a tab
    pub fn extract(&self, tab: &TabInfo) -> ExtendedTabInfo {
        let parsed_url = Url::parse(&tab.url).ok();
        
        let (domain, subdomain, path, query_params, is_secure) = if let Some(url) = &parsed_url {
            let host = url.host_str().unwrap_or("");
            let (domain, subdomain) = self.extract_domain_parts(host);
            let path = url.path().to_string();
            let query_params = self.extract_query_params(url);
            let is_secure = url.scheme() == "https";
            
            (domain, subdomain, path, query_params, is_secure)
        } else {
            (None, None, String::new(), HashMap::new(), false)
        };
        
        let category = domain.as_ref().and_then(|d| self.categorize_domain(d));
        
        let age_seconds = (Utc::now() - tab.created_at).num_seconds();
        
        ExtendedTabInfo {
            tab: tab.clone(),
            domain,
            subdomain,
            path,
            query_params,
            is_secure,
            category,
            age_seconds,
        }
    }

    /// Extract domain and subdomain from host
    fn extract_domain_parts(&self, host: &str) -> (Option<String>, Option<String>) {
        let parts: Vec<&str> = host.split('.').collect();
        
        if parts.len() >= 2 {
            // Handle common TLDs
            let domain = if parts.len() >= 3 && (parts[parts.len() - 2] == "co" || parts[parts.len() - 2] == "com") {
                // Handle domains like example.co.uk
                format!("{}.{}.{}", parts[parts.len() - 3], parts[parts.len() - 2], parts[parts.len() - 1])
            } else {
                format!("{}.{}", parts[parts.len() - 2], parts[parts.len() - 1])
            };
            
            let subdomain = if parts.len() > 2 {
                let subdomain_parts: Vec<&str> = parts[..parts.len() - 2].to_vec();
                if !subdomain_parts.is_empty() && subdomain_parts[0] != "www" {
                    Some(subdomain_parts.join("."))
                } else {
                    None
                }
            } else {
                None
            };
            
            (Some(domain), subdomain)
        } else {
            (Some(host.to_string()), None)
        }
    }

    /// Extract query parameters from URL
    fn extract_query_params(&self, url: &Url) -> HashMap<String, String> {
        url.query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    /// Categorize a domain
    fn categorize_domain(&self, domain: &str) -> Option<TabCategory> {
        let lower_domain = domain.to_lowercase();
        
        // Check custom categories first
        if let Some(category) = self.custom_categories.get(&lower_domain) {
            return Some(category.clone());
        }
        
        // Check built-in categories
        if SEARCH_DOMAINS.iter().any(|d| lower_domain.contains(d)) {
            return Some(TabCategory::Search);
        }
        if SOCIAL_DOMAINS.iter().any(|d| lower_domain.contains(d)) {
            return Some(TabCategory::SocialMedia);
        }
        if VIDEO_DOMAINS.iter().any(|d| lower_domain.contains(d)) {
            return Some(TabCategory::Video);
        }
        if NEWS_DOMAINS.iter().any(|d| lower_domain.contains(d)) {
            return Some(TabCategory::News);
        }
        if SHOPPING_DOMAINS.iter().any(|d| lower_domain.contains(d)) {
            return Some(TabCategory::Shopping);
        }
        if COMMUNICATION_DOMAINS.iter().any(|d| lower_domain.contains(d)) {
            return Some(TabCategory::Communication);
        }
        if DEV_DOMAINS.iter().any(|d| lower_domain.contains(d)) {
            return Some(TabCategory::Development);
        }
        if DOC_DOMAINS.iter().any(|d| lower_domain.contains(d)) {
            return Some(TabCategory::Documentation);
        }
        if FINANCE_DOMAINS.iter().any(|d| lower_domain.contains(d)) {
            return Some(TabCategory::Finance);
        }
        
        None
    }

    /// Extract extended information from multiple tabs
    pub fn extract_all(&self, tabs: &[TabInfo]) -> Vec<ExtendedTabInfo> {
        tabs.iter().map(|t| self.extract(t)).collect()
    }

    /// Group tabs by domain
    pub fn group_by_domain(&self, tabs: &[TabInfo]) -> HashMap<String, Vec<TabInfo>> {
        let mut groups: HashMap<String, Vec<TabInfo>> = HashMap::new();
        
        for tab in tabs {
            let extended = self.extract(tab);
            let key = extended.domain.unwrap_or_else(|| "unknown".to_string());
            groups.entry(key).or_default().push(tab.clone());
        }
        
        groups
    }

    /// Group tabs by category
    pub fn group_by_category(&self, tabs: &[TabInfo]) -> HashMap<TabCategory, Vec<TabInfo>> {
        let mut groups: HashMap<TabCategory, Vec<TabInfo>> = HashMap::new();
        
        for tab in tabs {
            let extended = self.extract(tab);
            let category = extended.category.unwrap_or(TabCategory::Other);
            groups.entry(category).or_default().push(tab.clone());
        }
        
        groups
    }

    /// Get statistics about tabs
    pub fn get_tab_stats(&self, tabs: &[TabInfo]) -> TabStats {
        let extended: Vec<ExtendedTabInfo> = self.extract_all(tabs);
        
        let secure_count = extended.iter().filter(|t| t.is_secure).count();
        let categorized_count = extended.iter().filter(|t| t.category.is_some()).count();
        
        let mut domain_counts: HashMap<String, usize> = HashMap::new();
        let mut category_counts: HashMap<TabCategory, usize> = HashMap::new();
        
        for ext in &extended {
            if let Some(domain) = &ext.domain {
                *domain_counts.entry(domain.clone()).or_insert(0) += 1;
            }
            let category = ext.category.clone().unwrap_or(TabCategory::Other);
            *category_counts.entry(category).or_insert(0) += 1;
        }
        
        let avg_age_seconds = if !extended.is_empty() {
            extended.iter().map(|t| t.age_seconds).sum::<i64>() / extended.len() as i64
        } else {
            0
        };
        
        TabStats {
            total_tabs: tabs.len(),
            secure_tabs: secure_count,
            insecure_tabs: tabs.len() - secure_count,
            categorized_tabs: categorized_count,
            uncategorized_tabs: tabs.len() - categorized_count,
            unique_domains: domain_counts.len(),
            domain_counts,
            category_counts,
            avg_age_seconds,
        }
    }
}

impl Default for TabExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about a collection of tabs
#[derive(Debug, Clone)]
pub struct TabStats {
    /// Total number of tabs
    pub total_tabs: usize,
    /// Number of tabs using HTTPS
    pub secure_tabs: usize,
    /// Number of tabs not using HTTPS
    pub insecure_tabs: usize,
    /// Number of tabs with detected category
    pub categorized_tabs: usize,
    /// Number of tabs without detected category
    pub uncategorized_tabs: usize,
    /// Number of unique domains
    pub unique_domains: usize,
    /// Count of tabs per domain
    pub domain_counts: HashMap<String, usize>,
    /// Count of tabs per category
    pub category_counts: HashMap<TabCategory, usize>,
    /// Average tab age in seconds
    pub avg_age_seconds: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use web_page_manager_core::{BrowserType, TabId};

    fn create_test_tab(url: &str) -> TabInfo {
        TabInfo {
            id: TabId::new(),
            url: url.to_string(),
            title: "Test Tab".to_string(),
            favicon_url: None,
            browser_type: BrowserType::Chrome,
            is_private: false,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
        }
    }

    #[test]
    fn test_extract_basic_info() {
        let extractor = TabExtractor::new();
        let tab = create_test_tab("https://www.example.com/path?query=value");
        
        let extended = extractor.extract(&tab);
        
        assert_eq!(extended.domain, Some("example.com".to_string()));
        assert_eq!(extended.path, "/path");
        assert!(extended.is_secure);
        assert_eq!(extended.query_params.get("query"), Some(&"value".to_string()));
    }

    #[test]
    fn test_categorize_search() {
        let extractor = TabExtractor::new();
        let tab = create_test_tab("https://www.google.com/search?q=test");
        
        let extended = extractor.extract(&tab);
        
        assert_eq!(extended.category, Some(TabCategory::Search));
    }

    #[test]
    fn test_categorize_social() {
        let extractor = TabExtractor::new();
        let tab = create_test_tab("https://twitter.com/user");
        
        let extended = extractor.extract(&tab);
        
        assert_eq!(extended.category, Some(TabCategory::SocialMedia));
    }

    #[test]
    fn test_categorize_video() {
        let extractor = TabExtractor::new();
        let tab = create_test_tab("https://www.youtube.com/watch?v=abc123");
        
        let extended = extractor.extract(&tab);
        
        assert_eq!(extended.category, Some(TabCategory::Video));
    }

    #[test]
    fn test_categorize_development() {
        let extractor = TabExtractor::new();
        let tab = create_test_tab("https://github.com/user/repo");
        
        let extended = extractor.extract(&tab);
        
        assert_eq!(extended.category, Some(TabCategory::Development));
    }

    #[test]
    fn test_custom_category() {
        let mut extractor = TabExtractor::new();
        extractor.add_custom_category("mysite.com", TabCategory::Entertainment);
        
        let tab = create_test_tab("https://mysite.com/page");
        let extended = extractor.extract(&tab);
        
        assert_eq!(extended.category, Some(TabCategory::Entertainment));
    }

    #[test]
    fn test_group_by_domain() {
        let extractor = TabExtractor::new();
        let tabs = vec![
            create_test_tab("https://example.com/page1"),
            create_test_tab("https://example.com/page2"),
            create_test_tab("https://other.com/page"),
        ];
        
        let groups = extractor.group_by_domain(&tabs);
        
        assert_eq!(groups.get("example.com").map(|v| v.len()), Some(2));
        assert_eq!(groups.get("other.com").map(|v| v.len()), Some(1));
    }

    #[test]
    fn test_group_by_category() {
        let extractor = TabExtractor::new();
        let tabs = vec![
            create_test_tab("https://github.com/repo1"),
            create_test_tab("https://github.com/repo2"),
            create_test_tab("https://youtube.com/video"),
        ];
        
        let groups = extractor.group_by_category(&tabs);
        
        assert_eq!(groups.get(&TabCategory::Development).map(|v| v.len()), Some(2));
        assert_eq!(groups.get(&TabCategory::Video).map(|v| v.len()), Some(1));
    }

    #[test]
    fn test_tab_stats() {
        let extractor = TabExtractor::new();
        let tabs = vec![
            create_test_tab("https://github.com/repo"),
            create_test_tab("https://youtube.com/video"),
            create_test_tab("http://insecure.com/page"),
        ];
        
        let stats = extractor.get_tab_stats(&tabs);
        
        assert_eq!(stats.total_tabs, 3);
        assert_eq!(stats.secure_tabs, 2);
        assert_eq!(stats.insecure_tabs, 1);
        assert_eq!(stats.unique_domains, 3);
    }

    #[test]
    fn test_extract_subdomain() {
        let extractor = TabExtractor::new();
        let tab = create_test_tab("https://api.example.com/endpoint");
        
        let extended = extractor.extract(&tab);
        
        assert_eq!(extended.domain, Some("example.com".to_string()));
        assert_eq!(extended.subdomain, Some("api".to_string()));
    }

    #[test]
    fn test_www_subdomain_ignored() {
        let extractor = TabExtractor::new();
        let tab = create_test_tab("https://www.example.com/page");
        
        let extended = extractor.extract(&tab);
        
        assert_eq!(extended.domain, Some("example.com".to_string()));
        assert_eq!(extended.subdomain, None);
    }
}
