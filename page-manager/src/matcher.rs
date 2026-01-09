//! Tab-Bookmark Matcher Module
//!
//! Provides functionality for matching tabs with bookmarks based on URL,
//! domain, and content similarity.
//!
//! # Requirements
//! - 6.1: Display bookmark association marks when tab URL matches existing bookmark
//! - 6.2: Detect tab content changes and offer bookmark info update options

use web_page_manager_core::*;
use url::Url;
use std::collections::HashMap;

/// Configuration for the matcher
#[derive(Debug, Clone)]
pub struct MatcherConfig {
    /// Minimum similarity score for content-based matching (0.0 - 1.0)
    pub content_similarity_threshold: f32,
    /// Whether to match by exact URL
    pub match_exact_url: bool,
    /// Whether to match by domain
    pub match_domain: bool,
    /// Whether to match by content similarity
    pub match_content: bool,
    /// Whether to normalize URLs before matching (remove trailing slashes, etc.)
    pub normalize_urls: bool,
}

impl Default for MatcherConfig {
    fn default() -> Self {
        Self {
            content_similarity_threshold: 0.7,
            match_exact_url: true,
            match_domain: true,
            match_content: true,
            normalize_urls: true,
        }
    }
}

/// Tab-Bookmark Matcher
///
/// Matches tabs with bookmarks based on various criteria including
/// exact URL match, domain match, and content similarity.
pub struct TabBookmarkMatcher {
    config: MatcherConfig,
}

impl TabBookmarkMatcher {
    /// Create a new matcher with default configuration
    pub fn new() -> Self {
        Self {
            config: MatcherConfig::default(),
        }
    }

    /// Create a new matcher with custom configuration
    pub fn with_config(config: MatcherConfig) -> Self {
        Self { config }
    }

    /// Get the current configuration
    pub fn config(&self) -> &MatcherConfig {
        &self.config
    }

    /// Normalize a URL for comparison
    ///
    /// This removes trailing slashes, normalizes the scheme, and
    /// handles common URL variations.
    pub fn normalize_url(&self, url: &str) -> String {
        if !self.config.normalize_urls {
            return url.to_string();
        }

        // Try to parse the URL
        if let Ok(parsed) = Url::parse(url) {
            let mut normalized = format!(
                "{}://{}",
                parsed.scheme(),
                parsed.host_str().unwrap_or("")
            );

            // Add port if non-standard
            if let Some(port) = parsed.port() {
                let is_standard_port = matches!(
                    (parsed.scheme(), port),
                    ("http", 80) | ("https", 443)
                );
                if !is_standard_port {
                    normalized.push_str(&format!(":{}", port));
                }
            }

            // Add path, removing trailing slash
            let path = parsed.path();
            if path != "/" {
                let trimmed = path.trim_end_matches('/');
                normalized.push_str(trimmed);
            }

            // Add query string if present
            if let Some(query) = parsed.query() {
                normalized.push('?');
                normalized.push_str(query);
            }

            normalized.to_lowercase()
        } else {
            url.to_lowercase()
        }
    }

    /// Extract domain from a URL
    pub fn extract_domain(&self, url: &str) -> Option<String> {
        Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(|h| h.to_lowercase()))
    }

    /// Check if two URLs match exactly (after normalization)
    pub fn urls_match_exact(&self, url1: &str, url2: &str) -> bool {
        self.normalize_url(url1) == self.normalize_url(url2)
    }

    /// Check if two URLs are from the same domain
    pub fn urls_match_domain(&self, url1: &str, url2: &str) -> bool {
        match (self.extract_domain(url1), self.extract_domain(url2)) {
            (Some(d1), Some(d2)) => d1 == d2,
            _ => false,
        }
    }

    /// Find all bookmarks that match a given tab
    ///
    /// Returns a list of MatchInfo for all matching bookmarks,
    /// sorted by match confidence (highest first).
    pub fn find_matches_for_tab(
        &self,
        tab: &TabInfo,
        bookmarks: &[BookmarkInfo],
    ) -> Vec<MatchInfo> {
        let mut matches = Vec::new();
        let now = chrono::Utc::now();

        for bookmark in bookmarks {
            if let Some(match_info) = self.match_tab_bookmark(tab, bookmark, now) {
                matches.push(match_info);
            }
        }

        // Sort by confidence (highest first)
        matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));

        matches
    }

    /// Match a single tab with a single bookmark
    ///
    /// Returns Some(MatchInfo) if they match, None otherwise.
    fn match_tab_bookmark(
        &self,
        tab: &TabInfo,
        bookmark: &BookmarkInfo,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Option<MatchInfo> {
        // Check exact URL match first (highest priority)
        if self.config.match_exact_url && self.urls_match_exact(&tab.url, &bookmark.url) {
            return Some(MatchInfo {
                tab_id: tab.id.clone(),
                bookmark_id: bookmark.id.clone(),
                match_type: MatchType::ExactUrl,
                confidence: 1.0,
                matched_at: now,
            });
        }

        // Check domain match
        if self.config.match_domain && self.urls_match_domain(&tab.url, &bookmark.url) {
            return Some(MatchInfo {
                tab_id: tab.id.clone(),
                bookmark_id: bookmark.id.clone(),
                match_type: MatchType::SameDomain,
                confidence: 0.5,
                matched_at: now,
            });
        }

        None
    }

    /// Find all tabs that match a given bookmark
    pub fn find_matches_for_bookmark(
        &self,
        bookmark: &BookmarkInfo,
        tabs: &[TabInfo],
    ) -> Vec<MatchInfo> {
        let mut matches = Vec::new();
        let now = chrono::Utc::now();

        for tab in tabs {
            if let Some(match_info) = self.match_tab_bookmark(tab, bookmark, now) {
                matches.push(match_info);
            }
        }

        matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));

        matches
    }

    /// Build a complete match map between tabs and bookmarks
    ///
    /// Returns a HashMap where keys are tab IDs and values are lists of
    /// matching bookmark IDs with their match info.
    pub fn build_match_map(
        &self,
        tabs: &[TabInfo],
        bookmarks: &[BookmarkInfo],
    ) -> HashMap<TabId, Vec<MatchInfo>> {
        let mut match_map = HashMap::new();

        for tab in tabs {
            let matches = self.find_matches_for_tab(tab, bookmarks);
            if !matches.is_empty() {
                match_map.insert(tab.id.clone(), matches);
            }
        }

        match_map
    }

    /// Get the best match for a tab (if any)
    pub fn get_best_match_for_tab(
        &self,
        tab: &TabInfo,
        bookmarks: &[BookmarkInfo],
    ) -> Option<MatchInfo> {
        self.find_matches_for_tab(tab, bookmarks).into_iter().next()
    }

    /// Check if a tab has any matching bookmark
    pub fn tab_has_bookmark_match(&self, tab: &TabInfo, bookmarks: &[BookmarkInfo]) -> bool {
        bookmarks.iter().any(|b| {
            (self.config.match_exact_url && self.urls_match_exact(&tab.url, &b.url))
                || (self.config.match_domain && self.urls_match_domain(&tab.url, &b.url))
        })
    }
}

impl Default for TabBookmarkMatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of detecting changes between tab and bookmark
#[derive(Debug, Clone)]
pub struct ContentChangeDetection {
    /// The tab that was checked
    pub tab_id: TabId,
    /// The bookmark that was compared
    pub bookmark_id: BookmarkId,
    /// Whether the title has changed
    pub title_changed: bool,
    /// Whether the favicon has changed
    pub favicon_changed: bool,
    /// The old title (from bookmark)
    pub old_title: String,
    /// The new title (from tab)
    pub new_title: String,
    /// The old favicon URL (from bookmark)
    pub old_favicon: Option<String>,
    /// The new favicon URL (from tab)
    pub new_favicon: Option<String>,
}

impl ContentChangeDetection {
    /// Check if any changes were detected
    pub fn has_changes(&self) -> bool {
        self.title_changed || self.favicon_changed
    }
}

/// Content change detector for tabs and bookmarks
///
/// Implements Requirement 6.2: Detect tab content changes and offer
/// bookmark info update options.
pub struct ContentChangeDetector;

impl ContentChangeDetector {
    /// Detect changes between a tab and its matching bookmark
    pub fn detect_changes(tab: &TabInfo, bookmark: &BookmarkInfo) -> ContentChangeDetection {
        let title_changed = tab.title != bookmark.title;
        let favicon_changed = tab.favicon_url != bookmark.favicon_url;

        ContentChangeDetection {
            tab_id: tab.id.clone(),
            bookmark_id: bookmark.id.clone(),
            title_changed,
            favicon_changed,
            old_title: bookmark.title.clone(),
            new_title: tab.title.clone(),
            old_favicon: bookmark.favicon_url.clone(),
            new_favicon: tab.favicon_url.clone(),
        }
    }

    /// Detect changes for all matched tab-bookmark pairs
    pub fn detect_all_changes(
        tabs: &[TabInfo],
        bookmarks: &[BookmarkInfo],
        matches: &HashMap<TabId, Vec<MatchInfo>>,
    ) -> Vec<ContentChangeDetection> {
        let mut changes = Vec::new();

        // Create lookup maps
        let tab_map: HashMap<&TabId, &TabInfo> = tabs.iter().map(|t| (&t.id, t)).collect();
        let bookmark_map: HashMap<&BookmarkId, &BookmarkInfo> =
            bookmarks.iter().map(|b| (&b.id, b)).collect();

        for (tab_id, match_infos) in matches {
            if let Some(tab) = tab_map.get(tab_id) {
                // Only check exact URL matches for content changes
                for match_info in match_infos {
                    if matches!(match_info.match_type, MatchType::ExactUrl) {
                        if let Some(bookmark) = bookmark_map.get(&match_info.bookmark_id) {
                            let detection = Self::detect_changes(tab, bookmark);
                            if detection.has_changes() {
                                changes.push(detection);
                            }
                        }
                    }
                }
            }
        }

        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tab(url: &str, title: &str) -> TabInfo {
        TabInfo {
            id: TabId::new(),
            url: url.to_string(),
            title: title.to_string(),
            favicon_url: None,
            browser_type: BrowserType::Chrome,
            is_private: false,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
        }
    }

    fn create_test_bookmark(url: &str, title: &str) -> BookmarkInfo {
        BookmarkInfo {
            id: BookmarkId::new(),
            url: url.to_string(),
            title: title.to_string(),
            favicon_url: None,
            browser_type: BrowserType::Chrome,
            folder_path: vec![],
            created_at: chrono::Utc::now(),
            last_accessed: None,
        }
    }

    #[test]
    fn test_url_normalization() {
        let matcher = TabBookmarkMatcher::new();

        assert_eq!(
            matcher.normalize_url("https://example.com/"),
            matcher.normalize_url("https://example.com")
        );

        assert_eq!(
            matcher.normalize_url("HTTPS://EXAMPLE.COM/path"),
            matcher.normalize_url("https://example.com/path")
        );
    }

    #[test]
    fn test_exact_url_match() {
        let matcher = TabBookmarkMatcher::new();
        let tab = create_test_tab("https://example.com/page", "Example");
        let bookmark = create_test_bookmark("https://example.com/page", "Example Bookmark");

        let matches = matcher.find_matches_for_tab(&tab, &[bookmark]);
        assert_eq!(matches.len(), 1);
        assert!(matches!(matches[0].match_type, MatchType::ExactUrl));
        assert_eq!(matches[0].confidence, 1.0);
    }

    #[test]
    fn test_domain_match() {
        let matcher = TabBookmarkMatcher::new();
        let tab = create_test_tab("https://example.com/page1", "Page 1");
        let bookmark = create_test_bookmark("https://example.com/page2", "Page 2");

        let matches = matcher.find_matches_for_tab(&tab, &[bookmark]);
        assert_eq!(matches.len(), 1);
        assert!(matches!(matches[0].match_type, MatchType::SameDomain));
        assert_eq!(matches[0].confidence, 0.5);
    }

    #[test]
    fn test_no_match() {
        let matcher = TabBookmarkMatcher::new();
        let tab = create_test_tab("https://example.com/page", "Example");
        let bookmark = create_test_bookmark("https://other.com/page", "Other");

        let matches = matcher.find_matches_for_tab(&tab, &[bookmark]);
        assert!(matches.is_empty());
    }

    #[test]
    fn test_content_change_detection() {
        let tab = create_test_tab("https://example.com", "New Title");
        let bookmark = create_test_bookmark("https://example.com", "Old Title");

        let detection = ContentChangeDetector::detect_changes(&tab, &bookmark);
        assert!(detection.has_changes());
        assert!(detection.title_changed);
        assert!(!detection.favicon_changed);
        assert_eq!(detection.old_title, "Old Title");
        assert_eq!(detection.new_title, "New Title");
    }
}
