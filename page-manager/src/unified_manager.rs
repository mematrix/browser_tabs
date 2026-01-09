//! Unified Page Manager Module
//!
//! Provides the main interface for unified management of tabs and bookmarks.
//! This is the core component that implements Requirements 6.1, 6.2, and 6.3.
//!
//! # Features
//! - Unified page information management
//! - Tab-bookmark association detection and display
//! - Content change detection and sync suggestions
//! - Data inheritance when creating bookmarks from tabs

use web_page_manager_core::*;
use crate::matcher::{
    ContentChangeDetection, ContentChangeDetector, MatcherConfig, TabBookmarkMatcher,
};
use crate::sync::{DataSyncManager, SyncAction, SyncQueue, SyncResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Configuration for the Page Unified Manager
#[derive(Debug, Clone)]
pub struct PageUnifiedManagerConfig {
    /// Matcher configuration
    pub matcher_config: MatcherConfig,
    /// Whether to auto-detect changes on data refresh
    pub auto_detect_changes: bool,
    /// Maximum number of pending sync items to keep
    pub max_pending_sync_items: usize,
}

impl Default for PageUnifiedManagerConfig {
    fn default() -> Self {
        Self {
            matcher_config: MatcherConfig::default(),
            auto_detect_changes: true,
            max_pending_sync_items: 100,
        }
    }
}

/// Association status for a tab
#[derive(Debug, Clone)]
pub struct TabAssociationStatus {
    /// The tab ID
    pub tab_id: TabId,
    /// Whether this tab has a matching bookmark
    pub has_bookmark: bool,
    /// The matching bookmark info (if any)
    pub matching_bookmark: Option<MatchInfo>,
    /// Whether there are pending changes to sync
    pub has_pending_changes: bool,
    /// The detected changes (if any)
    pub pending_changes: Option<ContentChangeDetection>,
}

/// Statistics about the unified page manager state
#[derive(Debug, Clone, Default)]
pub struct UnifiedManagerStats {
    /// Total number of unified pages
    pub total_pages: usize,
    /// Number of pages from active tabs
    pub active_tab_pages: usize,
    /// Number of pages from bookmarks only
    pub bookmark_only_pages: usize,
    /// Number of pages with both tab and bookmark
    pub matched_pages: usize,
    /// Number of pending sync items
    pub pending_sync_count: usize,
    /// Number of detected changes
    pub detected_changes_count: usize,
}

/// Page Unified Manager
///
/// The main component for unified management of tabs and bookmarks.
/// Implements Requirements 6.1, 6.2, and 6.3.
pub struct PageUnifiedManager {
    config: PageUnifiedManagerConfig,
    sync_manager: DataSyncManager,
    sync_queue: Arc<RwLock<SyncQueue>>,
    /// Cached unified pages
    unified_pages: Arc<RwLock<Vec<UnifiedPageInfo>>>,
    /// Cached tabs
    tabs: Arc<RwLock<Vec<TabInfo>>>,
    /// Cached bookmarks
    bookmarks: Arc<RwLock<Vec<BookmarkInfo>>>,
    /// Tab association status cache
    association_cache: Arc<RwLock<HashMap<TabId, TabAssociationStatus>>>,
}

impl PageUnifiedManager {
    /// Create a new Page Unified Manager with default configuration
    pub fn new() -> Self {
        Self::with_config(PageUnifiedManagerConfig::default())
    }

    /// Create a new Page Unified Manager with custom configuration
    pub fn with_config(config: PageUnifiedManagerConfig) -> Self {
        let matcher = TabBookmarkMatcher::with_config(config.matcher_config.clone());
        Self {
            config,
            sync_manager: DataSyncManager::with_matcher(matcher),
            sync_queue: Arc::new(RwLock::new(SyncQueue::new())),
            unified_pages: Arc::new(RwLock::new(Vec::new())),
            tabs: Arc::new(RwLock::new(Vec::new())),
            bookmarks: Arc::new(RwLock::new(Vec::new())),
            association_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the current configuration
    pub fn config(&self) -> &PageUnifiedManagerConfig {
        &self.config
    }

    /// Get a reference to the sync manager
    pub fn sync_manager(&self) -> &DataSyncManager {
        &self.sync_manager
    }

    // =========================================================================
    // Data Management Methods
    // =========================================================================

    /// Update the manager with new tab data
    pub async fn update_tabs(&self, tabs: Vec<TabInfo>) {
        let mut tabs_lock = self.tabs.write().await;
        *tabs_lock = tabs;
        drop(tabs_lock);

        // Refresh associations and unified pages
        self.refresh_associations().await;
        self.refresh_unified_pages().await;

        if self.config.auto_detect_changes {
            self.detect_and_queue_changes().await;
        }

        debug!("Updated tabs, refreshed associations");
    }

    /// Update the manager with new bookmark data
    pub async fn update_bookmarks(&self, bookmarks: Vec<BookmarkInfo>) {
        let mut bookmarks_lock = self.bookmarks.write().await;
        *bookmarks_lock = bookmarks;
        drop(bookmarks_lock);

        // Refresh associations and unified pages
        self.refresh_associations().await;
        self.refresh_unified_pages().await;

        if self.config.auto_detect_changes {
            self.detect_and_queue_changes().await;
        }

        debug!("Updated bookmarks, refreshed associations");
    }

    /// Update both tabs and bookmarks at once
    pub async fn update_all(&self, tabs: Vec<TabInfo>, bookmarks: Vec<BookmarkInfo>) {
        {
            let mut tabs_lock = self.tabs.write().await;
            *tabs_lock = tabs;
        }
        {
            let mut bookmarks_lock = self.bookmarks.write().await;
            *bookmarks_lock = bookmarks;
        }

        self.refresh_associations().await;
        self.refresh_unified_pages().await;

        if self.config.auto_detect_changes {
            self.detect_and_queue_changes().await;
        }

        debug!("Updated all data, refreshed associations");
    }

    /// Refresh the association cache
    async fn refresh_associations(&self) {
        let tabs = self.tabs.read().await;
        let bookmarks = self.bookmarks.read().await;
        let mut cache = self.association_cache.write().await;

        cache.clear();

        // Build match map
        let match_map = self.sync_manager.matcher().build_match_map(&tabs, &bookmarks);

        // Detect changes for matched pairs
        let changes = ContentChangeDetector::detect_all_changes(&tabs, &bookmarks, &match_map);
        let changes_by_tab: HashMap<TabId, ContentChangeDetection> =
            changes.into_iter().map(|c| (c.tab_id.clone(), c)).collect();

        // Build association status for each tab
        for tab in tabs.iter() {
            let matches = match_map.get(&tab.id);
            let best_match = matches.and_then(|m| m.first().cloned());
            let has_bookmark = best_match.is_some();
            let pending_changes = changes_by_tab.get(&tab.id).cloned();
            let has_pending_changes = pending_changes
                .as_ref()
                .map(|c| c.has_changes())
                .unwrap_or(false);

            cache.insert(
                tab.id.clone(),
                TabAssociationStatus {
                    tab_id: tab.id.clone(),
                    has_bookmark,
                    matching_bookmark: best_match,
                    has_pending_changes,
                    pending_changes,
                },
            );
        }
    }

    /// Refresh unified pages by merging tabs and bookmarks
    async fn refresh_unified_pages(&self) {
        let tabs = self.tabs.read().await;
        let bookmarks = self.bookmarks.read().await;
        let existing = self.unified_pages.read().await.clone();

        let merged = self.sync_manager.batch_merge(&tabs, &bookmarks, &existing);

        let mut pages = self.unified_pages.write().await;
        *pages = merged;
    }

    /// Detect changes and add them to the sync queue
    async fn detect_and_queue_changes(&self) {
        let tabs = self.tabs.read().await;
        let bookmarks = self.bookmarks.read().await;

        let actions = self.sync_manager.generate_sync_actions(&tabs, &bookmarks);

        if !actions.is_empty() {
            let mut queue = self.sync_queue.write().await;

            // Get the changes for each action
            let match_map = self.sync_manager.matcher().build_match_map(&tabs, &bookmarks);
            let changes = ContentChangeDetector::detect_all_changes(&tabs, &bookmarks, &match_map);
            let changes_by_bookmark: HashMap<BookmarkId, ContentChangeDetection> = changes
                .into_iter()
                .map(|c| (c.bookmark_id.clone(), c))
                .collect();

            for action in actions {
                if let SyncAction::UpdateBookmark { ref bookmark_id, .. } = action {
                    if let Some(change) = changes_by_bookmark.get(bookmark_id) {
                        // Limit queue size
                        if queue.pending_count() < self.config.max_pending_sync_items {
                            queue.add(change.clone(), action);
                        }
                    }
                }
            }

            info!("Queued {} sync actions", queue.pending_count());
        }
    }

    // =========================================================================
    // Query Methods
    // =========================================================================

    /// Get all unified pages
    pub async fn get_unified_pages(&self) -> Vec<UnifiedPageInfo> {
        self.unified_pages.read().await.clone()
    }

    /// Get a unified page by ID
    pub async fn get_unified_page_by_id(&self, id: &uuid::Uuid) -> Option<UnifiedPageInfo> {
        self.unified_pages
            .read()
            .await
            .iter()
            .find(|p| &p.id == id)
            .cloned()
    }

    /// Get a unified page by URL
    pub async fn get_unified_page_by_url(&self, url: &str) -> Option<UnifiedPageInfo> {
        let normalized = self.sync_manager.matcher().normalize_url(url);
        self.unified_pages
            .read()
            .await
            .iter()
            .find(|p| self.sync_manager.matcher().normalize_url(&p.url) == normalized)
            .cloned()
    }

    /// Get the association status for a tab
    ///
    /// Implements Requirement 6.1: Display bookmark association marks
    /// when tab URL matches existing bookmark.
    pub async fn get_tab_association_status(&self, tab_id: &TabId) -> Option<TabAssociationStatus> {
        self.association_cache.read().await.get(tab_id).cloned()
    }

    /// Get all tabs with their association status
    pub async fn get_all_tab_associations(&self) -> Vec<TabAssociationStatus> {
        self.association_cache.read().await.values().cloned().collect()
    }

    /// Get tabs that have matching bookmarks
    pub async fn get_tabs_with_bookmarks(&self) -> Vec<TabAssociationStatus> {
        self.association_cache
            .read()
            .await
            .values()
            .filter(|s| s.has_bookmark)
            .cloned()
            .collect()
    }

    /// Get tabs that have pending changes
    ///
    /// Implements Requirement 6.2: Detect tab content changes
    pub async fn get_tabs_with_pending_changes(&self) -> Vec<TabAssociationStatus> {
        self.association_cache
            .read()
            .await
            .values()
            .filter(|s| s.has_pending_changes)
            .cloned()
            .collect()
    }

    /// Check if a tab has a matching bookmark
    pub async fn tab_has_bookmark(&self, tab_id: &TabId) -> bool {
        self.association_cache
            .read()
            .await
            .get(tab_id)
            .map(|s| s.has_bookmark)
            .unwrap_or(false)
    }

    /// Find bookmarks matching a tab
    pub async fn find_bookmarks_for_tab(&self, tab_id: &TabId) -> Vec<MatchInfo> {
        let tabs = self.tabs.read().await;
        let bookmarks = self.bookmarks.read().await;

        if let Some(tab) = tabs.iter().find(|t| &t.id == tab_id) {
            self.sync_manager.matcher().find_matches_for_tab(tab, &bookmarks)
        } else {
            vec![]
        }
    }

    /// Find tabs matching a bookmark
    pub async fn find_tabs_for_bookmark(&self, bookmark_id: &BookmarkId) -> Vec<MatchInfo> {
        let tabs = self.tabs.read().await;
        let bookmarks = self.bookmarks.read().await;

        if let Some(bookmark) = bookmarks.iter().find(|b| &b.id == bookmark_id) {
            self.sync_manager
                .matcher()
                .find_matches_for_bookmark(bookmark, &tabs)
        } else {
            vec![]
        }
    }

    // =========================================================================
    // Sync Methods
    // =========================================================================

    /// Get the number of pending sync items
    pub async fn pending_sync_count(&self) -> usize {
        self.sync_queue.read().await.pending_count()
    }

    /// Get all pending sync items
    pub async fn get_pending_sync_items(&self) -> Vec<crate::sync::PendingSyncItem> {
        self.sync_queue.read().await.pending_items().to_vec()
    }

    /// Approve a specific sync item
    pub async fn approve_sync_item(&self, index: usize) -> bool {
        self.sync_queue.write().await.approve(index)
    }

    /// Approve all pending sync items
    pub async fn approve_all_sync_items(&self) {
        self.sync_queue.write().await.approve_all()
    }

    /// Execute approved sync actions
    ///
    /// Returns the sync result with performed actions.
    pub async fn execute_approved_syncs(&self) -> SyncResult {
        let approved = self.sync_queue.write().await.take_approved();

        if approved.is_empty() {
            return SyncResult::success(vec![]);
        }

        let mut performed_actions = Vec::new();
        let mut errors = Vec::new();

        for item in approved {
            match &item.suggested_action {
                SyncAction::UpdateBookmark {
                    bookmark_id,
                    new_title,
                    new_favicon,
                } => {
                    // Find and update the bookmark
                    let mut bookmarks = self.bookmarks.write().await;
                    if let Some(bookmark) = bookmarks.iter_mut().find(|b| &b.id == bookmark_id) {
                        if let Some(title) = new_title {
                            bookmark.title = title.clone();
                        }
                        if let Some(favicon) = new_favicon {
                            bookmark.favicon_url = Some(favicon.clone());
                        }
                        bookmark.last_accessed = Some(chrono::Utc::now());
                        performed_actions.push(item.suggested_action.clone());
                        info!("Updated bookmark {:?}", bookmark_id);
                    } else {
                        errors.push(format!("Bookmark {:?} not found", bookmark_id));
                    }
                }
                SyncAction::CreateBookmark { tab_id, folder_path } => {
                    // Find the tab and create a bookmark
                    let tabs = self.tabs.read().await;
                    if let Some(tab) = tabs.iter().find(|t| &t.id == tab_id) {
                        // Get or create unified page for this tab
                        let unified_page = self.get_unified_page_by_url(&tab.url).await;
                        let page = unified_page.unwrap_or_else(|| {
                            self.sync_manager.merge_to_unified_page(Some(tab), None, None)
                        });

                        let (bookmark, _) = self.sync_manager.create_bookmark_from_tab_with_inheritance(
                            tab,
                            &page,
                            folder_path.clone(),
                        );

                        let mut bookmarks = self.bookmarks.write().await;
                        bookmarks.push(bookmark);
                        performed_actions.push(item.suggested_action.clone());
                        info!("Created bookmark from tab {:?}", tab_id);
                    } else {
                        errors.push(format!("Tab {:?} not found", tab_id));
                    }
                }
                SyncAction::UpdateUnifiedPage { page_id, updates } => {
                    let mut pages = self.unified_pages.write().await;
                    if let Some(page) = pages.iter_mut().find(|p| &p.id == page_id) {
                        if let Some(title) = &updates.title {
                            page.title = title.clone();
                        }
                        if let Some(favicon) = &updates.favicon_url {
                            page.favicon_url = favicon.clone();
                        }
                        if let Some(summary) = &updates.content_summary {
                            page.content_summary = summary.clone();
                        }
                        if let Some(keywords) = &updates.keywords {
                            page.keywords = keywords.clone();
                        }
                        if let Some(category) = &updates.category {
                            page.category = category.clone();
                        }
                        page.last_accessed = chrono::Utc::now();
                        performed_actions.push(item.suggested_action.clone());
                        info!("Updated unified page {:?}", page_id);
                    } else {
                        errors.push(format!("Unified page {:?} not found", page_id));
                    }
                }
            }
        }

        // Refresh after sync
        self.refresh_associations().await;
        self.refresh_unified_pages().await;

        SyncResult::partial(performed_actions, errors)
    }

    /// Clear all pending sync items
    pub async fn clear_pending_syncs(&self) {
        self.sync_queue.write().await.clear();
    }

    // =========================================================================
    // Bookmark Creation Methods
    // =========================================================================

    /// Create a bookmark from a tab with data inheritance
    ///
    /// Implements Requirement 6.3: Auto-inherit analyzed content summary
    /// and tags when adding tab as bookmark.
    pub async fn create_bookmark_from_tab(
        &self,
        tab_id: &TabId,
        folder_path: Vec<String>,
    ) -> Result<(BookmarkInfo, UnifiedPageInfo)> {
        let tabs = self.tabs.read().await;
        let tab = tabs
            .iter()
            .find(|t| &t.id == tab_id)
            .ok_or_else(|| WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Tab {:?} not found", tab_id),
                },
            })?;

        // Get or create unified page for this tab
        let unified_page = self.get_unified_page_by_url(&tab.url).await;
        let page = unified_page.unwrap_or_else(|| {
            self.sync_manager.merge_to_unified_page(Some(tab), None, None)
        });

        let (bookmark, bookmark_page) = self
            .sync_manager
            .create_bookmark_from_tab_with_inheritance(tab, &page, folder_path);

        // Add the bookmark to our cache
        {
            let mut bookmarks = self.bookmarks.write().await;
            bookmarks.push(bookmark.clone());
        }

        // Add the bookmark page to unified pages
        {
            let mut pages = self.unified_pages.write().await;
            pages.push(bookmark_page.clone());
        }

        // Refresh associations
        self.refresh_associations().await;

        info!("Created bookmark from tab {:?}", tab_id);

        Ok((bookmark, bookmark_page))
    }

    // =========================================================================
    // Statistics Methods
    // =========================================================================

    /// Get statistics about the unified manager state
    pub async fn get_stats(&self) -> UnifiedManagerStats {
        let pages = self.unified_pages.read().await;
        let queue = self.sync_queue.read().await;
        let associations = self.association_cache.read().await;

        let mut stats = UnifiedManagerStats {
            total_pages: pages.len(),
            pending_sync_count: queue.pending_count(),
            detected_changes_count: associations.values().filter(|a| a.has_pending_changes).count(),
            ..Default::default()
        };

        for page in pages.iter() {
            match (&page.tab_info, &page.bookmark_info) {
                (Some(_), Some(_)) => stats.matched_pages += 1,
                (Some(_), None) => stats.active_tab_pages += 1,
                (None, Some(_)) => stats.bookmark_only_pages += 1,
                (None, None) => {}
            }
        }

        stats
    }
}

impl Default for PageUnifiedManager {
    fn default() -> Self {
        Self::new()
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

    #[tokio::test]
    async fn test_update_tabs() {
        let manager = PageUnifiedManager::new();
        let tabs = vec![
            create_test_tab("https://example.com", "Example"),
            create_test_tab("https://rust-lang.org", "Rust"),
        ];

        manager.update_tabs(tabs).await;

        let pages = manager.get_unified_pages().await;
        assert_eq!(pages.len(), 2);
    }

    #[tokio::test]
    async fn test_tab_bookmark_association() {
        let manager = PageUnifiedManager::new();

        let tab = create_test_tab("https://example.com", "Example Tab");
        let bookmark = create_test_bookmark("https://example.com", "Example Bookmark");

        manager.update_all(vec![tab.clone()], vec![bookmark]).await;

        let status = manager.get_tab_association_status(&tab.id).await;
        assert!(status.is_some());
        assert!(status.unwrap().has_bookmark);
    }

    #[tokio::test]
    async fn test_change_detection() {
        let manager = PageUnifiedManager::new();

        let tab = create_test_tab("https://example.com", "New Title");
        let bookmark = create_test_bookmark("https://example.com", "Old Title");

        manager.update_all(vec![tab.clone()], vec![bookmark]).await;

        let status = manager.get_tab_association_status(&tab.id).await.unwrap();
        assert!(status.has_pending_changes);
        assert!(status.pending_changes.is_some());
        assert!(status.pending_changes.unwrap().title_changed);
    }

    #[tokio::test]
    async fn test_create_bookmark_from_tab() {
        let manager = PageUnifiedManager::new();

        let tab = create_test_tab("https://example.com", "Example");
        manager.update_tabs(vec![tab.clone()]).await;

        let result = manager
            .create_bookmark_from_tab(&tab.id, vec!["Favorites".to_string()])
            .await;

        assert!(result.is_ok());
        let (bookmark, page) = result.unwrap();
        assert_eq!(bookmark.url, "https://example.com");
        assert_eq!(bookmark.title, "Example");
        assert_eq!(bookmark.folder_path, vec!["Favorites".to_string()]);
        assert!(page.bookmark_info.is_some());
    }

    #[tokio::test]
    async fn test_stats() {
        let manager = PageUnifiedManager::new();

        let tabs = vec![
            create_test_tab("https://example.com", "Example"),
            create_test_tab("https://rust-lang.org", "Rust"),
        ];

        let bookmarks = vec![
            create_test_bookmark("https://example.com", "Example Bookmark"),
            create_test_bookmark("https://python.org", "Python"),
        ];

        manager.update_all(tabs, bookmarks).await;

        let stats = manager.get_stats().await;
        assert_eq!(stats.total_pages, 3);
        assert_eq!(stats.matched_pages, 1); // example.com has both
        assert_eq!(stats.active_tab_pages, 1); // rust-lang.org
        assert_eq!(stats.bookmark_only_pages, 1); // python.org
    }
}
