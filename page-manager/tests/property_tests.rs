//! Property-based tests for the Page Manager module
//!
//! These tests verify the correctness properties defined in the design document.

use page_manager::*;
use proptest::prelude::*;

// ============================================================================
// Test Data Generators
// ============================================================================

/// Generate a valid URL string
fn arb_url() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "https://example.com",
        "https://example.com/page",
        "https://example.com/page?query=1",
        "https://rust-lang.org",
        "https://rust-lang.org/learn",
        "https://python.org",
        "https://github.com",
        "https://github.com/rust-lang/rust",
        "https://docs.rs",
        "https://crates.io",
    ])
    .prop_map(|s| s.to_string())
}

/// Generate a valid title string
fn arb_title() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 ]{1,50}".prop_map(|s| s.trim().to_string())
        .prop_filter("title must not be empty", |s| !s.is_empty())
}

/// Generate a TabInfo
fn arb_tab_info() -> impl Strategy<Value = TabInfo> {
    (arb_url(), arb_title(), any::<bool>()).prop_map(|(url, title, is_private)| TabInfo {
        id: TabId::new(),
        url,
        title,
        favicon_url: None,
        browser_type: BrowserType::Chrome,
        is_private,
        created_at: chrono::Utc::now(),
        last_accessed: chrono::Utc::now(),
    })
}

/// Generate a BookmarkInfo
fn arb_bookmark_info() -> impl Strategy<Value = BookmarkInfo> {
    (arb_url(), arb_title()).prop_map(|(url, title)| BookmarkInfo {
        id: BookmarkId::new(),
        url,
        title,
        favicon_url: None,
        browser_type: BrowserType::Chrome,
        folder_path: vec![],
        created_at: chrono::Utc::now(),
        last_accessed: None,
    })
}

/// Generate a list of tabs
fn arb_tabs(max_size: usize) -> impl Strategy<Value = Vec<TabInfo>> {
    prop::collection::vec(arb_tab_info(), 0..max_size)
}

/// Generate a list of bookmarks
fn arb_bookmarks(max_size: usize) -> impl Strategy<Value = Vec<BookmarkInfo>> {
    prop::collection::vec(arb_bookmark_info(), 0..max_size)
}

// ============================================================================
// Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: web-page-manager, Property 15: Tab-Bookmark Association Consistency
    /// 
    /// For any tab URL that matches an existing bookmark, the system should
    /// correctly identify the association and display it.
    ///
    /// Validates: Requirements 6.1, 6.2
    #[test]
    fn test_tab_bookmark_association_consistency(
        tab in arb_tab_info(),
    ) {
        // Create a bookmark with the same URL as the tab
        let bookmark = BookmarkInfo {
            id: BookmarkId::new(),
            url: tab.url.clone(),
            title: "Bookmark Title".to_string(),
            favicon_url: None,
            browser_type: BrowserType::Chrome,
            folder_path: vec![],
            created_at: chrono::Utc::now(),
            last_accessed: None,
        };

        let matcher = TabBookmarkMatcher::new();
        let matches = matcher.find_matches_for_tab(&tab, &[bookmark.clone()]);

        // Property: If a tab and bookmark have the same URL, they should match
        prop_assert!(!matches.is_empty(), "Tab and bookmark with same URL should match");
        
        // Property: The match should be an exact URL match with confidence 1.0
        let best_match = &matches[0];
        prop_assert!(matches!(best_match.match_type, MatchType::ExactUrl));
        prop_assert_eq!(best_match.confidence, 1.0);
    }

    /// Feature: web-page-manager, Property 16: Data Inheritance Integrity
    ///
    /// For any bookmark created from a tab, the new bookmark should automatically
    /// inherit the analyzed content summary and tags from the associated UnifiedPageInfo.
    ///
    /// Validates: Requirements 6.3
    #[test]
    fn test_data_inheritance_integrity(
        tab in arb_tab_info(),
        keywords in prop::collection::vec("[a-z]{3,10}", 0..5),
        category in prop::option::of("[a-z]{5,15}"),
    ) {
        // Skip private tabs as they shouldn't be bookmarked
        prop_assume!(!tab.is_private);

        // Create a unified page with content summary and keywords
        let unified_page = UnifiedPageInfo {
            id: uuid::Uuid::new_v4(),
            url: tab.url.clone(),
            title: tab.title.clone(),
            favicon_url: tab.favicon_url.clone(),
            content_summary: Some(ContentSummary {
                summary_text: "Test summary".to_string(),
                key_points: vec!["Point 1".to_string()],
                content_type: ContentType::Article,
                language: "en".to_string(),
                reading_time_minutes: 5,
                confidence_score: 0.9,
                generated_at: chrono::Utc::now(),
            }),
            keywords: keywords.clone(),
            category: category.clone(),
            source_type: PageSourceType::ActiveTab {
                browser: tab.browser_type,
                tab_id: tab.id.clone(),
            },
            browser_info: None,
            tab_info: Some(tab.clone()),
            bookmark_info: None,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 1,
        };

        // Create bookmark from tab
        let (bookmark, bookmark_page) = create_bookmark_from_tab(
            &tab,
            &unified_page,
            vec!["Test Folder".to_string()],
        );

        // Property: Bookmark should have the same URL as the tab
        prop_assert_eq!(&bookmark.url, &tab.url);

        // Property: Bookmark should have the same title as the tab
        prop_assert_eq!(&bookmark.title, &tab.title);

        // Property: Bookmark page should inherit content summary
        prop_assert!(bookmark_page.content_summary.is_some());
        prop_assert_eq!(
            &bookmark_page.content_summary.as_ref().unwrap().summary_text,
            "Test summary"
        );

        // Property: Bookmark page should inherit keywords
        prop_assert_eq!(&bookmark_page.keywords, &keywords);

        // Property: Bookmark page should inherit category
        prop_assert_eq!(&bookmark_page.category, &category);

        // Property: Bookmark page should have bookmark_info set
        prop_assert!(bookmark_page.bookmark_info.is_some());
    }

    /// Test URL normalization consistency
    ///
    /// For any URL, normalizing it twice should produce the same result.
    #[test]
    fn test_url_normalization_idempotent(url in arb_url()) {
        let matcher = TabBookmarkMatcher::new();
        
        let normalized_once = matcher.normalize_url(&url);
        let normalized_twice = matcher.normalize_url(&normalized_once);
        
        // Property: Normalization should be idempotent
        prop_assert_eq!(normalized_once, normalized_twice);
    }

    /// Test that exact URL matches have higher confidence than domain matches
    #[test]
    fn test_match_confidence_ordering(
        base_url in prop::sample::select(vec![
            "https://example.com",
            "https://rust-lang.org",
            "https://github.com",
        ]),
    ) {
        let tab = TabInfo {
            id: TabId::new(),
            url: format!("{}/page1", base_url),
            title: "Tab".to_string(),
            favicon_url: None,
            browser_type: BrowserType::Chrome,
            is_private: false,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
        };

        let exact_bookmark = BookmarkInfo {
            id: BookmarkId::new(),
            url: format!("{}/page1", base_url),
            title: "Exact".to_string(),
            favicon_url: None,
            browser_type: BrowserType::Chrome,
            folder_path: vec![],
            created_at: chrono::Utc::now(),
            last_accessed: None,
        };

        let domain_bookmark = BookmarkInfo {
            id: BookmarkId::new(),
            url: format!("{}/page2", base_url),
            title: "Domain".to_string(),
            favicon_url: None,
            browser_type: BrowserType::Chrome,
            folder_path: vec![],
            created_at: chrono::Utc::now(),
            last_accessed: None,
        };

        let matcher = TabBookmarkMatcher::new();
        let matches = matcher.find_matches_for_tab(&tab, &[exact_bookmark, domain_bookmark]);

        // Property: Should find both matches
        prop_assert_eq!(matches.len(), 2);

        // Property: Exact match should come first (higher confidence)
        prop_assert!(matches!(matches[0].match_type, MatchType::ExactUrl));
        prop_assert!(matches!(matches[1].match_type, MatchType::SameDomain));

        // Property: Exact match confidence should be higher
        prop_assert!(matches[0].confidence > matches[1].confidence);
    }

    /// Test batch merge preserves all unique URLs
    #[test]
    fn test_batch_merge_preserves_urls(
        tabs in arb_tabs(10),
        bookmarks in arb_bookmarks(10),
    ) {
        let sync_manager = DataSyncManager::new();
        let merged = sync_manager.batch_merge(&tabs, &bookmarks, &[]);

        // Collect all unique URLs from input
        let mut input_urls: std::collections::HashSet<String> = std::collections::HashSet::new();
        for tab in &tabs {
            input_urls.insert(sync_manager.matcher().normalize_url(&tab.url));
        }
        for bookmark in &bookmarks {
            input_urls.insert(sync_manager.matcher().normalize_url(&bookmark.url));
        }

        // Collect URLs from merged output
        let output_urls: std::collections::HashSet<String> = merged
            .iter()
            .map(|p| sync_manager.matcher().normalize_url(&p.url))
            .collect();

        // Property: All unique input URLs should be in the output
        for url in &input_urls {
            prop_assert!(
                output_urls.contains(url),
                "URL {} from input not found in merged output",
                url
            );
        }

        // Property: Output should not have more unique URLs than input
        prop_assert!(output_urls.len() <= input_urls.len());
    }

    /// Test content change detection accuracy
    #[test]
    fn test_content_change_detection(
        old_title in arb_title(),
        new_title in arb_title(),
    ) {
        let tab = TabInfo {
            id: TabId::new(),
            url: "https://example.com".to_string(),
            title: new_title.clone(),
            favicon_url: None,
            browser_type: BrowserType::Chrome,
            is_private: false,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
        };

        let bookmark = BookmarkInfo {
            id: BookmarkId::new(),
            url: "https://example.com".to_string(),
            title: old_title.clone(),
            favicon_url: None,
            browser_type: BrowserType::Chrome,
            folder_path: vec![],
            created_at: chrono::Utc::now(),
            last_accessed: None,
        };

        let detection = ContentChangeDetector::detect_changes(&tab, &bookmark);

        // Property: Title change should be detected if and only if titles differ
        prop_assert_eq!(detection.title_changed, old_title != new_title);

        // Property: Old and new titles should be correctly recorded
        prop_assert_eq!(&detection.old_title, &old_title);
        prop_assert_eq!(&detection.new_title, &new_title);

        // Property: has_changes should reflect actual changes
        prop_assert_eq!(detection.has_changes(), old_title != new_title);
    }
}

// ============================================================================
// Additional Unit Tests
// ============================================================================

#[test]
fn test_matcher_no_match_different_domains() {
    let matcher = TabBookmarkMatcher::new();

    let tab = TabInfo {
        id: TabId::new(),
        url: "https://example.com/page".to_string(),
        title: "Example".to_string(),
        favicon_url: None,
        browser_type: BrowserType::Chrome,
        is_private: false,
        created_at: chrono::Utc::now(),
        last_accessed: chrono::Utc::now(),
    };

    let bookmark = BookmarkInfo {
        id: BookmarkId::new(),
        url: "https://different.com/page".to_string(),
        title: "Different".to_string(),
        favicon_url: None,
        browser_type: BrowserType::Chrome,
        folder_path: vec![],
        created_at: chrono::Utc::now(),
        last_accessed: None,
    };

    let matches = matcher.find_matches_for_tab(&tab, &[bookmark]);
    assert!(matches.is_empty(), "Different domains should not match");
}

#[test]
fn test_sync_queue_operations() {
    let mut queue = SyncQueue::new();

    // Add items
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

    // Approve and take
    queue.approve(0);
    let approved = queue.take_approved();
    assert_eq!(approved.len(), 1);
    assert!(queue.is_empty());
}
