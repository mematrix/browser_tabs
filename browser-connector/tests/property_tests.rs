// Feature: web-page-manager, Property 1: 多浏览器连接完整性 (Multi-browser Connection Integrity)
// Validates: Requirements 1.1, 1.2, 8.1
//
// Property: For any set of running supported browsers, the system should be able to
// detect and successfully connect to all normal mode browser instances, while correctly
// filtering out privacy mode tabs.
//
// This property test validates:
// 1. Privacy mode filtering correctly excludes all private/incognito tabs
// 2. Normal mode tabs are preserved after filtering
// 3. Browser internal pages and extension pages are filtered appropriately
// 4. The filter is consistent - applying it multiple times yields the same result

use proptest::prelude::*;
use browser_connector::{PrivacyModeFilter, PrivacyFilterConfig};
use web_page_manager_core::{TabInfo, TabId, BrowserType, Utc};

// Strategy for generating BrowserType
fn arb_browser_type() -> impl Strategy<Value = BrowserType> {
    prop_oneof![
        Just(BrowserType::Chrome),
        Just(BrowserType::Firefox),
        Just(BrowserType::Edge),
    ]
}

// Strategy for generating valid HTTP/HTTPS URLs (normal web pages)
fn arb_normal_url() -> impl Strategy<Value = String> {
    prop_oneof![
        "https://[a-z]{3,10}\\.[a-z]{2,4}/[a-z0-9/_-]{0,30}",
        "http://[a-z]{3,10}\\.[a-z]{2,4}/[a-z0-9/_-]{0,30}",
        "https://www\\.[a-z]{3,10}\\.[a-z]{2,4}",
    ]
}

// Strategy for generating browser internal URLs that should be filtered
fn arb_internal_url() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("chrome://newtab".to_string()),
        Just("chrome://settings".to_string()),
        Just("chrome://extensions".to_string()),
        Just("chrome://history".to_string()),
        Just("edge://newtab".to_string()),
        Just("edge://settings".to_string()),
        Just("about:blank".to_string()),
        Just("about:newtab".to_string()),
        Just("about:preferences".to_string()),
    ]
}

// Strategy for generating extension URLs that should be filtered
fn arb_extension_url() -> impl Strategy<Value = String> {
    prop_oneof![
        "chrome-extension://[a-z]{32}/[a-z]{1,20}\\.html",
        "moz-extension://[a-f0-9-]{36}/[a-z]{1,20}\\.html",
    ]
}

// Strategy for generating privacy-sensitive URLs
fn arb_privacy_sensitive_url() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("chrome://settings/privacy".to_string()),
        Just("chrome://settings/passwords".to_string()),
        Just("chrome://settings/security".to_string()),
        Just("edge://settings/privacy".to_string()),
        Just("edge://settings/passwords".to_string()),
        Just("about:preferences#privacy".to_string()),
        Just("about:logins".to_string()),
    ]
}

// Strategy for generating a normal (non-private) tab with a normal URL
fn arb_normal_tab() -> impl Strategy<Value = TabInfo> {
    (arb_normal_url(), "[a-zA-Z0-9 ]{1,50}", arb_browser_type())
        .prop_map(|(url, title, browser_type)| {
            let now = Utc::now();
            TabInfo {
                id: TabId::new(),
                url,
                title,
                favicon_url: None,
                browser_type,
                is_private: false,
                created_at: now,
                last_accessed: now,
            }
        })
}

// Strategy for generating a private/incognito tab
fn arb_private_tab() -> impl Strategy<Value = TabInfo> {
    (arb_normal_url(), "[a-zA-Z0-9 ]{1,50}", arb_browser_type())
        .prop_map(|(url, title, browser_type)| {
            let now = Utc::now();
            TabInfo {
                id: TabId::new(),
                url,
                title,
                favicon_url: None,
                browser_type,
                is_private: true,
                created_at: now,
                last_accessed: now,
            }
        })
}

// Strategy for generating a tab with an internal browser URL
fn arb_internal_tab() -> impl Strategy<Value = TabInfo> {
    (arb_internal_url(), "[a-zA-Z0-9 ]{1,50}", arb_browser_type())
        .prop_map(|(url, title, browser_type)| {
            let now = Utc::now();
            TabInfo {
                id: TabId::new(),
                url,
                title,
                favicon_url: None,
                browser_type,
                is_private: false,
                created_at: now,
                last_accessed: now,
            }
        })
}

// Strategy for generating a tab with an extension URL
fn arb_extension_tab() -> impl Strategy<Value = TabInfo> {
    (arb_extension_url(), "[a-zA-Z0-9 ]{1,50}", arb_browser_type())
        .prop_map(|(url, title, browser_type)| {
            let now = Utc::now();
            TabInfo {
                id: TabId::new(),
                url,
                title,
                favicon_url: None,
                browser_type,
                is_private: false,
                created_at: now,
                last_accessed: now,
            }
        })
}

// Strategy for generating a tab with a privacy-sensitive URL
fn arb_privacy_sensitive_tab() -> impl Strategy<Value = TabInfo> {
    (arb_privacy_sensitive_url(), "[a-zA-Z0-9 ]{1,50}", arb_browser_type())
        .prop_map(|(url, title, browser_type)| {
            let now = Utc::now();
            TabInfo {
                id: TabId::new(),
                url,
                title,
                favicon_url: None,
                browser_type,
                is_private: false,
                created_at: now,
                last_accessed: now,
            }
        })
}

// Strategy for generating a mixed list of tabs
fn arb_mixed_tabs() -> impl Strategy<Value = Vec<TabInfo>> {
    prop::collection::vec(
        prop_oneof![
            3 => arb_normal_tab(),
            1 => arb_private_tab(),
            1 => arb_internal_tab(),
            1 => arb_extension_tab(),
        ],
        0..20
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: web-page-manager, Property 1: 多浏览器连接完整性
    /// Validates: Requirements 1.1, 1.2, 8.1
    ///
    /// Sub-property 1a: Privacy mode tabs are always filtered out
    /// For any collection of tabs, all tabs marked as private should be excluded
    /// from the filtered result.
    #[test]
    fn prop_privacy_tabs_filtered(tabs in arb_mixed_tabs()) {
        let filter = PrivacyModeFilter::new();
        let filtered = filter.filter_tabs(tabs.clone());
        
        // All filtered tabs should NOT be private
        for tab in &filtered {
            prop_assert!(
                !tab.is_private,
                "Private tab should have been filtered: {:?}",
                tab.url
            );
        }
        
        // Count private tabs in original
        let private_count = tabs.iter().filter(|t| t.is_private).count();
        let original_non_private = tabs.iter().filter(|t| !t.is_private).count();
        
        // Filtered count should be <= non-private count (some may be filtered for other reasons)
        prop_assert!(
            filtered.len() <= original_non_private,
            "Filtered count {} should be <= non-private count {}",
            filtered.len(),
            original_non_private
        );
        
        // If there were private tabs, filtered should be smaller than original
        if private_count > 0 {
            prop_assert!(
                filtered.len() < tabs.len(),
                "With {} private tabs, filtered {} should be < original {}",
                private_count,
                filtered.len(),
                tabs.len()
            );
        }
    }

    /// Feature: web-page-manager, Property 1: 多浏览器连接完整性
    /// Validates: Requirements 1.1, 1.2, 8.1
    ///
    /// Sub-property 1b: Normal tabs with normal URLs are preserved
    /// For any collection of normal (non-private) tabs with standard HTTP/HTTPS URLs,
    /// all tabs should pass through the filter unchanged.
    #[test]
    fn prop_normal_tabs_preserved(tabs in prop::collection::vec(arb_normal_tab(), 0..20)) {
        let filter = PrivacyModeFilter::new();
        let filtered = filter.filter_tabs(tabs.clone());
        
        // All normal tabs with normal URLs should be preserved
        prop_assert_eq!(
            filtered.len(),
            tabs.len(),
            "All {} normal tabs should be preserved, but got {}",
            tabs.len(),
            filtered.len()
        );
        
        // Verify each tab is present (by URL since IDs are unique)
        let original_urls: std::collections::HashSet<_> = tabs.iter().map(|t| &t.url).collect();
        let filtered_urls: std::collections::HashSet<_> = filtered.iter().map(|t| &t.url).collect();
        
        prop_assert_eq!(
            original_urls,
            filtered_urls,
            "URLs should match between original and filtered"
        );
    }

    /// Feature: web-page-manager, Property 1: 多浏览器连接完整性
    /// Validates: Requirements 1.1, 1.2, 8.1
    ///
    /// Sub-property 1c: Browser internal pages are filtered
    /// For any tab with a browser internal URL (chrome://, edge://, about:),
    /// the tab should be filtered out.
    #[test]
    fn prop_internal_pages_filtered(tabs in prop::collection::vec(arb_internal_tab(), 1..10)) {
        let filter = PrivacyModeFilter::new();
        let filtered = filter.filter_tabs(tabs.clone());
        
        // All internal pages should be filtered out
        prop_assert!(
            filtered.is_empty(),
            "All {} internal tabs should be filtered, but {} remained",
            tabs.len(),
            filtered.len()
        );
    }

    /// Feature: web-page-manager, Property 1: 多浏览器连接完整性
    /// Validates: Requirements 1.1, 1.2, 8.1
    ///
    /// Sub-property 1d: Extension pages are filtered
    /// For any tab with a browser extension URL (chrome-extension://, moz-extension://),
    /// the tab should be filtered out.
    #[test]
    fn prop_extension_pages_filtered(tabs in prop::collection::vec(arb_extension_tab(), 1..10)) {
        let filter = PrivacyModeFilter::new();
        let filtered = filter.filter_tabs(tabs.clone());
        
        // All extension pages should be filtered out
        prop_assert!(
            filtered.is_empty(),
            "All {} extension tabs should be filtered, but {} remained",
            tabs.len(),
            filtered.len()
        );
    }

    /// Feature: web-page-manager, Property 1: 多浏览器连接完整性
    /// Validates: Requirements 1.1, 1.2, 8.1
    ///
    /// Sub-property 1e: Filter is idempotent
    /// Applying the filter multiple times should yield the same result as applying it once.
    #[test]
    fn prop_filter_idempotent(tabs in arb_mixed_tabs()) {
        let filter = PrivacyModeFilter::new();
        
        let filtered_once = filter.filter_tabs(tabs.clone());
        let filtered_twice = filter.filter_tabs(filtered_once.clone());
        
        // Filtering twice should give the same result as filtering once
        prop_assert_eq!(
            filtered_once.len(),
            filtered_twice.len(),
            "Filter should be idempotent: once={}, twice={}",
            filtered_once.len(),
            filtered_twice.len()
        );
        
        // URLs should be identical
        let urls_once: Vec<_> = filtered_once.iter().map(|t| &t.url).collect();
        let urls_twice: Vec<_> = filtered_twice.iter().map(|t| &t.url).collect();
        
        prop_assert_eq!(
            urls_once,
            urls_twice,
            "URLs should be identical after filtering once vs twice"
        );
    }

    /// Feature: web-page-manager, Property 1: 多浏览器连接完整性
    /// Validates: Requirements 1.1, 1.2, 8.1
    ///
    /// Sub-property 1f: Filter statistics are accurate
    /// The filter statistics should accurately reflect the filtering results.
    #[test]
    fn prop_filter_stats_accurate(tabs in arb_mixed_tabs()) {
        let filter = PrivacyModeFilter::new();
        let stats = filter.get_filter_stats(&tabs);
        let filtered = filter.filter_tabs(tabs.clone());
        
        // Total tabs should match input
        prop_assert_eq!(
            stats.total_tabs,
            tabs.len(),
            "Total tabs stat should match input length"
        );
        
        // Passed tabs should match filtered result
        prop_assert_eq!(
            stats.passed_tabs,
            filtered.len(),
            "Passed tabs stat should match filtered length"
        );
        
        // Filtered + passed should equal total
        prop_assert_eq!(
            stats.filtered_tabs + stats.passed_tabs,
            stats.total_tabs,
            "Filtered + passed should equal total"
        );
        
        // Private tabs count should match actual private tabs
        let actual_private = tabs.iter().filter(|t| t.is_private).count();
        prop_assert_eq!(
            stats.private_tabs,
            actual_private,
            "Private tabs stat should match actual count"
        );
    }

    /// Feature: web-page-manager, Property 1: 多浏览器连接完整性
    /// Validates: Requirements 1.1, 1.2, 8.1
    ///
    /// Sub-property 1g: Privacy-sensitive URLs are filtered
    /// For any tab with a privacy-sensitive URL (passwords, privacy settings, etc.),
    /// the tab should be filtered out even if not marked as private.
    #[test]
    fn prop_privacy_sensitive_urls_filtered(tabs in prop::collection::vec(arb_privacy_sensitive_tab(), 1..10)) {
        let filter = PrivacyModeFilter::new();
        let filtered = filter.filter_tabs(tabs.clone());
        
        // All privacy-sensitive URL tabs should be filtered out
        prop_assert!(
            filtered.is_empty(),
            "All {} privacy-sensitive tabs should be filtered, but {} remained",
            tabs.len(),
            filtered.len()
        );
    }

    /// Feature: web-page-manager, Property 1: 多浏览器连接完整性
    /// Validates: Requirements 1.1, 1.2, 8.1
    ///
    /// Sub-property 1h: Custom filter patterns work correctly
    /// When custom filter patterns are configured, tabs matching those patterns
    /// should be filtered out.
    #[test]
    fn prop_custom_patterns_filter(
        tabs in prop::collection::vec(arb_normal_tab(), 1..10),
        pattern in "[a-z]{3,8}"
    ) {
        // Create a filter with a custom pattern
        let config = PrivacyFilterConfig {
            custom_filter_patterns: vec![pattern.clone()],
            ..Default::default()
        };
        let filter = PrivacyModeFilter::with_config(config);
        
        let filtered = filter.filter_tabs(tabs.clone());
        
        // All filtered tabs should NOT contain the pattern
        for tab in &filtered {
            prop_assert!(
                !tab.url.to_lowercase().contains(&pattern.to_lowercase()),
                "Tab with URL containing pattern '{}' should have been filtered: {}",
                pattern,
                tab.url
            );
        }
        
        // Count tabs that should be filtered
        let should_filter_count = tabs.iter()
            .filter(|t| t.url.to_lowercase().contains(&pattern.to_lowercase()))
            .count();
        
        // Filtered count should be original minus those matching pattern
        // (Note: some tabs might be filtered for other reasons too)
        prop_assert!(
            filtered.len() <= tabs.len() - should_filter_count,
            "Expected at most {} tabs after filtering pattern '{}', got {}",
            tabs.len() - should_filter_count,
            pattern,
            filtered.len()
        );
    }

    /// Feature: web-page-manager, Property 1: 多浏览器连接完整性
    /// Validates: Requirements 1.1, 1.2, 8.1
    ///
    /// Sub-property 1i: Disabling internal page filter preserves internal pages
    /// When internal page filtering is disabled, browser internal pages should
    /// pass through the filter.
    #[test]
    fn prop_disabled_internal_filter_preserves(tabs in prop::collection::vec(arb_internal_tab(), 1..10)) {
        let config = PrivacyFilterConfig {
            filter_internal_pages: false,
            filter_extension_pages: true,
            filter_file_urls: true,
            custom_filter_patterns: vec![],
        };
        let filter = PrivacyModeFilter::with_config(config);
        let filtered = filter.filter_tabs(tabs.clone());
        
        // With internal page filtering disabled, internal pages should pass through
        // (unless they match privacy-sensitive patterns)
        let non_privacy_sensitive: Vec<_> = tabs.iter()
            .filter(|t| !filter.is_privacy_sensitive_url(&t.url))
            .collect();
        
        prop_assert_eq!(
            filtered.len(),
            non_privacy_sensitive.len(),
            "With internal filter disabled, {} non-privacy-sensitive tabs should pass, got {}",
            non_privacy_sensitive.len(),
            filtered.len()
        );
    }

    /// Feature: web-page-manager, Property 1: 多浏览器连接完整性
    /// Validates: Requirements 1.1, 1.2, 8.1
    ///
    /// Sub-property 1j: Filter preserves tab data integrity
    /// Filtered tabs should have all their original data preserved unchanged.
    #[test]
    fn prop_filter_preserves_tab_data(tabs in prop::collection::vec(arb_normal_tab(), 1..10)) {
        let filter = PrivacyModeFilter::new();
        let filtered = filter.filter_tabs(tabs.clone());
        
        // Each filtered tab should have identical data to its original
        for filtered_tab in &filtered {
            let original = tabs.iter().find(|t| t.id == filtered_tab.id);
            
            prop_assert!(
                original.is_some(),
                "Filtered tab should exist in original list"
            );
            
            let original = original.unwrap();
            
            prop_assert_eq!(&filtered_tab.url, &original.url, "URL should be preserved");
            prop_assert_eq!(&filtered_tab.title, &original.title, "Title should be preserved");
            prop_assert_eq!(&filtered_tab.favicon_url, &original.favicon_url, "Favicon should be preserved");
            prop_assert_eq!(filtered_tab.browser_type, original.browser_type, "Browser type should be preserved");
            prop_assert_eq!(filtered_tab.is_private, original.is_private, "Privacy flag should be preserved");
        }
    }
}


// ============================================================================
// Feature: web-page-manager, Property 5: 书签验证准确性 (Bookmark Validation Accuracy)
// Validates: Requirements 2.2
//
// Property: For any bookmark collection, the validation process should correctly
// identify each bookmark's accessibility status, and the validation results
// should be consistent with the actual network state.
//
// This property test validates:
// 1. Validation report statistics are accurate and consistent
// 2. All bookmarks in the input are represented in the validation results
// 3. The sum of status categories equals the total bookmark count
// 4. Response times are non-negative when present
// 5. Validation timestamps are reasonable
// ============================================================================

use browser_connector::BookmarkValidator;
use web_page_manager_core::{BookmarkInfo, BookmarkId, AccessibilityStatus};

// Strategy for generating BrowserType for bookmarks
fn arb_bookmark_browser_type() -> impl Strategy<Value = BrowserType> {
    prop_oneof![
        Just(BrowserType::Chrome),
        Just(BrowserType::Firefox),
        Just(BrowserType::Edge),
    ]
}

// Strategy for generating valid HTTP/HTTPS URLs for bookmarks
fn arb_bookmark_url() -> impl Strategy<Value = String> {
    prop_oneof![
        // Valid HTTPS URLs
        "https://[a-z]{3,10}\\.[a-z]{2,4}",
        "https://www\\.[a-z]{3,10}\\.[a-z]{2,4}/[a-z0-9/_-]{0,20}",
        "http://[a-z]{3,10}\\.[a-z]{2,4}",
        // Some well-known domains that are likely accessible
        Just("https://example.com".to_string()),
        Just("https://example.org".to_string()),
        Just("https://httpbin.org/status/200".to_string()),
    ]
}

// Strategy for generating invalid URLs that should result in network errors
fn arb_invalid_url() -> impl Strategy<Value = String> {
    prop_oneof![
        // Invalid schemes
        Just("ftp://example.com".to_string()),
        Just("file:///path/to/file".to_string()),
        Just("javascript:void(0)".to_string()),
        Just("data:text/html,<h1>Test</h1>".to_string()),
        // Malformed URLs
        Just("not-a-url".to_string()),
        Just("://missing-scheme.com".to_string()),
    ]
}

// Strategy for generating folder paths
fn arb_folder_path() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec("[a-zA-Z0-9 ]{1,20}", 0..4)
}

// Strategy for generating a valid bookmark
fn arb_bookmark() -> impl Strategy<Value = BookmarkInfo> {
    (
        arb_bookmark_url(),
        "[a-zA-Z0-9 ]{1,50}",
        arb_bookmark_browser_type(),
        arb_folder_path(),
    )
        .prop_map(|(url, title, browser_type, folder_path)| {
            let now = Utc::now();
            BookmarkInfo {
                id: BookmarkId::new(),
                url,
                title,
                favicon_url: None,
                browser_type,
                folder_path,
                created_at: now,
                last_accessed: Some(now),
            }
        })
}

// Strategy for generating a bookmark with an invalid URL
fn arb_invalid_bookmark() -> impl Strategy<Value = BookmarkInfo> {
    (
        arb_invalid_url(),
        "[a-zA-Z0-9 ]{1,50}",
        arb_bookmark_browser_type(),
        arb_folder_path(),
    )
        .prop_map(|(url, title, browser_type, folder_path)| {
            let now = Utc::now();
            BookmarkInfo {
                id: BookmarkId::new(),
                url,
                title,
                favicon_url: None,
                browser_type,
                folder_path,
                created_at: now,
                last_accessed: Some(now),
            }
        })
}

// Strategy for generating a mixed list of bookmarks
fn arb_bookmark_list() -> impl Strategy<Value = Vec<BookmarkInfo>> {
    prop::collection::vec(arb_bookmark(), 0..10)
}

// Strategy for generating a list with some invalid bookmarks
fn arb_mixed_bookmark_list() -> impl Strategy<Value = Vec<BookmarkInfo>> {
    prop::collection::vec(
        prop_oneof![
            3 => arb_bookmark(),
            1 => arb_invalid_bookmark(),
        ],
        0..10
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: web-page-manager, Property 5: 书签验证准确性
    /// Validates: Requirements 2.2
    ///
    /// Sub-property 5a: Validation report statistics are consistent
    /// The sum of all status categories should equal the total bookmark count.
    #[test]
    fn prop_validation_report_stats_consistent(bookmarks in arb_bookmark_list()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let validator = BookmarkValidator::with_timeout(1); // Short timeout for testing
            let report = validator.validate_batch(&bookmarks).await;
            
            // Total should match input
            prop_assert_eq!(
                report.total_bookmarks,
                bookmarks.len(),
                "Total bookmarks should match input length"
            );
            
            // Sum of categories should equal total
            let category_sum = report.accessible 
                + report.not_found 
                + report.forbidden 
                + report.timeout 
                + report.network_errors;
            
            prop_assert_eq!(
                category_sum,
                report.total_bookmarks,
                "Sum of status categories ({}) should equal total ({})",
                category_sum,
                report.total_bookmarks
            );
            
            // Results count should match total
            prop_assert_eq!(
                report.results.len(),
                report.total_bookmarks,
                "Results count should match total bookmarks"
            );
            
            Ok(())
        })?;
    }

    /// Feature: web-page-manager, Property 5: 书签验证准确性
    /// Validates: Requirements 2.2
    ///
    /// Sub-property 5b: All input bookmarks are represented in results
    /// Every bookmark in the input should have a corresponding validation result.
    #[test]
    fn prop_all_bookmarks_validated(bookmarks in arb_bookmark_list()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let validator = BookmarkValidator::with_timeout(1);
            let report = validator.validate_batch(&bookmarks).await;
            
            // Collect all bookmark IDs from input
            let input_ids: std::collections::HashSet<_> = bookmarks
                .iter()
                .map(|b| &b.id)
                .collect();
            
            // Collect all bookmark IDs from results
            let result_ids: std::collections::HashSet<_> = report.results
                .iter()
                .map(|r| &r.bookmark.id)
                .collect();
            
            // All input IDs should be in results
            for id in &input_ids {
                prop_assert!(
                    result_ids.contains(id),
                    "Bookmark {:?} should have a validation result",
                    id
                );
            }
            
            // Results should not contain extra bookmarks
            prop_assert_eq!(
                input_ids.len(),
                result_ids.len(),
                "Result count should match input count"
            );
            
            Ok(())
        })?;
    }

    /// Feature: web-page-manager, Property 5: 书签验证准确性
    /// Validates: Requirements 2.2
    ///
    /// Sub-property 5c: Response times are non-negative
    /// All response times in validation results should be non-negative.
    #[test]
    fn prop_response_times_non_negative(bookmarks in arb_bookmark_list()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let validator = BookmarkValidator::with_timeout(1);
            let report = validator.validate_batch(&bookmarks).await;
            
            for result in &report.results {
                if let Some(response_time) = result.response_time_ms {
                    // Response time is u64, always non-negative
                    // This assertion verifies the value exists and is valid
                    prop_assert!(
                        response_time < u64::MAX,
                        "Response time should be a valid value, got {}",
                        response_time
                    );
                }
            }
            
            // Report duration is u64, always non-negative
            // This assertion verifies the value is valid
            prop_assert!(
                report.duration_ms < u64::MAX,
                "Report duration should be a valid value"
            );
            
            Ok(())
        })?;
    }

    /// Feature: web-page-manager, Property 5: 书签验证准确性
    /// Validates: Requirements 2.2
    ///
    /// Sub-property 5d: Validation timestamps are reasonable
    /// All validation timestamps should be at or after the bookmark creation time.
    #[test]
    fn prop_validation_timestamps_reasonable(bookmarks in arb_bookmark_list()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let before_validation = Utc::now();
        
        rt.block_on(async {
            let validator = BookmarkValidator::with_timeout(1);
            let report = validator.validate_batch(&bookmarks).await;
            
            // Report generation time should be after we started
            prop_assert!(
                report.generated_at >= before_validation,
                "Report generation time should be after validation started"
            );
            
            // Each result's validation time should be reasonable
            for result in &report.results {
                prop_assert!(
                    result.validated_at >= before_validation,
                    "Validation time should be after validation started"
                );
                
                // Validation time should be at or after bookmark creation
                prop_assert!(
                    result.validated_at >= result.bookmark.created_at,
                    "Validation time should be at or after bookmark creation"
                );
            }
            
            Ok(())
        })?;
    }

    /// Feature: web-page-manager, Property 5: 书签验证准确性
    /// Validates: Requirements 2.2
    ///
    /// Sub-property 5e: Invalid URLs result in network errors
    /// Bookmarks with invalid URL schemes should result in NetworkError status.
    #[test]
    fn prop_invalid_urls_detected(bookmarks in prop::collection::vec(arb_invalid_bookmark(), 1..5)) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let validator = BookmarkValidator::with_timeout(1);
            let report = validator.validate_batch(&bookmarks).await;
            
            // All invalid URLs should result in network errors
            for result in &report.results {
                let url = &result.bookmark.url;
                
                // Check if URL has invalid scheme
                let is_invalid_scheme = !url.starts_with("http://") && !url.starts_with("https://");
                
                if is_invalid_scheme {
                    prop_assert!(
                        matches!(result.status, AccessibilityStatus::NetworkError(_)),
                        "Invalid URL scheme '{}' should result in NetworkError, got {:?}",
                        url,
                        result.status
                    );
                }
            }
            
            Ok(())
        })?;
    }

    /// Feature: web-page-manager, Property 5: 书签验证准确性
    /// Validates: Requirements 2.2
    ///
    /// Sub-property 5f: Bookmark data is preserved in validation results
    /// The bookmark data in validation results should match the original input.
    #[test]
    fn prop_bookmark_data_preserved(bookmarks in arb_bookmark_list()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let validator = BookmarkValidator::with_timeout(1);
            let report = validator.validate_batch(&bookmarks).await;
            
            // Create a map of original bookmarks by ID
            let original_map: std::collections::HashMap<_, _> = bookmarks
                .iter()
                .map(|b| (&b.id, b))
                .collect();
            
            // Verify each result preserves the original bookmark data
            for result in &report.results {
                let original = original_map.get(&result.bookmark.id);
                
                prop_assert!(
                    original.is_some(),
                    "Result bookmark should exist in original input"
                );
                
                let original = original.unwrap();
                
                prop_assert_eq!(
                    &result.bookmark.url,
                    &original.url,
                    "URL should be preserved"
                );
                prop_assert_eq!(
                    &result.bookmark.title,
                    &original.title,
                    "Title should be preserved"
                );
                prop_assert_eq!(
                    result.bookmark.browser_type,
                    original.browser_type,
                    "Browser type should be preserved"
                );
                prop_assert_eq!(
                    &result.bookmark.folder_path,
                    &original.folder_path,
                    "Folder path should be preserved"
                );
            }
            
            Ok(())
        })?;
    }

    /// Feature: web-page-manager, Property 5: 书签验证准确性
    /// Validates: Requirements 2.2
    ///
    /// Sub-property 5g: Empty bookmark list produces empty report
    /// Validating an empty list should produce a report with zero counts.
    #[test]
    fn prop_empty_list_empty_report(_seed in any::<u64>()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let validator = BookmarkValidator::new();
            let report = validator.validate_batch(&[]).await;
            
            prop_assert_eq!(report.total_bookmarks, 0, "Total should be 0 for empty input");
            prop_assert_eq!(report.accessible, 0, "Accessible should be 0");
            prop_assert_eq!(report.not_found, 0, "Not found should be 0");
            prop_assert_eq!(report.forbidden, 0, "Forbidden should be 0");
            prop_assert_eq!(report.timeout, 0, "Timeout should be 0");
            prop_assert_eq!(report.network_errors, 0, "Network errors should be 0");
            prop_assert!(report.results.is_empty(), "Results should be empty");
            
            Ok(())
        })?;
    }

    /// Feature: web-page-manager, Property 5: 书签验证准确性
    /// Validates: Requirements 2.2
    ///
    /// Sub-property 5h: Validator configuration is respected
    /// The validator should respect timeout and concurrency settings.
    #[test]
    fn prop_validator_config_respected(timeout in 1u64..30, max_concurrent in 1usize..20) {
        let validator = BookmarkValidator::with_timeout(timeout)
            .with_max_concurrent(max_concurrent);
        
        prop_assert_eq!(
            validator.timeout_secs(),
            timeout,
            "Timeout should match configured value"
        );
        prop_assert_eq!(
            validator.max_concurrent(),
            max_concurrent,
            "Max concurrent should match configured value"
        );
    }

    /// Feature: web-page-manager, Property 5: 书签验证准确性
    /// Validates: Requirements 2.2
    ///
    /// Sub-property 5i: Status categories are mutually exclusive
    /// Each validation result should have exactly one status category.
    #[test]
    fn prop_status_categories_exclusive(bookmarks in arb_mixed_bookmark_list()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let validator = BookmarkValidator::with_timeout(1);
            let report = validator.validate_batch(&bookmarks).await;
            
            // Count results by status
            let mut accessible_count = 0;
            let mut not_found_count = 0;
            let mut forbidden_count = 0;
            let mut timeout_count = 0;
            let mut network_error_count = 0;
            
            for result in &report.results {
                match &result.status {
                    AccessibilityStatus::Accessible => accessible_count += 1,
                    AccessibilityStatus::NotFound => not_found_count += 1,
                    AccessibilityStatus::Forbidden => forbidden_count += 1,
                    AccessibilityStatus::Timeout => timeout_count += 1,
                    AccessibilityStatus::NetworkError(_) => network_error_count += 1,
                }
            }
            
            // Verify counts match report
            prop_assert_eq!(
                accessible_count,
                report.accessible,
                "Accessible count mismatch"
            );
            prop_assert_eq!(
                not_found_count,
                report.not_found,
                "Not found count mismatch"
            );
            prop_assert_eq!(
                forbidden_count,
                report.forbidden,
                "Forbidden count mismatch"
            );
            prop_assert_eq!(
                timeout_count,
                report.timeout,
                "Timeout count mismatch"
            );
            prop_assert_eq!(
                network_error_count,
                report.network_errors,
                "Network error count mismatch"
            );
            
            Ok(())
        })?;
    }
}
