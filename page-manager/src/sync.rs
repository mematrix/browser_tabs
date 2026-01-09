//! Data Synchronization Module
//!
//! Provides functionality for synchronizing data between tabs and bookmarks,
//! including update propagation and data inheritance.
//!
//! # Requirements
//! - 6.2: Detect tab content changes and offer bookmark info update options
//! - 6.3: Auto-inherit analyzed content summary and tags when adding tab as bookmark

use web_page_manager_core::*;
use crate::matcher::{ContentChangeDetection, ContentChangeDetector, TabBookmarkMatcher};
use std::collections::HashMap;

/// Synchronization action to be performed
#[derive(Debug, Clone)]
pub enum SyncAction {
    /// Update bookmark with tab's current information
    UpdateBookmark {
        bookmark_id: BookmarkId,
        new_title: Option<String>,
        new_favicon: Option<String>,
    },
    /// Create a new bookmark from a tab
    CreateBookmark {
        tab_id: TabId,
        folder_path: Vec<String>,
    },
    /// Update unified page info with new data
    UpdateUnifiedPage {
        page_id: uuid::Uuid,
        updates: PageUpdates,
    },
}

/// Updates to apply to a unified page
#[derive(Debug, Clone, Default)]
pub struct PageUpdates {
    pub title: Option<String>,
    pub favicon_url: Option<Option<String>>,
    pub content_summary: Option<Option<ContentSummary>>,
    pub keywords: Option<Vec<String>>,
    pub category: Option<Option<String>>,
}

/// Result of a synchronization operation
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Whether the sync was successful
    pub success: bool,
    /// Actions that were performed
    pub actions_performed: Vec<SyncAction>,
    /// Errors that occurred (if any)
    pub errors: Vec<String>,
    /// Number of items synchronized
    pub items_synced: usize,
}

impl SyncResult {
    /// Create a successful sync result
    pub fn success(actions: Vec<SyncAction>) -> Self {
        let items_synced = actions.len();
        Self {
            success: true,
            actions_performed: actions,
            errors: vec![],
            items_synced,
        }
    }

    /// Create a failed sync result
    pub fn failure(errors: Vec<String>) -> Self {
        Self {
            success: false,
            actions_performed: vec![],
            errors,
            items_synced: 0,
        }
    }

    /// Create a partial success result
    pub fn partial(actions: Vec<SyncAction>, errors: Vec<String>) -> Self {
        let items_synced = actions.len();
        Self {
            success: errors.is_empty(),
            actions_performed: actions,
            errors,
            items_synced,
        }
    }
}

/// Data synchronization manager
///
/// Handles synchronization between tabs, bookmarks, and unified pages.
pub struct DataSyncManager {
    matcher: TabBookmarkMatcher,
}

impl DataSyncManager {
    /// Create a new sync manager
    pub fn new() -> Self {
        Self {
            matcher: TabBookmarkMatcher::new(),
        }
    }

    /// Create a sync manager with a custom matcher
    pub fn with_matcher(matcher: TabBookmarkMatcher) -> Self {
        Self { matcher }
    }

    /// Get the matcher reference
    pub fn matcher(&self) -> &TabBookmarkMatcher {
        &self.matcher
    }

    /// Generate sync actions for detected content changes
    ///
    /// This analyzes tabs and bookmarks to find changes that need
    /// to be synchronized.
    pub fn generate_sync_actions(
        &self,
        tabs: &[TabInfo],
        bookmarks: &[BookmarkInfo],
    ) -> Vec<SyncAction> {
        let mut actions = Vec::new();

        // Build match map
        let match_map = self.matcher.build_match_map(tabs, bookmarks);

        // Detect content changes
        let changes = ContentChangeDetector::detect_all_changes(tabs, bookmarks, &match_map);

        // Generate update actions for each change
        for change in changes {
            if change.has_changes() {
                actions.push(SyncAction::UpdateBookmark {
                    bookmark_id: change.bookmark_id,
                    new_title: if change.title_changed {
                        Some(change.new_title)
                    } else {
                        None
                    },
                    new_favicon: if change.favicon_changed {
                        change.new_favicon
                    } else {
                        None
                    },
                });
            }
        }

        actions
    }

    /// Apply a bookmark update from tab changes
    ///
    /// Returns the updated BookmarkInfo.
    pub fn apply_bookmark_update(
        &self,
        bookmark: &BookmarkInfo,
        new_title: Option<String>,
        new_favicon: Option<String>,
    ) -> BookmarkInfo {
        let mut updated = bookmark.clone();

        if let Some(title) = new_title {
            updated.title = title;
        }

        if let Some(favicon) = new_favicon {
            updated.favicon_url = Some(favicon);
        }

        updated.last_accessed = Some(chrono::Utc::now());

        updated
    }

    /// Create a bookmark from a tab with data inheritance
    ///
    /// Implements Requirement 6.3: Auto-inherit analyzed content summary
    /// and tags when adding tab as bookmark.
    ///
    /// This uses the `create_bookmark_from_tab` function from core types
    /// to ensure proper data inheritance.
    pub fn create_bookmark_from_tab_with_inheritance(
        &self,
        tab: &TabInfo,
        unified_page: &UnifiedPageInfo,
        folder_path: Vec<String>,
    ) -> (BookmarkInfo, UnifiedPageInfo) {
        // Use the core function that handles data inheritance
        create_bookmark_from_tab(tab, unified_page, folder_path)
    }

    /// Merge tab and bookmark data into a unified page
    ///
    /// Creates or updates a UnifiedPageInfo that combines data from
    /// both the tab and bookmark.
    pub fn merge_to_unified_page(
        &self,
        tab: Option<&TabInfo>,
        bookmark: Option<&BookmarkInfo>,
        existing_page: Option<&UnifiedPageInfo>,
    ) -> UnifiedPageInfo {
        let now = chrono::Utc::now();
        let id = existing_page.map(|p| p.id).unwrap_or_else(uuid::Uuid::new_v4);

        // Determine the primary source and URL
        let (url, title, favicon_url, browser_type) = match (tab, bookmark) {
            (Some(t), Some(b)) => {
                // Prefer tab data as it's more current
                (
                    t.url.clone(),
                    t.title.clone(),
                    t.favicon_url.clone().or_else(|| b.favicon_url.clone()),
                    t.browser_type,
                )
            }
            (Some(t), None) => (
                t.url.clone(),
                t.title.clone(),
                t.favicon_url.clone(),
                t.browser_type,
            ),
            (None, Some(b)) => (
                b.url.clone(),
                b.title.clone(),
                b.favicon_url.clone(),
                b.browser_type,
            ),
            (None, None) => {
                // Return existing page or create empty
                return existing_page.cloned().unwrap_or_else(|| UnifiedPageInfo {
                    id,
                    url: String::new(),
                    title: String::new(),
                    favicon_url: None,
                    content_summary: None,
                    keywords: vec![],
                    category: None,
                    source_type: PageSourceType::Bookmark {
                        browser: BrowserType::Chrome,
                        bookmark_id: BookmarkId::new(),
                    },
                    browser_info: None,
                    tab_info: None,
                    bookmark_info: None,
                    created_at: now,
                    last_accessed: now,
                    access_count: 0,
                });
            }
        };

        // Determine source type
        let source_type = match (tab, bookmark) {
            (Some(t), _) => PageSourceType::ActiveTab {
                browser: t.browser_type,
                tab_id: t.id.clone(),
            },
            (None, Some(b)) => PageSourceType::Bookmark {
                browser: b.browser_type,
                bookmark_id: b.id.clone(),
            },
            _ => unreachable!(),
        };

        // Preserve existing analyzed data
        let (content_summary, keywords, category) = if let Some(existing) = existing_page {
            (
                existing.content_summary.clone(),
                existing.keywords.clone(),
                existing.category.clone(),
            )
        } else {
            (None, vec![], None)
        };

        UnifiedPageInfo {
            id,
            url,
            title,
            favicon_url,
            content_summary,
            keywords,
            category,
            source_type,
            browser_info: Some(BrowserInstance {
                browser_type,
                version: String::new(),
                process_id: 0,
                debug_port: None,
                profile_path: None,
            }),
            tab_info: tab.cloned(),
            bookmark_info: bookmark.cloned(),
            created_at: existing_page.map(|p| p.created_at).unwrap_or(now),
            last_accessed: now,
            access_count: existing_page.map(|p| p.access_count + 1).unwrap_or(1),
        }
    }

    /// Batch merge tabs and bookmarks into unified pages
    ///
    /// This is the main entry point for merging tab and bookmark data.
    /// It matches tabs with bookmarks and creates unified page entries.
    pub fn batch_merge(
        &self,
        tabs: &[TabInfo],
        bookmarks: &[BookmarkInfo],
        existing_pages: &[UnifiedPageInfo],
    ) -> Vec<UnifiedPageInfo> {
        let mut result = Vec::new();
        let mut processed_urls = std::collections::HashSet::new();

        // Create lookup maps
        let existing_by_url: HashMap<&str, &UnifiedPageInfo> =
            existing_pages.iter().map(|p| (p.url.as_str(), p)).collect();
        let bookmark_by_url: HashMap<&str, &BookmarkInfo> =
            bookmarks.iter().map(|b| (b.url.as_str(), b)).collect();

        // Process tabs first (they represent current state)
        for tab in tabs {
            let normalized_url = self.matcher.normalize_url(&tab.url);
            if processed_urls.contains(&normalized_url) {
                continue;
            }

            let matching_bookmark = bookmark_by_url.get(tab.url.as_str()).copied();
            let existing_page = existing_by_url.get(tab.url.as_str()).copied();

            let unified = self.merge_to_unified_page(Some(tab), matching_bookmark, existing_page);
            result.push(unified);
            processed_urls.insert(normalized_url);
        }

        // Process bookmarks that don't have matching tabs
        for bookmark in bookmarks {
            let normalized_url = self.matcher.normalize_url(&bookmark.url);
            if processed_urls.contains(&normalized_url) {
                continue;
            }

            let existing_page = existing_by_url.get(bookmark.url.as_str()).copied();
            let unified = self.merge_to_unified_page(None, Some(bookmark), existing_page);
            result.push(unified);
            processed_urls.insert(normalized_url);
        }

        result
    }
}

impl Default for DataSyncManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Pending sync item for user review
#[derive(Debug, Clone)]
pub struct PendingSyncItem {
    /// The detected change
    pub change: ContentChangeDetection,
    /// The suggested action
    pub suggested_action: SyncAction,
    /// Whether the user has approved this sync
    pub approved: bool,
}

/// Sync queue for managing pending synchronization items
pub struct SyncQueue {
    pending: Vec<PendingSyncItem>,
}

impl SyncQueue {
    /// Create a new empty sync queue
    pub fn new() -> Self {
        Self { pending: vec![] }
    }

    /// Add a pending sync item
    pub fn add(&mut self, change: ContentChangeDetection, action: SyncAction) {
        self.pending.push(PendingSyncItem {
            change,
            suggested_action: action,
            approved: false,
        });
    }

    /// Get all pending items
    pub fn pending_items(&self) -> &[PendingSyncItem] {
        &self.pending
    }

    /// Get the number of pending items
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Approve a specific item by index
    pub fn approve(&mut self, index: usize) -> bool {
        if let Some(item) = self.pending.get_mut(index) {
            item.approved = true;
            true
        } else {
            false
        }
    }

    /// Approve all pending items
    pub fn approve_all(&mut self) {
        for item in &mut self.pending {
            item.approved = true;
        }
    }

    /// Get all approved items and clear them from the queue
    pub fn take_approved(&mut self) -> Vec<PendingSyncItem> {
        let (approved, remaining): (Vec<_>, Vec<_>) =
            self.pending.drain(..).partition(|item| item.approved);
        self.pending = remaining;
        approved
    }

    /// Clear all pending items
    pub fn clear(&mut self) {
        self.pending.clear();
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

impl Default for SyncQueue {
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

    #[test]
    fn test_merge_tab_only() {
        let sync_manager = DataSyncManager::new();
        let tab = create_test_tab("https://example.com", "Example");

        let unified = sync_manager.merge_to_unified_page(Some(&tab), None, None);

        assert_eq!(unified.url, "https://example.com");
        assert_eq!(unified.title, "Example");
        assert!(unified.tab_info.is_some());
        assert!(unified.bookmark_info.is_none());
    }

    #[test]
    fn test_merge_bookmark_only() {
        let sync_manager = DataSyncManager::new();
        let bookmark = create_test_bookmark("https://example.com", "Example Bookmark");

        let unified = sync_manager.merge_to_unified_page(None, Some(&bookmark), None);

        assert_eq!(unified.url, "https://example.com");
        assert_eq!(unified.title, "Example Bookmark");
        assert!(unified.tab_info.is_none());
        assert!(unified.bookmark_info.is_some());
    }

    #[test]
    fn test_merge_tab_and_bookmark() {
        let sync_manager = DataSyncManager::new();
        let tab = create_test_tab("https://example.com", "Tab Title");
        let bookmark = create_test_bookmark("https://example.com", "Bookmark Title");

        let unified = sync_manager.merge_to_unified_page(Some(&tab), Some(&bookmark), None);

        // Tab title should take precedence
        assert_eq!(unified.title, "Tab Title");
        assert!(unified.tab_info.is_some());
        assert!(unified.bookmark_info.is_some());
    }

    #[test]
    fn test_batch_merge() {
        let sync_manager = DataSyncManager::new();

        let tabs = vec![
            create_test_tab("https://example.com", "Example"),
            create_test_tab("https://rust-lang.org", "Rust"),
        ];

        let bookmarks = vec![
            create_test_bookmark("https://example.com", "Example Bookmark"),
            create_test_bookmark("https://python.org", "Python"),
        ];

        let unified_pages = sync_manager.batch_merge(&tabs, &bookmarks, &[]);

        // Should have 3 unique pages
        assert_eq!(unified_pages.len(), 3);

        // Check that example.com has both tab and bookmark info
        let example_page = unified_pages.iter().find(|p| p.url == "https://example.com");
        assert!(example_page.is_some());
        let example = example_page.unwrap();
        assert!(example.tab_info.is_some());
        assert!(example.bookmark_info.is_some());
    }

    #[test]
    fn test_sync_queue() {
        let mut queue = SyncQueue::new();

        let change = ContentChangeDetection {
            tab_id: TabId::new(),
            bookmark_id: BookmarkId::new(),
            title_changed: true,
            favicon_changed: false,
            old_title: "Old".to_string(),
            new_title: "New".to_string(),
            old_favicon: None,
            new_favicon: None,
        };

        let action = SyncAction::UpdateBookmark {
            bookmark_id: change.bookmark_id.clone(),
            new_title: Some("New".to_string()),
            new_favicon: None,
        };

        queue.add(change, action);
        assert_eq!(queue.pending_count(), 1);

        queue.approve(0);
        let approved = queue.take_approved();
        assert_eq!(approved.len(), 1);
        assert!(queue.is_empty());
    }
}
