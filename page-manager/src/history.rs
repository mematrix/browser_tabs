//! Tab History Manager Module
//!
//! Provides functionality for managing closed tab history, including:
//! - Automatic saving of closed tab information
//! - History record query and filtering
//! - Rich history information with content summaries and tags
//! - Tab restoration to specified browsers
//! - Automatic cleanup strategies based on time and importance
//! - History export and backup functionality
//!
//! # Requirements Implemented
//! - 7.1: Auto-save closed tab complete information to history
//! - 7.2: Preserve page title, URL, close time, and analyzed content summary
//! - 7.3: Display richer information than browser history including content preview and tags
//! - 7.4: Restore history tabs in specified browser
//! - 7.5: Provide automatic cleanup strategy based on time and importance

use web_page_manager_core::*;
use browser_connector::{TabEvent, TabMonitor, BrowserConnector};
use std::collections::HashMap;
use std::sync::Arc;
use std::path::Path;
use tokio::sync::RwLock;
use chrono::{DateTime, Duration, Utc};
use tracing::{debug, info, warn};
use serde::{Deserialize, Serialize};

/// Configuration for the Tab History Manager
#[derive(Debug, Clone)]
pub struct TabHistoryManagerConfig {
    /// Maximum number of history entries to keep in memory cache
    pub max_cache_entries: usize,
    /// Whether to automatically save closed tabs
    pub auto_save_closed_tabs: bool,
    /// Minimum time a tab must be open before saving to history (in seconds)
    pub min_tab_lifetime_secs: u64,
    /// Whether to save internal browser pages (chrome://, about:, etc.)
    pub save_internal_pages: bool,
    /// Default retention policy for automatic cleanup
    pub default_retention_policy: RetentionPolicy,
    /// Whether to run automatic cleanup on startup
    pub auto_cleanup_on_startup: bool,
    /// Interval for automatic cleanup in hours (0 = disabled)
    pub auto_cleanup_interval_hours: u32,
}

impl Default for TabHistoryManagerConfig {
    fn default() -> Self {
        Self {
            max_cache_entries: 1000,
            auto_save_closed_tabs: true,
            min_tab_lifetime_secs: 5,
            save_internal_pages: false,
            default_retention_policy: RetentionPolicy::default(),
            auto_cleanup_on_startup: true,
            auto_cleanup_interval_hours: 24,
        }
    }
}

/// Statistics about the history manager
#[derive(Debug, Clone, Default)]
pub struct HistoryManagerStats {
    /// Total number of history entries in cache
    pub cached_entries: usize,
    /// Number of entries by browser type
    pub entries_by_browser: HashMap<BrowserType, usize>,
    /// Number of entries saved in current session
    pub session_saves: usize,
    /// Number of entries restored in current session
    pub session_restores: usize,
    /// Oldest entry timestamp
    pub oldest_entry: Option<DateTime<Utc>>,
    /// Newest entry timestamp
    pub newest_entry: Option<DateTime<Utc>>,
    /// Number of entries cleaned up in current session
    pub session_cleanups: usize,
    /// Last cleanup timestamp
    pub last_cleanup: Option<DateTime<Utc>>,
}

/// Result of a tab restoration operation
#[derive(Debug, Clone)]
pub struct RestoreResult {
    /// The history entry that was restored
    pub history_id: HistoryId,
    /// The new tab ID in the target browser
    pub new_tab_id: Option<TabId>,
    /// The browser where the tab was restored
    pub target_browser: BrowserType,
    /// Whether the restoration was successful
    pub success: bool,
    /// Error message if restoration failed
    pub error: Option<String>,
    /// Timestamp of the restoration attempt
    pub restored_at: DateTime<Utc>,
}

/// Cleanup result containing statistics about the cleanup operation
#[derive(Debug, Clone)]
pub struct CleanupResult {
    /// Number of entries deleted due to age
    pub deleted_by_age: usize,
    /// Number of entries deleted due to max entries limit
    pub deleted_by_limit: usize,
    /// Number of important entries preserved
    pub preserved_important: usize,
    /// Total entries remaining after cleanup
    pub remaining_entries: usize,
    /// Timestamp of the cleanup
    pub cleaned_at: DateTime<Utc>,
}

/// Export format for history data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    /// JSON format (default)
    Json,
    /// CSV format for spreadsheet compatibility
    Csv,
    /// HTML format for browser viewing
    Html,
}

/// Exported history data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedHistory {
    /// Export metadata
    pub metadata: ExportMetadata,
    /// The history entries
    pub entries: Vec<HistoryEntry>,
}

/// Metadata for exported history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportMetadata {
    /// Export timestamp
    pub exported_at: DateTime<Utc>,
    /// Application version
    pub app_version: String,
    /// Total number of entries
    pub entry_count: usize,
    /// Date range of entries
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Export format used
    pub format: String,
}

/// Tab History Manager
///
/// Manages the history of closed tabs, providing rich information
/// beyond what browser history typically offers.
///
/// Implements Requirements 7.1, 7.2, and 7.3.
pub struct TabHistoryManager {
    config: TabHistoryManagerConfig,
    /// In-memory cache of history entries
    history_cache: Arc<RwLock<Vec<HistoryEntry>>>,
    /// Map of page content summaries by URL for enrichment
    content_summaries: Arc<RwLock<HashMap<String, ContentSummary>>>,
    /// Session statistics
    stats: Arc<RwLock<HistoryManagerStats>>,
    /// Reference to tab monitor for event subscription
    tab_monitor: Option<Arc<TabMonitor>>,
}

impl TabHistoryManager {
    /// Create a new Tab History Manager with default configuration
    pub fn new() -> Self {
        Self::with_config(TabHistoryManagerConfig::default())
    }

    /// Create a new Tab History Manager with custom configuration
    pub fn with_config(config: TabHistoryManagerConfig) -> Self {
        Self {
            config,
            history_cache: Arc::new(RwLock::new(Vec::new())),
            content_summaries: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(HistoryManagerStats::default())),
            tab_monitor: None,
        }
    }

    /// Set the tab monitor for event subscription
    pub fn set_tab_monitor(&mut self, monitor: Arc<TabMonitor>) {
        self.tab_monitor = Some(monitor);
    }

    /// Get the current configuration
    pub fn config(&self) -> &TabHistoryManagerConfig {
        &self.config
    }

    // =========================================================================
    // Tab Close Event Handling (Requirement 7.1)
    // =========================================================================

    /// Process tab events and save closed tabs to history
    ///
    /// This method should be called with events from the TabMonitor.
    /// It filters for Closed events and saves them to history.
    ///
    /// Implements Requirement 7.1: Auto-save closed tab information
    pub async fn process_tab_events(&self, events: &[TabEvent]) -> Vec<HistoryId> {
        let mut saved_ids = Vec::new();

        for event in events {
            if let TabEvent::Closed {
                tab_id,
                browser_type,
                timestamp,
                last_known_info,
            } = event
            {
                if let Some(tab_info) = last_known_info {
                    // Check if we should save this tab
                    if self.should_save_tab(tab_info) {
                        if let Ok(history_id) = self
                            .save_closed_tab(tab_info.clone(), *timestamp)
                            .await
                        {
                            saved_ids.push(history_id);
                            debug!(
                                "Saved closed tab to history: {:?} from {:?}",
                                tab_id, browser_type
                            );
                        }
                    }
                }
            }
        }

        saved_ids
    }

    /// Check if a tab should be saved to history
    fn should_save_tab(&self, tab: &TabInfo) -> bool {
        // Skip private tabs
        if tab.is_private {
            return false;
        }

        // Skip internal pages if configured
        if !self.config.save_internal_pages && self.is_internal_page(&tab.url) {
            return false;
        }

        // Check minimum lifetime
        let lifetime = Utc::now() - tab.created_at;
        if lifetime.num_seconds() < self.config.min_tab_lifetime_secs as i64 {
            return false;
        }

        true
    }

    /// Check if a URL is a browser internal page
    fn is_internal_page(&self, url: &str) -> bool {
        let lower_url = url.to_lowercase();
        lower_url.starts_with("chrome://")
            || lower_url.starts_with("edge://")
            || lower_url.starts_with("about:")
            || lower_url.starts_with("chrome-extension://")
            || lower_url.starts_with("moz-extension://")
            || lower_url.starts_with("file://")
    }

    // =========================================================================
    // History Record Management (Requirement 7.2)
    // =========================================================================

    /// Save a closed tab to history
    ///
    /// Creates a complete history entry with page title, URL, close time,
    /// and any available content summary.
    ///
    /// Implements Requirement 7.2: Preserve complete tab information
    pub async fn save_closed_tab(
        &self,
        tab: TabInfo,
        close_time: DateTime<Utc>,
    ) -> Result<HistoryId> {
        let history_id = HistoryId::new();

        // Try to get content summary for this URL
        let content_summary = self.get_content_summary(&tab.url).await;

        // Create the page info for the history entry
        let page_info = UnifiedPageInfo {
            id: uuid::Uuid::new_v4(),
            url: tab.url.clone(),
            title: tab.title.clone(),
            favicon_url: tab.favicon_url.clone(),
            content_summary,
            keywords: vec![],
            category: None,
            source_type: PageSourceType::ClosedTab {
                history_id: history_id.clone(),
            },
            browser_info: Some(BrowserInstance {
                browser_type: tab.browser_type,
                version: String::new(),
                process_id: 0,
                debug_port: None,
                profile_path: None,
            }),
            tab_info: Some(tab.clone()),
            bookmark_info: None,
            created_at: tab.created_at,
            last_accessed: close_time,
            access_count: 0,
        };

        // Create session info
        let session_info = SessionInfo {
            session_id: uuid::Uuid::new_v4().to_string(),
            window_id: None,
            tab_index: None,
            scroll_position: None,
        };

        let entry = HistoryEntry {
            id: history_id.clone(),
            page_info,
            browser_type: tab.browser_type,
            tab_id: Some(tab.id),
            closed_at: close_time,
            session_info: Some(session_info),
        };

        // Add to cache
        self.add_to_cache(entry).await;

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.session_saves += 1;
        }

        info!("Saved tab to history: {} - {}", tab.title, tab.url);

        Ok(history_id)
    }

    /// Add a history entry to the cache
    async fn add_to_cache(&self, entry: HistoryEntry) {
        let mut cache = self.history_cache.write().await;

        // Add the new entry
        cache.push(entry);

        // Trim cache if needed
        while cache.len() > self.config.max_cache_entries {
            cache.remove(0);
        }

        // Update stats
        drop(cache);
        self.update_cache_stats().await;
    }

    /// Update cache statistics
    async fn update_cache_stats(&self) {
        let cache = self.history_cache.read().await;
        let mut stats = self.stats.write().await;

        stats.cached_entries = cache.len();
        stats.entries_by_browser.clear();

        for entry in cache.iter() {
            *stats
                .entries_by_browser
                .entry(entry.browser_type)
                .or_insert(0) += 1;
        }

        stats.oldest_entry = cache.first().map(|e| e.closed_at);
        stats.newest_entry = cache.last().map(|e| e.closed_at);
    }

    /// Register a content summary for a URL
    ///
    /// This allows the history manager to enrich history entries
    /// with content summaries when tabs are closed.
    pub async fn register_content_summary(&self, url: &str, summary: ContentSummary) {
        let mut summaries = self.content_summaries.write().await;
        summaries.insert(url.to_string(), summary);
    }

    /// Get content summary for a URL
    async fn get_content_summary(&self, url: &str) -> Option<ContentSummary> {
        let summaries = self.content_summaries.read().await;
        summaries.get(url).cloned()
    }

    // =========================================================================
    // History Query and Filtering (Requirement 7.3)
    // =========================================================================

    /// Get history entries with filtering
    ///
    /// Provides rich history information including content preview and tags.
    ///
    /// Implements Requirement 7.3: Display richer information than browser history
    pub async fn get_history(&self, filter: &HistoryFilter) -> Vec<HistoryEntry> {
        let cache = self.history_cache.read().await;

        let mut results: Vec<HistoryEntry> = cache
            .iter()
            .filter(|entry| self.matches_filter(entry, filter))
            .cloned()
            .collect();

        // Sort by closed_at descending (most recent first)
        results.sort_by(|a, b| b.closed_at.cmp(&a.closed_at));

        // Apply offset
        if let Some(offset) = filter.offset {
            if offset < results.len() {
                results = results.into_iter().skip(offset).collect();
            } else {
                results.clear();
            }
        }

        // Apply limit
        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }

        results
    }

    /// Check if a history entry matches the filter
    fn matches_filter(&self, entry: &HistoryEntry, filter: &HistoryFilter) -> bool {
        // Browser type filter
        if let Some(browser) = filter.browser_type {
            if entry.browser_type != browser {
                return false;
            }
        }

        // Date range filter
        if let Some(from) = filter.from_date {
            if entry.closed_at < from {
                return false;
            }
        }

        if let Some(to) = filter.to_date {
            if entry.closed_at > to {
                return false;
            }
        }

        // URL pattern filter
        if let Some(ref pattern) = filter.url_pattern {
            if !entry
                .page_info
                .url
                .to_lowercase()
                .contains(&pattern.to_lowercase())
            {
                return false;
            }
        }

        // Title pattern filter
        if let Some(ref pattern) = filter.title_pattern {
            if !entry
                .page_info
                .title
                .to_lowercase()
                .contains(&pattern.to_lowercase())
            {
                return false;
            }
        }

        true
    }

    /// Get a history entry by ID
    pub async fn get_by_id(&self, id: &HistoryId) -> Option<HistoryEntry> {
        let cache = self.history_cache.read().await;
        cache.iter().find(|e| &e.id == id).cloned()
    }

    /// Get recent history entries
    pub async fn get_recent(&self, count: usize) -> Vec<HistoryEntry> {
        self.get_history(&HistoryFilter {
            limit: Some(count),
            ..Default::default()
        })
        .await
    }

    /// Get history entries for a specific browser
    pub async fn get_by_browser(&self, browser_type: BrowserType) -> Vec<HistoryEntry> {
        self.get_history(&HistoryFilter {
            browser_type: Some(browser_type),
            ..Default::default()
        })
        .await
    }

    /// Get history entries within a time range
    pub async fn get_in_time_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Vec<HistoryEntry> {
        self.get_history(&HistoryFilter {
            from_date: Some(from),
            to_date: Some(to),
            ..Default::default()
        })
        .await
    }

    /// Search history by text query
    ///
    /// Searches across title, URL, and content summary.
    pub async fn search(&self, query: &str, limit: usize) -> Vec<HistoryEntry> {
        let cache = self.history_cache.read().await;
        let query_lower = query.to_lowercase();

        let mut results: Vec<HistoryEntry> = cache
            .iter()
            .filter(|entry| {
                entry.page_info.title.to_lowercase().contains(&query_lower)
                    || entry.page_info.url.to_lowercase().contains(&query_lower)
                    || entry
                        .page_info
                        .content_summary
                        .as_ref()
                        .map(|s| s.summary_text.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
                    || entry
                        .page_info
                        .keywords
                        .iter()
                        .any(|k| k.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect();

        // Sort by relevance (entries with query in title first, then by date)
        results.sort_by(|a, b| {
            let a_title_match = a.page_info.title.to_lowercase().contains(&query_lower);
            let b_title_match = b.page_info.title.to_lowercase().contains(&query_lower);

            match (a_title_match, b_title_match) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.closed_at.cmp(&a.closed_at),
            }
        });

        results.truncate(limit);
        results
    }

    /// Get history entries closed within the last N minutes
    pub async fn get_recently_closed(&self, within_minutes: i64) -> Vec<HistoryEntry> {
        let cutoff = Utc::now() - Duration::minutes(within_minutes);
        self.get_history(&HistoryFilter {
            from_date: Some(cutoff),
            ..Default::default()
        })
        .await
    }

    /// Count history entries matching a filter
    pub async fn count(&self, filter: &HistoryFilter) -> usize {
        let cache = self.history_cache.read().await;
        cache
            .iter()
            .filter(|entry| self.matches_filter(entry, filter))
            .count()
    }

    /// Get total count of history entries in cache
    pub async fn total_count(&self) -> usize {
        self.history_cache.read().await.len()
    }

    // =========================================================================
    // History Management
    // =========================================================================

    /// Delete a history entry by ID
    pub async fn delete(&self, id: &HistoryId) -> bool {
        let mut cache = self.history_cache.write().await;
        let initial_len = cache.len();
        cache.retain(|e| &e.id != id);
        let deleted = cache.len() < initial_len;

        if deleted {
            drop(cache);
            self.update_cache_stats().await;
            debug!("Deleted history entry: {:?}", id);
        }

        deleted
    }

    /// Delete history entries older than a timestamp
    pub async fn delete_older_than(&self, timestamp: DateTime<Utc>) -> usize {
        let mut cache = self.history_cache.write().await;
        let initial_len = cache.len();
        cache.retain(|e| e.closed_at >= timestamp);
        let deleted = initial_len - cache.len();

        if deleted > 0 {
            drop(cache);
            self.update_cache_stats().await;
            info!("Deleted {} old history entries", deleted);
        }

        deleted
    }

    /// Apply retention policy to clean up old history
    ///
    /// This implements automatic cleanup based on age and entry count.
    pub async fn apply_retention_policy(&self, policy: &RetentionPolicy) -> usize {
        let mut total_deleted = 0;

        // Delete entries older than max_age_days
        let cutoff = Utc::now() - Duration::days(policy.max_age_days as i64);
        total_deleted += self.delete_older_than(cutoff).await;

        // Trim to max_entries if needed
        let mut cache = self.history_cache.write().await;
        if cache.len() > policy.max_entries {
            // Sort by importance if preserving important entries
            if policy.preserve_important {
                cache.sort_by(|a, b| {
                    let a_importance = self.calculate_importance(a);
                    let b_importance = self.calculate_importance(b);
                    b_importance
                        .partial_cmp(&a_importance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }

            let to_remove = cache.len() - policy.max_entries;
            cache.truncate(policy.max_entries);
            total_deleted += to_remove;
        }

        if total_deleted > 0 {
            drop(cache);
            self.update_cache_stats().await;
            info!(
                "Applied retention policy, deleted {} entries",
                total_deleted
            );
        }

        total_deleted
    }

    /// Calculate importance score for a history entry
    fn calculate_importance(&self, entry: &HistoryEntry) -> f32 {
        let mut score = 0.0;

        // Entries with content summaries are more important
        if entry.page_info.content_summary.is_some() {
            score += 0.3;
        }

        // Entries with keywords are more important
        if !entry.page_info.keywords.is_empty() {
            score += 0.2;
        }

        // More recent entries are more important
        let age_hours = (Utc::now() - entry.closed_at).num_hours() as f32;
        let recency_score = 1.0 / (1.0 + age_hours / 24.0);
        score += recency_score * 0.5;

        score
    }

    /// Clear all history entries from cache
    pub async fn clear(&self) {
        let mut cache = self.history_cache.write().await;
        cache.clear();
        drop(cache);

        self.update_cache_stats().await;
        info!("Cleared all history entries");
    }

    // =========================================================================
    // Statistics
    // =========================================================================

    /// Get history manager statistics
    pub async fn get_stats(&self) -> HistoryManagerStats {
        self.stats.read().await.clone()
    }

    /// Get entries grouped by domain
    pub async fn get_entries_by_domain(&self) -> HashMap<String, Vec<HistoryEntry>> {
        let cache = self.history_cache.read().await;
        let mut grouped: HashMap<String, Vec<HistoryEntry>> = HashMap::new();

        for entry in cache.iter() {
            let domain = self.extract_domain(&entry.page_info.url);
            grouped
                .entry(domain)
                .or_default()
                .push(entry.clone());
        }

        grouped
    }

    /// Extract domain from URL
    fn extract_domain(&self, url: &str) -> String {
        url::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string())
    }

    /// Get the most frequently closed domains
    pub async fn get_top_domains(&self, count: usize) -> Vec<(String, usize)> {
        let grouped = self.get_entries_by_domain().await;
        let mut domain_counts: Vec<(String, usize)> = grouped
            .into_iter()
            .map(|(domain, entries)| (domain, entries.len()))
            .collect();

        domain_counts.sort_by(|a, b| b.1.cmp(&a.1));
        domain_counts.truncate(count);
        domain_counts
    }

    // =========================================================================
    // Tab Restoration (Requirement 7.4)
    // =========================================================================

    /// Restore a history tab in the specified browser
    ///
    /// This method reopens a previously closed tab in the target browser.
    /// If the target browser is not specified, it uses the original browser.
    ///
    /// Implements Requirement 7.4: Restore history tabs in specified browser
    pub async fn restore_tab<C: BrowserConnector>(
        &self,
        history_id: &HistoryId,
        connector: &C,
    ) -> Result<RestoreResult> {
        // Get the history entry
        let entry = self.get_by_id(history_id).await.ok_or_else(|| {
            WebPageManagerError::History {
                source: HistoryError::EntryNotFound {
                    history_id: history_id.0.to_string(),
                },
            }
        })?;

        let target_browser = connector.browser_type();
        let url = &entry.page_info.url;

        // Attempt to create the tab in the target browser
        match connector.create_tab(url).await {
            Ok(new_tab_id) => {
                // Update stats
                {
                    let mut stats = self.stats.write().await;
                    stats.session_restores += 1;
                }

                info!(
                    "Restored tab from history: {} -> {:?}",
                    entry.page_info.title, target_browser
                );

                Ok(RestoreResult {
                    history_id: history_id.clone(),
                    new_tab_id: Some(new_tab_id),
                    target_browser,
                    success: true,
                    error: None,
                    restored_at: Utc::now(),
                })
            }
            Err(e) => {
                warn!(
                    "Failed to restore tab: {} - {}",
                    entry.page_info.title,
                    e
                );

                Ok(RestoreResult {
                    history_id: history_id.clone(),
                    new_tab_id: None,
                    target_browser,
                    success: false,
                    error: Some(e.to_string()),
                    restored_at: Utc::now(),
                })
            }
        }
    }

    /// Restore multiple history tabs in batch
    ///
    /// This method restores multiple tabs at once, returning results for each.
    pub async fn restore_tabs_batch<C: BrowserConnector>(
        &self,
        history_ids: &[HistoryId],
        connector: &C,
    ) -> Vec<RestoreResult> {
        let mut results = Vec::with_capacity(history_ids.len());

        for history_id in history_ids {
            let result = self.restore_tab(history_id, connector).await;
            match result {
                Ok(r) => results.push(r),
                Err(e) => results.push(RestoreResult {
                    history_id: history_id.clone(),
                    new_tab_id: None,
                    target_browser: connector.browser_type(),
                    success: false,
                    error: Some(e.to_string()),
                    restored_at: Utc::now(),
                }),
            }
        }

        results
    }

    /// Get the URL for a history entry (for manual restoration)
    ///
    /// This is useful when automatic restoration fails and the user
    /// needs to manually open the URL.
    pub async fn get_restore_url(&self, history_id: &HistoryId) -> Option<String> {
        self.get_by_id(history_id)
            .await
            .map(|entry| entry.page_info.url)
    }

    /// Get URLs for multiple history entries
    pub async fn get_restore_urls(&self, history_ids: &[HistoryId]) -> Vec<(HistoryId, Option<String>)> {
        let mut results = Vec::with_capacity(history_ids.len());

        for history_id in history_ids {
            let url = self.get_restore_url(history_id).await;
            results.push((history_id.clone(), url));
        }

        results
    }

    // =========================================================================
    // Automatic Cleanup (Requirement 7.5)
    // =========================================================================

    /// Run automatic cleanup based on the configured retention policy
    ///
    /// This method applies the default retention policy to clean up old
    /// and less important history entries.
    ///
    /// Implements Requirement 7.5: Automatic cleanup based on time and importance
    pub async fn run_auto_cleanup(&self) -> CleanupResult {
        let policy = &self.config.default_retention_policy;
        self.cleanup_with_policy(policy).await
    }

    /// Run cleanup with a specific retention policy
    ///
    /// This provides more control over the cleanup process.
    pub async fn cleanup_with_policy(&self, policy: &RetentionPolicy) -> CleanupResult {
        let deleted_by_age: usize;
        let mut deleted_by_limit = 0;
        let mut preserved_important = 0;

        // Step 1: Delete entries older than max_age_days
        let cutoff = Utc::now() - Duration::days(policy.max_age_days as i64);
        
        {
            let mut cache = self.history_cache.write().await;
            let initial_len = cache.len();

            if policy.preserve_important {
                // Keep important entries even if old
                let mut temp_preserved = 0;
                cache.retain(|e| {
                    if e.closed_at >= cutoff {
                        true
                    } else {
                        let importance = self.calculate_importance(e);
                        if importance >= policy.importance_threshold {
                            temp_preserved += 1;
                            true
                        } else {
                            false
                        }
                    }
                });
                preserved_important = temp_preserved;
                deleted_by_age = initial_len.saturating_sub(cache.len());
            } else {
                cache.retain(|e| e.closed_at >= cutoff);
                deleted_by_age = initial_len.saturating_sub(cache.len());
            }
        }

        // Step 2: Trim to max_entries if needed
        {
            let mut cache = self.history_cache.write().await;

            if cache.len() > policy.max_entries {
                if policy.preserve_important {
                    // Sort by importance (descending) to keep most important
                    cache.sort_by(|a, b| {
                        let a_importance = self.calculate_importance(a);
                        let b_importance = self.calculate_importance(b);
                        b_importance
                            .partial_cmp(&a_importance)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                }

                let to_remove = cache.len() - policy.max_entries;
                cache.truncate(policy.max_entries);
                deleted_by_limit = to_remove;
            }
        }

        // Update stats
        let remaining_entries = {
            let cache = self.history_cache.read().await;
            cache.len()
        };

        {
            let mut stats = self.stats.write().await;
            stats.session_cleanups += deleted_by_age + deleted_by_limit;
            stats.last_cleanup = Some(Utc::now());
            stats.cached_entries = remaining_entries;
        }

        self.update_cache_stats().await;

        let result = CleanupResult {
            deleted_by_age,
            deleted_by_limit,
            preserved_important,
            remaining_entries,
            cleaned_at: Utc::now(),
        };

        info!(
            "Cleanup completed: {} by age, {} by limit, {} preserved, {} remaining",
            deleted_by_age, deleted_by_limit, preserved_important, remaining_entries
        );

        result
    }

    /// Check if cleanup is needed based on current state
    pub async fn needs_cleanup(&self) -> bool {
        let cache = self.history_cache.read().await;
        let policy = &self.config.default_retention_policy;

        // Check if over max entries
        if cache.len() > policy.max_entries {
            return true;
        }

        // Check if any entries are older than max_age_days
        let cutoff = Utc::now() - Duration::days(policy.max_age_days as i64);
        cache.iter().any(|e| e.closed_at < cutoff)
    }

    /// Get entries that would be deleted by the current retention policy
    ///
    /// This is useful for previewing cleanup before actually running it.
    pub async fn preview_cleanup(&self) -> Vec<HistoryEntry> {
        let policy = &self.config.default_retention_policy;
        self.preview_cleanup_with_policy(policy).await
    }

    /// Preview cleanup with a specific policy
    pub async fn preview_cleanup_with_policy(&self, policy: &RetentionPolicy) -> Vec<HistoryEntry> {
        let cache = self.history_cache.read().await;
        let cutoff = Utc::now() - Duration::days(policy.max_age_days as i64);

        let mut to_delete: Vec<HistoryEntry> = cache
            .iter()
            .filter(|e| {
                if e.closed_at >= cutoff {
                    false
                } else if policy.preserve_important {
                    let importance = self.calculate_importance(e);
                    importance < policy.importance_threshold
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        // Also include entries that would be deleted due to max_entries limit
        let remaining_after_age = cache.len() - to_delete.len();
        if remaining_after_age > policy.max_entries {
            let mut remaining: Vec<&HistoryEntry> = cache
                .iter()
                .filter(|e| !to_delete.iter().any(|d| d.id == e.id))
                .collect();

            if policy.preserve_important {
                remaining.sort_by(|a, b| {
                    let a_importance = self.calculate_importance(a);
                    let b_importance = self.calculate_importance(b);
                    a_importance
                        .partial_cmp(&b_importance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }

            let excess = remaining_after_age - policy.max_entries;
            for entry in remaining.into_iter().take(excess) {
                to_delete.push(entry.clone());
            }
        }

        to_delete
    }

    // =========================================================================
    // Export and Backup
    // =========================================================================

    /// Export history entries to a specified format
    ///
    /// This creates a backup of history data that can be imported later.
    pub async fn export(&self, format: ExportFormat) -> Result<String> {
        let filter = HistoryFilter::default();
        self.export_filtered(&filter, format).await
    }

    /// Export filtered history entries
    pub async fn export_filtered(
        &self,
        filter: &HistoryFilter,
        format: ExportFormat,
    ) -> Result<String> {
        let entries = self.get_history(filter).await;
        
        let date_range = if entries.is_empty() {
            None
        } else {
            let oldest = entries.iter().map(|e| e.closed_at).min();
            let newest = entries.iter().map(|e| e.closed_at).max();
            oldest.zip(newest)
        };

        let metadata = ExportMetadata {
            exported_at: Utc::now(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            entry_count: entries.len(),
            date_range,
            format: format!("{:?}", format),
        };

        let exported = ExportedHistory { metadata, entries };

        match format {
            ExportFormat::Json => {
                serde_json::to_string_pretty(&exported).map_err(|e| {
                    WebPageManagerError::System {
                        source: SystemError::Configuration {
                            details: format!("Failed to serialize history: {}", e),
                        },
                    }
                })
            }
            ExportFormat::Csv => self.export_to_csv(&exported),
            ExportFormat::Html => self.export_to_html(&exported),
        }
    }

    /// Export to CSV format
    fn export_to_csv(&self, exported: &ExportedHistory) -> Result<String> {
        let mut csv = String::new();
        
        // Header
        csv.push_str("id,url,title,browser_type,closed_at,has_summary,keywords\n");
        
        // Data rows
        for entry in &exported.entries {
            let keywords = entry.page_info.keywords.join(";");
            let has_summary = entry.page_info.content_summary.is_some();
            
            csv.push_str(&format!(
                "\"{}\",\"{}\",\"{}\",\"{:?}\",\"{}\",{},\"{}\"\n",
                entry.id.0,
                entry.page_info.url.replace('"', "\"\""),
                entry.page_info.title.replace('"', "\"\""),
                entry.browser_type,
                entry.closed_at.to_rfc3339(),
                has_summary,
                keywords.replace('"', "\"\""),
            ));
        }
        
        Ok(csv)
    }

    /// Export to HTML format
    fn export_to_html(&self, exported: &ExportedHistory) -> Result<String> {
        let mut html = String::new();
        
        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str("<meta charset=\"UTF-8\">\n");
        html.push_str("<title>Tab History Export</title>\n");
        html.push_str("<style>\n");
        html.push_str("body { font-family: Arial, sans-serif; margin: 20px; }\n");
        html.push_str("table { border-collapse: collapse; width: 100%; }\n");
        html.push_str("th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }\n");
        html.push_str("th { background-color: #4CAF50; color: white; }\n");
        html.push_str("tr:nth-child(even) { background-color: #f2f2f2; }\n");
        html.push_str("a { color: #1a73e8; }\n");
        html.push_str(".metadata { margin-bottom: 20px; color: #666; }\n");
        html.push_str("</style>\n</head>\n<body>\n");
        
        // Metadata
        html.push_str("<div class=\"metadata\">\n");
        html.push_str(&format!("<p>Exported: {}</p>\n", exported.metadata.exported_at.to_rfc3339()));
        html.push_str(&format!("<p>Total entries: {}</p>\n", exported.metadata.entry_count));
        if let Some((oldest, newest)) = exported.metadata.date_range {
            html.push_str(&format!(
                "<p>Date range: {} to {}</p>\n",
                oldest.format("%Y-%m-%d"),
                newest.format("%Y-%m-%d")
            ));
        }
        html.push_str("</div>\n");
        
        // Table
        html.push_str("<table>\n<tr>\n");
        html.push_str("<th>Title</th><th>URL</th><th>Browser</th><th>Closed At</th>\n");
        html.push_str("</tr>\n");
        
        for entry in &exported.entries {
            html.push_str("<tr>\n");
            html.push_str(&format!(
                "<td>{}</td>\n",
                html_escape(&entry.page_info.title)
            ));
            html.push_str(&format!(
                "<td><a href=\"{}\">{}</a></td>\n",
                html_escape(&entry.page_info.url),
                html_escape(&truncate_url(&entry.page_info.url, 50))
            ));
            html.push_str(&format!("<td>{:?}</td>\n", entry.browser_type));
            html.push_str(&format!(
                "<td>{}</td>\n",
                entry.closed_at.format("%Y-%m-%d %H:%M")
            ));
            html.push_str("</tr>\n");
        }
        
        html.push_str("</table>\n</body>\n</html>");
        
        Ok(html)
    }

    /// Import history from exported JSON data
    pub async fn import(&self, json_data: &str) -> Result<usize> {
        let exported: ExportedHistory = serde_json::from_str(json_data).map_err(|e| {
            WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to parse history data: {}", e),
                },
            }
        })?;

        let mut imported_count = 0;
        let mut cache = self.history_cache.write().await;

        for entry in exported.entries {
            // Check for duplicates by ID
            if !cache.iter().any(|e| e.id == entry.id) {
                cache.push(entry);
                imported_count += 1;
            }
        }

        // Trim cache if needed
        while cache.len() > self.config.max_cache_entries {
            cache.remove(0);
        }

        drop(cache);
        self.update_cache_stats().await;

        info!("Imported {} history entries", imported_count);
        Ok(imported_count)
    }

    /// Save history to a file
    pub async fn save_to_file(&self, path: &Path, format: ExportFormat) -> Result<()> {
        let content = self.export(format).await?;
        
        tokio::fs::write(path, content).await.map_err(|e| {
            WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to write history file: {}", e),
                },
            }
        })?;

        info!("Saved history to {:?}", path);
        Ok(())
    }

    /// Load history from a file
    pub async fn load_from_file(&self, path: &Path) -> Result<usize> {
        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to read history file: {}", e),
                },
            }
        })?;

        self.import(&content).await
    }
}

/// Helper function to escape HTML special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Helper function to truncate URLs for display
fn truncate_url(url: &str, max_len: usize) -> String {
    if url.len() <= max_len {
        url.to_string()
    } else {
        format!("{}...", &url[..max_len - 3])
    }
}

impl Default for TabHistoryManager {
    fn default() -> Self {
        Self::new()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tab(url: &str, title: &str, browser_type: BrowserType) -> TabInfo {
        TabInfo {
            id: TabId::new(),
            url: url.to_string(),
            title: title.to_string(),
            favicon_url: None,
            browser_type,
            is_private: false,
            created_at: Utc::now() - Duration::minutes(10), // Created 10 minutes ago
            last_accessed: Utc::now(),
        }
    }

    fn create_private_tab(url: &str, title: &str) -> TabInfo {
        TabInfo {
            id: TabId::new(),
            url: url.to_string(),
            title: title.to_string(),
            favicon_url: None,
            browser_type: BrowserType::Chrome,
            is_private: true,
            created_at: Utc::now() - Duration::minutes(10),
            last_accessed: Utc::now(),
        }
    }

    fn create_new_tab(url: &str, title: &str) -> TabInfo {
        TabInfo {
            id: TabId::new(),
            url: url.to_string(),
            title: title.to_string(),
            favicon_url: None,
            browser_type: BrowserType::Chrome,
            is_private: false,
            created_at: Utc::now(), // Just created
            last_accessed: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_save_closed_tab() {
        let manager = TabHistoryManager::new();
        let tab = create_test_tab("https://example.com", "Example", BrowserType::Chrome);

        let result = manager.save_closed_tab(tab.clone(), Utc::now()).await;
        assert!(result.is_ok());

        let history = manager.get_recent(10).await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].page_info.url, "https://example.com");
        assert_eq!(history[0].page_info.title, "Example");
        assert_eq!(history[0].browser_type, BrowserType::Chrome);
    }

    #[tokio::test]
    async fn test_filter_by_browser() {
        let manager = TabHistoryManager::new();

        let chrome_tab = create_test_tab("https://chrome.example.com", "Chrome Tab", BrowserType::Chrome);
        let firefox_tab = create_test_tab("https://firefox.example.com", "Firefox Tab", BrowserType::Firefox);

        manager.save_closed_tab(chrome_tab, Utc::now()).await.unwrap();
        manager.save_closed_tab(firefox_tab, Utc::now()).await.unwrap();

        let chrome_history = manager.get_by_browser(BrowserType::Chrome).await;
        assert_eq!(chrome_history.len(), 1);
        assert!(chrome_history[0].page_info.url.contains("chrome"));

        let firefox_history = manager.get_by_browser(BrowserType::Firefox).await;
        assert_eq!(firefox_history.len(), 1);
        assert!(firefox_history[0].page_info.url.contains("firefox"));
    }

    #[tokio::test]
    async fn test_filter_by_date_range() {
        let manager = TabHistoryManager::new();

        let tab1 = create_test_tab("https://old.example.com", "Old Tab", BrowserType::Chrome);
        let tab2 = create_test_tab("https://new.example.com", "New Tab", BrowserType::Chrome);

        let old_time = Utc::now() - Duration::hours(2);
        let new_time = Utc::now();

        manager.save_closed_tab(tab1, old_time).await.unwrap();
        manager.save_closed_tab(tab2, new_time).await.unwrap();

        // Get entries from last hour
        let recent = manager
            .get_in_time_range(Utc::now() - Duration::hours(1), Utc::now())
            .await;
        assert_eq!(recent.len(), 1);
        assert!(recent[0].page_info.url.contains("new"));
    }

    #[tokio::test]
    async fn test_filter_by_url_pattern() {
        let manager = TabHistoryManager::new();

        let tab1 = create_test_tab("https://rust-lang.org", "Rust", BrowserType::Chrome);
        let tab2 = create_test_tab("https://python.org", "Python", BrowserType::Chrome);

        manager.save_closed_tab(tab1, Utc::now()).await.unwrap();
        manager.save_closed_tab(tab2, Utc::now()).await.unwrap();

        let filter = HistoryFilter {
            url_pattern: Some("rust".to_string()),
            ..Default::default()
        };

        let results = manager.get_history(&filter).await;
        assert_eq!(results.len(), 1);
        assert!(results[0].page_info.url.contains("rust"));
    }

    #[tokio::test]
    async fn test_filter_by_title_pattern() {
        let manager = TabHistoryManager::new();

        let tab1 = create_test_tab("https://example1.com", "Programming Tutorial", BrowserType::Chrome);
        let tab2 = create_test_tab("https://example2.com", "News Article", BrowserType::Chrome);

        manager.save_closed_tab(tab1, Utc::now()).await.unwrap();
        manager.save_closed_tab(tab2, Utc::now()).await.unwrap();

        let filter = HistoryFilter {
            title_pattern: Some("tutorial".to_string()),
            ..Default::default()
        };

        let results = manager.get_history(&filter).await;
        assert_eq!(results.len(), 1);
        assert!(results[0].page_info.title.contains("Tutorial"));
    }

    #[tokio::test]
    async fn test_search() {
        let manager = TabHistoryManager::new();

        let tab1 = create_test_tab("https://rust-lang.org", "Rust Programming Language", BrowserType::Chrome);
        let tab2 = create_test_tab("https://python.org", "Python Programming", BrowserType::Chrome);
        let tab3 = create_test_tab("https://news.com", "Daily News", BrowserType::Chrome);

        manager.save_closed_tab(tab1, Utc::now()).await.unwrap();
        manager.save_closed_tab(tab2, Utc::now()).await.unwrap();
        manager.save_closed_tab(tab3, Utc::now()).await.unwrap();

        // Search for "programming"
        let results = manager.search("programming", 10).await;
        assert_eq!(results.len(), 2);

        // Search for "rust"
        let results = manager.search("rust", 10).await;
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_entry() {
        let manager = TabHistoryManager::new();
        let tab = create_test_tab("https://example.com", "Example", BrowserType::Chrome);

        let history_id = manager.save_closed_tab(tab, Utc::now()).await.unwrap();
        assert_eq!(manager.total_count().await, 1);

        let deleted = manager.delete(&history_id).await;
        assert!(deleted);
        assert_eq!(manager.total_count().await, 0);
    }

    #[tokio::test]
    async fn test_delete_older_than() {
        let manager = TabHistoryManager::new();

        let tab1 = create_test_tab("https://old.example.com", "Old", BrowserType::Chrome);
        let tab2 = create_test_tab("https://new.example.com", "New", BrowserType::Chrome);

        let old_time = Utc::now() - Duration::hours(2);
        let new_time = Utc::now();

        manager.save_closed_tab(tab1, old_time).await.unwrap();
        manager.save_closed_tab(tab2, new_time).await.unwrap();

        let deleted = manager.delete_older_than(Utc::now() - Duration::hours(1)).await;
        assert_eq!(deleted, 1);
        assert_eq!(manager.total_count().await, 1);
    }

    #[tokio::test]
    async fn test_skip_private_tabs() {
        let manager = TabHistoryManager::new();
        let private_tab = create_private_tab("https://private.example.com", "Private");

        assert!(!manager.should_save_tab(&private_tab));
    }

    #[tokio::test]
    async fn test_skip_internal_pages() {
        let manager = TabHistoryManager::new();
        let internal_tab = create_test_tab("chrome://settings", "Settings", BrowserType::Chrome);

        assert!(!manager.should_save_tab(&internal_tab));
    }

    #[tokio::test]
    async fn test_skip_new_tabs() {
        let manager = TabHistoryManager::new();
        let new_tab = create_new_tab("https://example.com", "Example");

        // Tab was just created, should not be saved
        assert!(!manager.should_save_tab(&new_tab));
    }

    #[tokio::test]
    async fn test_process_tab_events() {
        let manager = TabHistoryManager::new();
        let tab = create_test_tab("https://example.com", "Example", BrowserType::Chrome);

        let events = vec![TabEvent::Closed {
            tab_id: tab.id.clone(),
            browser_type: BrowserType::Chrome,
            timestamp: Utc::now(),
            last_known_info: Some(tab),
        }];

        let saved_ids = manager.process_tab_events(&events).await;
        assert_eq!(saved_ids.len(), 1);
        assert_eq!(manager.total_count().await, 1);
    }

    #[tokio::test]
    async fn test_get_recently_closed() {
        let manager = TabHistoryManager::new();

        let tab1 = create_test_tab("https://old.example.com", "Old", BrowserType::Chrome);
        let tab2 = create_test_tab("https://recent.example.com", "Recent", BrowserType::Chrome);

        let old_time = Utc::now() - Duration::hours(2);
        let recent_time = Utc::now() - Duration::minutes(5);

        manager.save_closed_tab(tab1, old_time).await.unwrap();
        manager.save_closed_tab(tab2, recent_time).await.unwrap();

        let recently_closed = manager.get_recently_closed(30).await;
        assert_eq!(recently_closed.len(), 1);
        assert!(recently_closed[0].page_info.url.contains("recent"));
    }

    #[tokio::test]
    async fn test_content_summary_enrichment() {
        let manager = TabHistoryManager::new();

        // Register a content summary
        let summary = ContentSummary {
            summary_text: "This is a test summary".to_string(),
            key_points: vec!["Point 1".to_string()],
            content_type: ContentType::Article,
            language: "en".to_string(),
            reading_time_minutes: 5,
            confidence_score: 0.9,
            generated_at: Utc::now(),
        };
        manager
            .register_content_summary("https://example.com", summary)
            .await;

        // Save a tab with that URL
        let tab = create_test_tab("https://example.com", "Example", BrowserType::Chrome);
        manager.save_closed_tab(tab, Utc::now()).await.unwrap();

        // Check that the history entry has the content summary
        let history = manager.get_recent(1).await;
        assert!(history[0].page_info.content_summary.is_some());
        assert_eq!(
            history[0].page_info.content_summary.as_ref().unwrap().summary_text,
            "This is a test summary"
        );
    }

    #[tokio::test]
    async fn test_stats() {
        let manager = TabHistoryManager::new();

        let chrome_tab = create_test_tab("https://chrome.example.com", "Chrome", BrowserType::Chrome);
        let firefox_tab = create_test_tab("https://firefox.example.com", "Firefox", BrowserType::Firefox);

        manager.save_closed_tab(chrome_tab, Utc::now()).await.unwrap();
        manager.save_closed_tab(firefox_tab, Utc::now()).await.unwrap();

        let stats = manager.get_stats().await;
        assert_eq!(stats.cached_entries, 2);
        assert_eq!(stats.session_saves, 2);
        assert_eq!(stats.entries_by_browser.get(&BrowserType::Chrome), Some(&1));
        assert_eq!(stats.entries_by_browser.get(&BrowserType::Firefox), Some(&1));
    }

    #[tokio::test]
    async fn test_get_top_domains() {
        let manager = TabHistoryManager::new();

        // Add multiple tabs from same domain
        for i in 0..3 {
            let tab = create_test_tab(
                &format!("https://example.com/page{}", i),
                &format!("Page {}", i),
                BrowserType::Chrome,
            );
            manager.save_closed_tab(tab, Utc::now()).await.unwrap();
        }

        // Add one tab from different domain
        let tab = create_test_tab("https://other.com", "Other", BrowserType::Chrome);
        manager.save_closed_tab(tab, Utc::now()).await.unwrap();

        let top_domains = manager.get_top_domains(5).await;
        assert_eq!(top_domains[0].0, "example.com");
        assert_eq!(top_domains[0].1, 3);
    }

    #[tokio::test]
    async fn test_retention_policy() {
        let manager = TabHistoryManager::new();

        // Add some entries
        for i in 0..5 {
            let tab = create_test_tab(
                &format!("https://example{}.com", i),
                &format!("Example {}", i),
                BrowserType::Chrome,
            );
            manager.save_closed_tab(tab, Utc::now()).await.unwrap();
        }

        assert_eq!(manager.total_count().await, 5);

        // Apply retention policy with max 3 entries
        let policy = RetentionPolicy {
            max_age_days: 30,
            max_entries: 3,
            preserve_important: false,
            importance_threshold: 0.5,
        };

        let deleted = manager.apply_retention_policy(&policy).await;
        assert_eq!(deleted, 2);
        assert_eq!(manager.total_count().await, 3);
    }

    #[tokio::test]
    async fn test_clear() {
        let manager = TabHistoryManager::new();

        let tab = create_test_tab("https://example.com", "Example", BrowserType::Chrome);
        manager.save_closed_tab(tab, Utc::now()).await.unwrap();

        assert_eq!(manager.total_count().await, 1);

        manager.clear().await;
        assert_eq!(manager.total_count().await, 0);
    }

    #[tokio::test]
    async fn test_pagination() {
        let manager = TabHistoryManager::new();

        // Add 10 entries
        for i in 0..10 {
            let tab = create_test_tab(
                &format!("https://example{}.com", i),
                &format!("Example {}", i),
                BrowserType::Chrome,
            );
            manager.save_closed_tab(tab, Utc::now()).await.unwrap();
        }

        // Get first page
        let filter = HistoryFilter {
            limit: Some(3),
            offset: Some(0),
            ..Default::default()
        };
        let page1 = manager.get_history(&filter).await;
        assert_eq!(page1.len(), 3);

        // Get second page
        let filter = HistoryFilter {
            limit: Some(3),
            offset: Some(3),
            ..Default::default()
        };
        let page2 = manager.get_history(&filter).await;
        assert_eq!(page2.len(), 3);

        // Ensure different entries
        assert_ne!(page1[0].id, page2[0].id);
    }

    // =========================================================================
    // Tests for Tab Restoration (Requirement 7.4)
    // =========================================================================

    #[tokio::test]
    async fn test_get_restore_url() {
        let manager = TabHistoryManager::new();
        let tab = create_test_tab("https://restore.example.com", "Restore Test", BrowserType::Chrome);

        let history_id = manager.save_closed_tab(tab, Utc::now()).await.unwrap();
        
        let url = manager.get_restore_url(&history_id).await;
        assert!(url.is_some());
        assert_eq!(url.unwrap(), "https://restore.example.com");
    }

    #[tokio::test]
    async fn test_get_restore_url_not_found() {
        let manager = TabHistoryManager::new();
        let fake_id = HistoryId::new();
        
        let url = manager.get_restore_url(&fake_id).await;
        assert!(url.is_none());
    }

    #[tokio::test]
    async fn test_get_restore_urls_batch() {
        let manager = TabHistoryManager::new();
        
        let tab1 = create_test_tab("https://url1.example.com", "URL 1", BrowserType::Chrome);
        let tab2 = create_test_tab("https://url2.example.com", "URL 2", BrowserType::Chrome);
        
        let id1 = manager.save_closed_tab(tab1, Utc::now()).await.unwrap();
        let id2 = manager.save_closed_tab(tab2, Utc::now()).await.unwrap();
        let fake_id = HistoryId::new();
        
        let results = manager.get_restore_urls(&[id1.clone(), id2.clone(), fake_id]).await;
        
        assert_eq!(results.len(), 3);
        assert!(results[0].1.is_some());
        assert!(results[1].1.is_some());
        assert!(results[2].1.is_none());
    }

    // =========================================================================
    // Tests for Automatic Cleanup (Requirement 7.5)
    // =========================================================================

    #[tokio::test]
    async fn test_cleanup_with_policy() {
        let manager = TabHistoryManager::new();

        // Add entries with different ages
        for i in 0..5 {
            let tab = create_test_tab(
                &format!("https://example{}.com", i),
                &format!("Example {}", i),
                BrowserType::Chrome,
            );
            manager.save_closed_tab(tab, Utc::now()).await.unwrap();
        }

        let policy = RetentionPolicy {
            max_age_days: 30,
            max_entries: 3,
            preserve_important: false,
            importance_threshold: 0.5,
        };

        let result = manager.cleanup_with_policy(&policy).await;
        
        assert_eq!(result.deleted_by_limit, 2);
        assert_eq!(result.remaining_entries, 3);
    }

    #[tokio::test]
    async fn test_cleanup_preserves_important() {
        let manager = TabHistoryManager::new();

        // Add an entry with content summary (more important)
        let summary = ContentSummary {
            summary_text: "Important content".to_string(),
            key_points: vec!["Key point".to_string()],
            content_type: ContentType::Article,
            language: "en".to_string(),
            reading_time_minutes: 5,
            confidence_score: 0.9,
            generated_at: Utc::now(),
        };
        manager.register_content_summary("https://important.example.com", summary).await;

        let important_tab = create_test_tab("https://important.example.com", "Important", BrowserType::Chrome);
        manager.save_closed_tab(important_tab, Utc::now() - Duration::days(40)).await.unwrap();

        // Add regular entries
        for i in 0..3 {
            let tab = create_test_tab(
                &format!("https://regular{}.com", i),
                &format!("Regular {}", i),
                BrowserType::Chrome,
            );
            manager.save_closed_tab(tab, Utc::now()).await.unwrap();
        }

        let policy = RetentionPolicy {
            max_age_days: 30,
            max_entries: 10,
            preserve_important: true,
            importance_threshold: 0.2, // Low threshold to preserve the important entry
        };

        let result = manager.cleanup_with_policy(&policy).await;
        
        // The important entry should be preserved even though it's old
        assert!(result.preserved_important > 0 || result.deleted_by_age == 0);
    }

    #[tokio::test]
    async fn test_needs_cleanup() {
        let config = TabHistoryManagerConfig {
            max_cache_entries: 5,
            default_retention_policy: RetentionPolicy {
                max_age_days: 30,
                max_entries: 3,
                preserve_important: false,
                importance_threshold: 0.5,
            },
            ..Default::default()
        };
        let manager = TabHistoryManager::with_config(config);

        // Initially no cleanup needed
        assert!(!manager.needs_cleanup().await);

        // Add entries beyond max_entries
        for i in 0..5 {
            let tab = create_test_tab(
                &format!("https://example{}.com", i),
                &format!("Example {}", i),
                BrowserType::Chrome,
            );
            manager.save_closed_tab(tab, Utc::now()).await.unwrap();
        }

        // Now cleanup should be needed
        assert!(manager.needs_cleanup().await);
    }

    #[tokio::test]
    async fn test_preview_cleanup() {
        let manager = TabHistoryManager::new();

        // Add entries
        for i in 0..5 {
            let tab = create_test_tab(
                &format!("https://example{}.com", i),
                &format!("Example {}", i),
                BrowserType::Chrome,
            );
            manager.save_closed_tab(tab, Utc::now()).await.unwrap();
        }

        let policy = RetentionPolicy {
            max_age_days: 30,
            max_entries: 3,
            preserve_important: false,
            importance_threshold: 0.5,
        };

        let to_delete = manager.preview_cleanup_with_policy(&policy).await;
        
        // Should preview 2 entries for deletion
        assert_eq!(to_delete.len(), 2);
        
        // Actual count should still be 5 (preview doesn't delete)
        assert_eq!(manager.total_count().await, 5);
    }

    #[tokio::test]
    async fn test_run_auto_cleanup() {
        let config = TabHistoryManagerConfig {
            default_retention_policy: RetentionPolicy {
                max_age_days: 30,
                max_entries: 3,
                preserve_important: false,
                importance_threshold: 0.5,
            },
            ..Default::default()
        };
        let manager = TabHistoryManager::with_config(config);

        // Add entries
        for i in 0..5 {
            let tab = create_test_tab(
                &format!("https://example{}.com", i),
                &format!("Example {}", i),
                BrowserType::Chrome,
            );
            manager.save_closed_tab(tab, Utc::now()).await.unwrap();
        }

        let result = manager.run_auto_cleanup().await;
        
        assert_eq!(result.remaining_entries, 3);
        
        // Stats should be updated
        let stats = manager.get_stats().await;
        assert!(stats.last_cleanup.is_some());
    }

    // =========================================================================
    // Tests for Export and Backup
    // =========================================================================

    #[tokio::test]
    async fn test_export_json() {
        let manager = TabHistoryManager::new();

        let tab = create_test_tab("https://export.example.com", "Export Test", BrowserType::Chrome);
        manager.save_closed_tab(tab, Utc::now()).await.unwrap();

        let json = manager.export(ExportFormat::Json).await.unwrap();
        
        assert!(json.contains("export.example.com"));
        assert!(json.contains("Export Test"));
        assert!(json.contains("exported_at"));
        assert!(json.contains("entry_count"));
    }

    #[tokio::test]
    async fn test_export_csv() {
        let manager = TabHistoryManager::new();

        let tab = create_test_tab("https://csv.example.com", "CSV Test", BrowserType::Chrome);
        manager.save_closed_tab(tab, Utc::now()).await.unwrap();

        let csv = manager.export(ExportFormat::Csv).await.unwrap();
        
        assert!(csv.contains("id,url,title,browser_type"));
        assert!(csv.contains("csv.example.com"));
        assert!(csv.contains("CSV Test"));
    }

    #[tokio::test]
    async fn test_export_html() {
        let manager = TabHistoryManager::new();

        let tab = create_test_tab("https://html.example.com", "HTML Test", BrowserType::Chrome);
        manager.save_closed_tab(tab, Utc::now()).await.unwrap();

        let html = manager.export(ExportFormat::Html).await.unwrap();
        
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("html.example.com"));
        assert!(html.contains("HTML Test"));
        assert!(html.contains("<table>"));
    }

    #[tokio::test]
    async fn test_import_json() {
        let manager = TabHistoryManager::new();

        // First export some data
        let tab = create_test_tab("https://import.example.com", "Import Test", BrowserType::Chrome);
        manager.save_closed_tab(tab, Utc::now()).await.unwrap();
        
        let json = manager.export(ExportFormat::Json).await.unwrap();
        
        // Clear and reimport
        manager.clear().await;
        assert_eq!(manager.total_count().await, 0);
        
        let imported = manager.import(&json).await.unwrap();
        
        assert_eq!(imported, 1);
        assert_eq!(manager.total_count().await, 1);
        
        let history = manager.get_recent(1).await;
        assert_eq!(history[0].page_info.url, "https://import.example.com");
    }

    #[tokio::test]
    async fn test_import_skips_duplicates() {
        let manager = TabHistoryManager::new();

        let tab = create_test_tab("https://duplicate.example.com", "Duplicate Test", BrowserType::Chrome);
        manager.save_closed_tab(tab, Utc::now()).await.unwrap();
        
        let json = manager.export(ExportFormat::Json).await.unwrap();
        
        // Try to import the same data again
        let imported = manager.import(&json).await.unwrap();
        
        // Should skip duplicates
        assert_eq!(imported, 0);
        assert_eq!(manager.total_count().await, 1);
    }

    #[tokio::test]
    async fn test_export_filtered() {
        let manager = TabHistoryManager::new();

        let chrome_tab = create_test_tab("https://chrome.example.com", "Chrome", BrowserType::Chrome);
        let firefox_tab = create_test_tab("https://firefox.example.com", "Firefox", BrowserType::Firefox);
        
        manager.save_closed_tab(chrome_tab, Utc::now()).await.unwrap();
        manager.save_closed_tab(firefox_tab, Utc::now()).await.unwrap();

        let filter = HistoryFilter {
            browser_type: Some(BrowserType::Chrome),
            ..Default::default()
        };

        let json = manager.export_filtered(&filter, ExportFormat::Json).await.unwrap();
        
        assert!(json.contains("chrome.example.com"));
        assert!(!json.contains("firefox.example.com"));
    }

    #[tokio::test]
    async fn test_cleanup_stats_updated() {
        let manager = TabHistoryManager::new();

        for i in 0..5 {
            let tab = create_test_tab(
                &format!("https://example{}.com", i),
                &format!("Example {}", i),
                BrowserType::Chrome,
            );
            manager.save_closed_tab(tab, Utc::now()).await.unwrap();
        }

        let policy = RetentionPolicy {
            max_age_days: 30,
            max_entries: 3,
            preserve_important: false,
            importance_threshold: 0.5,
        };

        manager.cleanup_with_policy(&policy).await;
        
        let stats = manager.get_stats().await;
        assert!(stats.session_cleanups > 0);
        assert!(stats.last_cleanup.is_some());
    }
}
