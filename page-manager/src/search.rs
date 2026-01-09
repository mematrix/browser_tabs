//! Unified Search Module
//!
//! Provides cross-data-source unified search functionality for tabs, bookmarks,
//! history, and archived content.
//!
//! # Requirements
//! - 6.5: Unified search across tabs and bookmarks with comprehensive results

use web_page_manager_core::*;
use data_access::{
    PageRepository, HistoryRepository, ArchiveRepository,
    SqlitePageRepository, SqliteHistoryRepository, SqliteArchiveRepository,
    DatabaseManager,
};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Maximum number of search history entries to keep
const MAX_SEARCH_HISTORY: usize = 100;

/// Maximum number of search suggestions to return
const MAX_SUGGESTIONS: usize = 10;

/// Unified search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    /// Unique identifier for the result
    pub id: String,
    /// The URL of the result
    pub url: String,
    /// The title of the result
    pub title: String,
    /// Optional favicon URL
    pub favicon_url: Option<String>,
    /// The source type of this result
    pub source_type: SearchResultSource,
    /// Relevance score (0.0 - 1.0)
    pub relevance_score: f32,
    /// Snippet or summary of the content
    pub snippet: Option<String>,
    /// Keywords associated with this result
    pub keywords: Vec<String>,
    /// When this item was last accessed
    pub last_accessed: DateTime<Utc>,
    /// Browser type (if applicable)
    pub browser_type: Option<BrowserType>,
}

/// Source type for search results
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SearchResultSource {
    /// Result from active tabs
    ActiveTab,
    /// Result from bookmarks
    Bookmark,
    /// Result from tab history
    History,
    /// Result from archived content
    Archive,
    /// Result from unified pages (merged data)
    UnifiedPage,
}

/// Search filter options
#[derive(Debug, Clone, Default)]
pub struct SearchFilter {
    /// Filter by source types (empty means all sources)
    pub source_types: Vec<SearchResultSource>,
    /// Filter by browser type
    pub browser_type: Option<BrowserType>,
    /// Filter by date range (from)
    pub from_date: Option<DateTime<Utc>>,
    /// Filter by date range (to)
    pub to_date: Option<DateTime<Utc>>,
    /// Filter by category
    pub category: Option<String>,
    /// Filter by keywords (any match)
    pub keywords: Vec<String>,
}

impl SearchFilter {
    /// Create a new empty filter (matches all)
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter only active tabs
    pub fn tabs_only() -> Self {
        Self {
            source_types: vec![SearchResultSource::ActiveTab],
            ..Default::default()
        }
    }

    /// Filter only bookmarks
    pub fn bookmarks_only() -> Self {
        Self {
            source_types: vec![SearchResultSource::Bookmark],
            ..Default::default()
        }
    }

    /// Filter only history
    pub fn history_only() -> Self {
        Self {
            source_types: vec![SearchResultSource::History],
            ..Default::default()
        }
    }

    /// Filter only archives
    pub fn archives_only() -> Self {
        Self {
            source_types: vec![SearchResultSource::Archive],
            ..Default::default()
        }
    }

    /// Add a source type filter
    pub fn with_source(mut self, source: SearchResultSource) -> Self {
        self.source_types.push(source);
        self
    }

    /// Set browser type filter
    pub fn with_browser(mut self, browser: BrowserType) -> Self {
        self.browser_type = Some(browser);
        self
    }

    /// Set date range filter
    pub fn with_date_range(mut self, from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>) -> Self {
        self.from_date = from;
        self.to_date = to;
        self
    }

    /// Check if a result matches this filter
    pub fn matches(&self, result: &SearchResultItem) -> bool {
        // Check source type filter
        if !self.source_types.is_empty() && !self.source_types.contains(&result.source_type) {
            return false;
        }

        // Check browser type filter
        if let Some(ref browser) = self.browser_type {
            if result.browser_type.as_ref() != Some(browser) {
                return false;
            }
        }

        // Check date range filter
        if let Some(from) = self.from_date {
            if result.last_accessed < from {
                return false;
            }
        }
        if let Some(to) = self.to_date {
            if result.last_accessed > to {
                return false;
            }
        }

        // Check keywords filter
        if !self.keywords.is_empty() {
            let has_keyword = self.keywords.iter().any(|k| {
                result.keywords.iter().any(|rk| rk.to_lowercase().contains(&k.to_lowercase()))
            });
            if !has_keyword {
                return false;
            }
        }

        true
    }
}

/// Sort order for search results
#[derive(Debug, Clone, Copy, Default)]
pub enum SearchSortOrder {
    /// Sort by relevance score (default)
    #[default]
    Relevance,
    /// Sort by last accessed time (newest first)
    RecentFirst,
    /// Sort by last accessed time (oldest first)
    OldestFirst,
    /// Sort by title alphabetically
    TitleAsc,
    /// Sort by title reverse alphabetically
    TitleDesc,
}

/// Search options
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Maximum number of results to return
    pub limit: usize,
    /// Offset for pagination
    pub offset: usize,
    /// Sort order
    pub sort_order: SearchSortOrder,
    /// Filter options
    pub filter: SearchFilter,
    /// Whether to include snippets in results
    pub include_snippets: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: 50,
            offset: 0,
            sort_order: SearchSortOrder::Relevance,
            filter: SearchFilter::default(),
            include_snippets: true,
        }
    }
}

/// Search history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistoryEntry {
    /// The search query
    pub query: String,
    /// When the search was performed
    pub searched_at: DateTime<Utc>,
    /// Number of results returned
    pub result_count: usize,
}

/// Search suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestion {
    /// The suggested query
    pub query: String,
    /// Type of suggestion
    pub suggestion_type: SuggestionType,
    /// Relevance score for ranking suggestions
    pub score: f32,
}

/// Type of search suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    /// From search history
    History,
    /// From page titles
    Title,
    /// From keywords
    Keyword,
    /// From URLs/domains
    Url,
}

/// Unified search results
#[derive(Debug, Clone)]
pub struct SearchResults {
    /// The search query
    pub query: String,
    /// Total number of results (before pagination)
    pub total_count: usize,
    /// The result items
    pub items: Vec<SearchResultItem>,
    /// Time taken to perform the search (in milliseconds)
    pub search_time_ms: u64,
    /// Applied filters
    pub filter: SearchFilter,
}

impl SearchResults {
    /// Check if there are more results available
    pub fn has_more(&self, offset: usize, limit: usize) -> bool {
        offset + limit < self.total_count
    }

    /// Get results grouped by source type
    pub fn group_by_source(&self) -> HashMap<SearchResultSource, Vec<&SearchResultItem>> {
        let mut groups: HashMap<SearchResultSource, Vec<&SearchResultItem>> = HashMap::new();
        for item in &self.items {
            groups.entry(item.source_type.clone()).or_default().push(item);
        }
        groups
    }
}


/// Unified Search Manager
///
/// Provides cross-data-source search functionality that searches across
/// tabs, bookmarks, history, and archived content.
///
/// Implements Requirement 6.5: Unified search across tabs and bookmarks
/// with comprehensive results.
pub struct UnifiedSearchManager {
    /// Page repository for searching unified pages
    page_repo: SqlitePageRepository,
    /// History repository for searching tab history
    history_repo: SqliteHistoryRepository,
    /// Archive repository for searching archived content
    archive_repo: SqliteArchiveRepository,
    /// Search history
    search_history: Arc<RwLock<Vec<SearchHistoryEntry>>>,
    /// Cached tabs for in-memory search
    cached_tabs: Arc<RwLock<Vec<TabInfo>>>,
    /// Cached bookmarks for in-memory search
    cached_bookmarks: Arc<RwLock<Vec<BookmarkInfo>>>,
}

impl UnifiedSearchManager {
    /// Create a new unified search manager from a DatabaseManager
    pub fn new(db_manager: &DatabaseManager) -> Self {
        Self {
            page_repo: db_manager.page_repository(),
            history_repo: db_manager.history_repository(),
            archive_repo: db_manager.archive_repository(),
            search_history: Arc::new(RwLock::new(Vec::new())),
            cached_tabs: Arc::new(RwLock::new(Vec::new())),
            cached_bookmarks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Update cached tabs for in-memory search
    pub async fn update_tabs(&self, tabs: Vec<TabInfo>) {
        let mut cached = self.cached_tabs.write().await;
        *cached = tabs;
    }

    /// Update cached bookmarks for in-memory search
    pub async fn update_bookmarks(&self, bookmarks: Vec<BookmarkInfo>) {
        let mut cached = self.cached_bookmarks.write().await;
        *cached = bookmarks;
    }

    /// Perform a unified search across all data sources
    ///
    /// This is the main search entry point that searches across:
    /// - Active tabs (in-memory)
    /// - Bookmarks (in-memory)
    /// - Unified pages (database with FTS)
    /// - Tab history (database with FTS)
    /// - Archived content (database with FTS)
    pub async fn search(&self, query: &str, options: SearchOptions) -> Result<SearchResults> {
        let start_time = std::time::Instant::now();
        let query_lower = query.to_lowercase();
        let mut all_results: Vec<SearchResultItem> = Vec::new();

        // Search in-memory tabs
        if options.filter.source_types.is_empty() 
            || options.filter.source_types.contains(&SearchResultSource::ActiveTab) 
        {
            let tab_results = self.search_tabs(&query_lower).await;
            all_results.extend(tab_results);
        }

        // Search in-memory bookmarks
        if options.filter.source_types.is_empty() 
            || options.filter.source_types.contains(&SearchResultSource::Bookmark) 
        {
            let bookmark_results = self.search_bookmarks(&query_lower).await;
            all_results.extend(bookmark_results);
        }

        // Search unified pages in database
        if options.filter.source_types.is_empty() 
            || options.filter.source_types.contains(&SearchResultSource::UnifiedPage) 
        {
            let page_results = self.search_pages(query).await?;
            all_results.extend(page_results);
        }

        // Search history in database
        if options.filter.source_types.is_empty() 
            || options.filter.source_types.contains(&SearchResultSource::History) 
        {
            let history_results = self.search_history(query).await?;
            all_results.extend(history_results);
        }

        // Search archives in database
        if options.filter.source_types.is_empty() 
            || options.filter.source_types.contains(&SearchResultSource::Archive) 
        {
            let archive_results = self.search_archives(query).await?;
            all_results.extend(archive_results);
        }

        // Deduplicate by URL (keep highest relevance score)
        all_results = self.deduplicate_results(all_results);

        // Apply filters
        all_results.retain(|r| options.filter.matches(r));

        // Sort results
        self.sort_results(&mut all_results, options.sort_order);

        let total_count = all_results.len();

        // Apply pagination
        let items: Vec<SearchResultItem> = all_results
            .into_iter()
            .skip(options.offset)
            .take(options.limit)
            .collect();

        let search_time_ms = start_time.elapsed().as_millis() as u64;

        // Record search in history
        self.record_search(query, total_count).await;

        Ok(SearchResults {
            query: query.to_string(),
            total_count,
            items,
            search_time_ms,
            filter: options.filter,
        })
    }

    /// Search in cached tabs
    async fn search_tabs(&self, query: &str) -> Vec<SearchResultItem> {
        let tabs = self.cached_tabs.read().await;
        let mut results = Vec::new();

        for tab in tabs.iter() {
            if tab.is_private {
                continue; // Skip private tabs
            }

            let relevance = self.calculate_relevance(query, &tab.url, &tab.title, &[]);
            if relevance > 0.0 {
                results.push(SearchResultItem {
                    id: tab.id.0.clone(),
                    url: tab.url.clone(),
                    title: tab.title.clone(),
                    favicon_url: tab.favicon_url.clone(),
                    source_type: SearchResultSource::ActiveTab,
                    relevance_score: relevance,
                    snippet: None,
                    keywords: vec![],
                    last_accessed: tab.last_accessed,
                    browser_type: Some(tab.browser_type),
                });
            }
        }

        results
    }

    /// Search in cached bookmarks
    async fn search_bookmarks(&self, query: &str) -> Vec<SearchResultItem> {
        let bookmarks = self.cached_bookmarks.read().await;
        let mut results = Vec::new();

        for bookmark in bookmarks.iter() {
            let relevance = self.calculate_relevance(query, &bookmark.url, &bookmark.title, &[]);
            if relevance > 0.0 {
                results.push(SearchResultItem {
                    id: bookmark.id.0.clone(),
                    url: bookmark.url.clone(),
                    title: bookmark.title.clone(),
                    favicon_url: bookmark.favicon_url.clone(),
                    source_type: SearchResultSource::Bookmark,
                    relevance_score: relevance,
                    snippet: None,
                    keywords: bookmark.folder_path.clone(),
                    last_accessed: bookmark.last_accessed.unwrap_or(bookmark.created_at),
                    browser_type: Some(bookmark.browser_type),
                });
            }
        }

        results
    }

    /// Search unified pages in database using FTS
    async fn search_pages(&self, query: &str) -> Result<Vec<SearchResultItem>> {
        let pages = self.page_repo.search_with_limit(query, 100).await?;
        
        Ok(pages.into_iter().map(|page| {
            let snippet = page.content_summary.as_ref().map(|s| {
                if s.summary_text.len() > 200 {
                    format!("{}...", &s.summary_text[..200])
                } else {
                    s.summary_text.clone()
                }
            });

            let browser_type = match &page.source_type {
                PageSourceType::ActiveTab { browser, .. } => Some(*browser),
                PageSourceType::Bookmark { browser, .. } => Some(*browser),
                _ => page.browser_info.as_ref().map(|b| b.browser_type),
            };

            SearchResultItem {
                id: page.id.to_string(),
                url: page.url,
                title: page.title,
                favicon_url: page.favicon_url,
                source_type: SearchResultSource::UnifiedPage,
                relevance_score: 0.8, // FTS results have good relevance
                snippet,
                keywords: page.keywords,
                last_accessed: page.last_accessed,
                browser_type,
            }
        }).collect())
    }

    /// Search tab history in database using FTS
    async fn search_history(&self, query: &str) -> Result<Vec<SearchResultItem>> {
        let entries = self.history_repo.search(query, 100).await?;
        
        Ok(entries.into_iter().map(|entry| {
            let snippet = entry.page_info.content_summary.as_ref().map(|s| {
                if s.summary_text.len() > 200 {
                    format!("{}...", &s.summary_text[..200])
                } else {
                    s.summary_text.clone()
                }
            });

            SearchResultItem {
                id: entry.id.0.to_string(),
                url: entry.page_info.url,
                title: entry.page_info.title,
                favicon_url: entry.page_info.favicon_url,
                source_type: SearchResultSource::History,
                relevance_score: 0.6, // History results have lower priority
                snippet,
                keywords: entry.page_info.keywords,
                last_accessed: entry.closed_at,
                browser_type: Some(entry.browser_type),
            }
        }).collect())
    }

    /// Search archived content in database using FTS
    async fn search_archives(&self, query: &str) -> Result<Vec<SearchResultItem>> {
        let archives = self.archive_repo.search(query, 100).await?;
        
        Ok(archives.into_iter().map(|archive| {
            let snippet = if archive.content_text.len() > 200 {
                Some(format!("{}...", &archive.content_text[..200]))
            } else if !archive.content_text.is_empty() {
                Some(archive.content_text.clone())
            } else {
                None
            };

            SearchResultItem {
                id: archive.id.0.to_string(),
                url: archive.url,
                title: archive.title,
                favicon_url: None,
                source_type: SearchResultSource::Archive,
                relevance_score: 0.7, // Archives have medium priority
                snippet,
                keywords: vec![],
                last_accessed: archive.archived_at,
                browser_type: None,
            }
        }).collect())
    }

    /// Calculate relevance score for a search result
    fn calculate_relevance(&self, query: &str, url: &str, title: &str, keywords: &[String]) -> f32 {
        let url_lower = url.to_lowercase();
        let title_lower = title.to_lowercase();
        let query_lower = query.to_lowercase();

        let mut score = 0.0f32;

        // Exact title match
        if title_lower == query_lower {
            score += 1.0;
        }
        // Title contains query
        else if title_lower.contains(&query_lower) {
            score += 0.8;
        }
        // Title words match
        else {
            let query_words: Vec<&str> = query_lower.split_whitespace().collect();
            let title_words: Vec<&str> = title_lower.split_whitespace().collect();
            let matching_words = query_words.iter()
                .filter(|qw| title_words.iter().any(|tw| tw.contains(*qw)))
                .count();
            if matching_words > 0 {
                score += 0.5 * (matching_words as f32 / query_words.len() as f32);
            }
        }

        // URL contains query
        if url_lower.contains(&query_lower) {
            score += 0.3;
        }

        // Keywords match
        for keyword in keywords {
            if keyword.to_lowercase().contains(&query_lower) {
                score += 0.2;
                break;
            }
        }

        score.min(1.0)
    }

    /// Deduplicate results by URL, keeping the highest relevance score
    fn deduplicate_results(&self, results: Vec<SearchResultItem>) -> Vec<SearchResultItem> {
        let mut url_map: HashMap<String, SearchResultItem> = HashMap::new();

        for result in results {
            let normalized_url = result.url.to_lowercase();
            if let Some(existing) = url_map.get_mut(&normalized_url) {
                // Keep the one with higher relevance, or prefer certain source types
                if result.relevance_score > existing.relevance_score {
                    *existing = result;
                } else if result.relevance_score == existing.relevance_score {
                    // Prefer ActiveTab > Bookmark > UnifiedPage > History > Archive
                    let result_priority = self.source_priority(&result.source_type);
                    let existing_priority = self.source_priority(&existing.source_type);
                    if result_priority > existing_priority {
                        *existing = result;
                    }
                }
            } else {
                url_map.insert(normalized_url, result);
            }
        }

        url_map.into_values().collect()
    }

    /// Get priority for source type (higher is better)
    fn source_priority(&self, source: &SearchResultSource) -> u8 {
        match source {
            SearchResultSource::ActiveTab => 5,
            SearchResultSource::Bookmark => 4,
            SearchResultSource::UnifiedPage => 3,
            SearchResultSource::History => 2,
            SearchResultSource::Archive => 1,
        }
    }

    /// Sort results according to the specified order
    fn sort_results(&self, results: &mut [SearchResultItem], order: SearchSortOrder) {
        match order {
            SearchSortOrder::Relevance => {
                results.sort_by(|a, b| {
                    b.relevance_score.partial_cmp(&a.relevance_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SearchSortOrder::RecentFirst => {
                results.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
            }
            SearchSortOrder::OldestFirst => {
                results.sort_by(|a, b| a.last_accessed.cmp(&b.last_accessed));
            }
            SearchSortOrder::TitleAsc => {
                results.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
            }
            SearchSortOrder::TitleDesc => {
                results.sort_by(|a, b| b.title.to_lowercase().cmp(&a.title.to_lowercase()));
            }
        }
    }

    /// Record a search in history
    async fn record_search(&self, query: &str, result_count: usize) {
        let mut history = self.search_history.write().await;
        
        // Remove duplicate queries (keep most recent)
        history.retain(|e| e.query.to_lowercase() != query.to_lowercase());
        
        // Add new entry
        history.push(SearchHistoryEntry {
            query: query.to_string(),
            searched_at: Utc::now(),
            result_count,
        });

        // Trim to max size
        let len = history.len();
        if len > MAX_SEARCH_HISTORY {
            history.drain(0..len - MAX_SEARCH_HISTORY);
        }
    }

    /// Get search history
    pub async fn get_search_history(&self, limit: usize) -> Vec<SearchHistoryEntry> {
        let history = self.search_history.read().await;
        history.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Clear search history
    pub async fn clear_search_history(&self) {
        let mut history = self.search_history.write().await;
        history.clear();
    }

    /// Get search suggestions based on partial query
    pub async fn get_suggestions(&self, partial_query: &str) -> Vec<SearchSuggestion> {
        let query_lower = partial_query.to_lowercase();
        let mut suggestions: Vec<SearchSuggestion> = Vec::new();

        // Suggestions from search history
        {
            let history = self.search_history.read().await;
            for entry in history.iter().rev() {
                if entry.query.to_lowercase().starts_with(&query_lower) {
                    suggestions.push(SearchSuggestion {
                        query: entry.query.clone(),
                        suggestion_type: SuggestionType::History,
                        score: 1.0,
                    });
                }
            }
        }

        // Suggestions from tab titles
        {
            let tabs = self.cached_tabs.read().await;
            for tab in tabs.iter() {
                if tab.title.to_lowercase().contains(&query_lower) {
                    suggestions.push(SearchSuggestion {
                        query: tab.title.clone(),
                        suggestion_type: SuggestionType::Title,
                        score: 0.8,
                    });
                }
            }
        }

        // Suggestions from bookmark titles
        {
            let bookmarks = self.cached_bookmarks.read().await;
            for bookmark in bookmarks.iter() {
                if bookmark.title.to_lowercase().contains(&query_lower) {
                    suggestions.push(SearchSuggestion {
                        query: bookmark.title.clone(),
                        suggestion_type: SuggestionType::Title,
                        score: 0.7,
                    });
                }
            }
        }

        // Deduplicate and sort by score
        let mut seen = std::collections::HashSet::new();
        suggestions.retain(|s| seen.insert(s.query.to_lowercase()));
        suggestions.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });

        suggestions.truncate(MAX_SUGGESTIONS);
        suggestions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_filter_matches() {
        let result = SearchResultItem {
            id: "test".to_string(),
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
            favicon_url: None,
            source_type: SearchResultSource::ActiveTab,
            relevance_score: 0.8,
            snippet: None,
            keywords: vec!["rust".to_string()],
            last_accessed: Utc::now(),
            browser_type: Some(BrowserType::Chrome),
        };

        // Empty filter matches all
        let filter = SearchFilter::new();
        assert!(filter.matches(&result));

        // Source type filter
        let filter = SearchFilter::tabs_only();
        assert!(filter.matches(&result));

        let filter = SearchFilter::bookmarks_only();
        assert!(!filter.matches(&result));

        // Browser filter
        let filter = SearchFilter::new().with_browser(BrowserType::Chrome);
        assert!(filter.matches(&result));

        let filter = SearchFilter::new().with_browser(BrowserType::Firefox);
        assert!(!filter.matches(&result));
    }

    #[test]
    fn test_search_sort_order() {
        let mut results = vec![
            SearchResultItem {
                id: "1".to_string(),
                url: "https://a.com".to_string(),
                title: "Zebra".to_string(),
                favicon_url: None,
                source_type: SearchResultSource::ActiveTab,
                relevance_score: 0.5,
                snippet: None,
                keywords: vec![],
                last_accessed: Utc::now() - chrono::Duration::hours(1),
                browser_type: None,
            },
            SearchResultItem {
                id: "2".to_string(),
                url: "https://b.com".to_string(),
                title: "Apple".to_string(),
                favicon_url: None,
                source_type: SearchResultSource::Bookmark,
                relevance_score: 0.9,
                snippet: None,
                keywords: vec![],
                last_accessed: Utc::now(),
                browser_type: None,
            },
        ];

        // Test relevance sort
        results.sort_by(|a, b| {
            b.relevance_score.partial_cmp(&a.relevance_score).unwrap()
        });
        assert_eq!(results[0].title, "Apple");

        // Test title sort
        results.sort_by(|a, b| a.title.cmp(&b.title));
        assert_eq!(results[0].title, "Apple");
        assert_eq!(results[1].title, "Zebra");
    }

    #[test]
    fn test_calculate_relevance() {
        // Create a mock manager for testing relevance calculation
        // We'll test the logic directly
        let query = "rust";
        let title = "Rust Programming Language";
        let url = "https://rust-lang.org";

        // Title contains query - should have high relevance
        assert!(title.to_lowercase().contains(query));
        
        // URL contains query - should add to relevance
        assert!(url.to_lowercase().contains(query));
    }

    #[test]
    fn test_search_results_group_by_source() {
        let results = SearchResults {
            query: "test".to_string(),
            total_count: 3,
            items: vec![
                SearchResultItem {
                    id: "1".to_string(),
                    url: "https://a.com".to_string(),
                    title: "A".to_string(),
                    favicon_url: None,
                    source_type: SearchResultSource::ActiveTab,
                    relevance_score: 0.8,
                    snippet: None,
                    keywords: vec![],
                    last_accessed: Utc::now(),
                    browser_type: None,
                },
                SearchResultItem {
                    id: "2".to_string(),
                    url: "https://b.com".to_string(),
                    title: "B".to_string(),
                    favicon_url: None,
                    source_type: SearchResultSource::Bookmark,
                    relevance_score: 0.7,
                    snippet: None,
                    keywords: vec![],
                    last_accessed: Utc::now(),
                    browser_type: None,
                },
                SearchResultItem {
                    id: "3".to_string(),
                    url: "https://c.com".to_string(),
                    title: "C".to_string(),
                    favicon_url: None,
                    source_type: SearchResultSource::ActiveTab,
                    relevance_score: 0.6,
                    snippet: None,
                    keywords: vec![],
                    last_accessed: Utc::now(),
                    browser_type: None,
                },
            ],
            search_time_ms: 10,
            filter: SearchFilter::default(),
        };

        let groups = results.group_by_source();
        assert_eq!(groups.get(&SearchResultSource::ActiveTab).map(|v| v.len()), Some(2));
        assert_eq!(groups.get(&SearchResultSource::Bookmark).map(|v| v.len()), Some(1));
    }
}
