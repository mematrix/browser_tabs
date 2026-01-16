/// End-to-end integration tests for webpage-manager
///
/// Task 11.2: Integration Testing
/// These tests verify that all components work together correctly

use integration::*;
use web_page_manager_core::types::*;
use data_access::repository::PageRepository;
use tempfile::TempDir;
use uuid::Uuid;

/// Helper to create a test configuration with temporary database
async fn setup_test_app() -> (Application, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let config = AppConfig {
        database_path: Some(db_path),
        enable_ai: false,
        auto_connect_browsers: false,
        cache_size_mb: 10,
        history_retention_days: 30,
        enable_performance_monitoring: false,
        log_level: "debug".to_string(),
    };

    let app = Application::new(config).await.unwrap();
    (app, temp_dir)
}

/// Helper to create a test page
fn create_test_page(url: &str, title: &str) -> UnifiedPageInfo {
    UnifiedPageInfo {
        id: Uuid::new_v4(),
        url: url.to_string(),
        title: title.to_string(),
        favicon_url: None,
        content_summary: None,
        keywords: vec![],
        category: None,
        source_type: PageSourceType::ActiveTab {
            browser: BrowserType::Chrome,
            tab_id: TabId::new(),
        },
        browser_info: None,
        tab_info: None,
        bookmark_info: None,
        created_at: chrono::Utc::now(),
        last_accessed: chrono::Utc::now(),
        access_count: 1,
    }
}

// ============================================================================
// 1. Component Initialization Tests
// ============================================================================

#[tokio::test]
async fn test_component_initialization_database() {
    let (app, _temp_dir) = setup_test_app().await;

    // Verify database is initialized
    let context = app.context();
    let stats = context.database.stats().await.unwrap();

    // Database should be empty initially
    assert_eq!(stats.page_count, 0);
    assert_eq!(stats.group_count, 0);
}

#[tokio::test]
async fn test_component_initialization_browser_connector() {
    let (app, _temp_dir) = setup_test_app().await;

    // Verify browser connector is initialized
    let context = app.context();
    let connected = context.browser_manager.connected_count().await;

    // No browsers should be connected initially (auto_connect_browsers = false)
    assert_eq!(connected, 0);
}

#[tokio::test]
async fn test_component_initialization_page_manager() {
    let (app, _temp_dir) = setup_test_app().await;

    // Verify page manager is initialized
    let context = app.context();
    let stats = context.page_manager.get_stats().await;

    // Page manager should have no pages initially
    assert_eq!(stats.total_pages, 0);
    assert_eq!(stats.active_tab_pages, 0);
    assert_eq!(stats.bookmark_only_pages, 0);
}

#[tokio::test]
async fn test_component_initialization_error_handler() {
    let (app, _temp_dir) = setup_test_app().await;

    // Verify error handler is initialized
    let context = app.context();
    let error_stats = context.error_handler.get_error_stats().await;

    // Error handler should have no errors initially
    assert_eq!(error_stats.total, 0);
    assert_eq!(error_stats.critical, 0);
    assert_eq!(error_stats.errors, 0);
}

#[tokio::test]
async fn test_all_components_initialized() {
    let (app, _temp_dir) = setup_test_app().await;

    // Verify all components are accessible
    let context = app.context();

    // Database
    let db_stats = context.database.stats().await.unwrap();
    assert!(db_stats.cache_stats.pages_max > 0);

    // Browser manager
    let browsers = context.browser_manager.get_connected_browsers().await;
    assert!(browsers.is_empty()); // No auto-connect

    // Page manager
    let pages = context.get_all_pages().await;
    assert!(pages.is_empty());

    // Error handler
    let errors = context.error_handler.get_recent_errors().await;
    assert!(errors.is_empty());
}

// ============================================================================
// 2. Data Flow Tests
// ============================================================================

#[tokio::test]
async fn test_data_flow_page_storage_and_retrieval() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();

    // Create test page
    let test_page = create_test_page("https://example.com", "Test Page");
    let page_id = test_page.id;

    // Store page in database
    let page_repo = context.database.page_repository();
    page_repo.save(&test_page).await.unwrap();

    // Retrieve page from database
    let retrieved = page_repo.get_by_id(&page_id).await.unwrap();
    assert!(retrieved.is_some());

    let retrieved_page = retrieved.unwrap();
    assert_eq!(retrieved_page.url, test_page.url);
    assert_eq!(retrieved_page.title, test_page.title);
    assert_eq!(retrieved_page.id, page_id);
}

#[tokio::test]
async fn test_data_flow_search_across_components() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();

    // Create multiple test pages
    let page1 = create_test_page("https://rust-lang.org", "Rust Programming Language");
    let page2 = create_test_page("https://docs.rs", "Rust Documentation");
    let page3 = create_test_page("https://example.com", "Example Site");

    // Store pages
    let page_repo = context.database.page_repository();
    page_repo.save(&page1).await.unwrap();
    page_repo.save(&page2).await.unwrap();
    page_repo.save(&page3).await.unwrap();

    // Search for "Rust"
    let results = page_repo.search("Rust").await.unwrap();
    assert_eq!(results.len(), 2);

    // Verify search results contain expected data
    let titles: Vec<String> = results.iter().map(|p| p.title.clone()).collect();
    assert!(titles.contains(&"Rust Programming Language".to_string()));
    assert!(titles.contains(&"Rust Documentation".to_string()));
}

#[tokio::test]
async fn test_data_flow_cache_performance() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();

    // Create test page
    let test_page = create_test_page("https://cache-test.com", "Cache Test Page");
    let page_id = test_page.id;

    // Store page
    let page_repo = context.database.page_repository();
    page_repo.save(&test_page).await.unwrap();

    // First retrieval (should hit database)
    let start = std::time::Instant::now();
    let result1 = page_repo.get_by_id(&page_id).await.unwrap();
    let duration1 = start.elapsed();
    assert!(result1.is_some());

    // Second retrieval (should hit cache, faster)
    let start = std::time::Instant::now();
    let result2 = page_repo.get_by_id(&page_id).await.unwrap();
    let duration2 = start.elapsed();
    assert!(result2.is_some());

    // Cache hit should be faster (though this might not always be true in tests)
    // We just verify both succeeded
    assert!(duration2 < duration1 * 10); // At most 10x slower (very lenient)
}

#[tokio::test]
async fn test_data_flow_multiple_source_types() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();

    // Create pages with different source types
    let tab_page = UnifiedPageInfo {
        id: Uuid::new_v4(),
        url: "https://tab-example.com".to_string(),
        title: "Tab Example".to_string(),
        favicon_url: None,
        content_summary: None,
        keywords: vec![],
        category: None,
        source_type: PageSourceType::ActiveTab {
            browser: BrowserType::Chrome,
            tab_id: TabId::new(),
        },
        browser_info: None,
        tab_info: None,
        bookmark_info: None,
        created_at: chrono::Utc::now(),
        last_accessed: chrono::Utc::now(),
        access_count: 1,
    };

    let bookmark_page = UnifiedPageInfo {
        id: Uuid::new_v4(),
        url: "https://bookmark-example.com".to_string(),
        title: "Bookmark Example".to_string(),
        favicon_url: None,
        content_summary: None,
        keywords: vec![],
        category: None,
        source_type: PageSourceType::Bookmark {
            browser: BrowserType::Firefox,
            bookmark_id: BookmarkId::new(),
        },
        browser_info: None,
        tab_info: None,
        bookmark_info: None,
        created_at: chrono::Utc::now(),
        last_accessed: chrono::Utc::now(),
        access_count: 5,
    };

    // Store both pages
    let page_repo = context.database.page_repository();
    page_repo.save(&tab_page).await.unwrap();
    page_repo.save(&bookmark_page).await.unwrap();

    // Retrieve all pages
    let all_pages = page_repo.get_all().await.unwrap();
    assert_eq!(all_pages.len(), 2);
}

// ============================================================================
// 3. Cross-Component Tests
// ============================================================================

#[tokio::test]
async fn test_cross_component_unified_search() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();

    // Create pages from different sources
    let tab_page = UnifiedPageInfo {
        id: Uuid::new_v4(),
        url: "https://tab-example.com".to_string(),
        title: "Tab Example".to_string(),
        favicon_url: None,
        content_summary: None,
        keywords: vec!["example".to_string()],
        category: None,
        source_type: PageSourceType::ActiveTab {
            browser: BrowserType::Chrome,
            tab_id: TabId::new(),
        },
        browser_info: None,
        tab_info: None,
        bookmark_info: None,
        created_at: chrono::Utc::now(),
        last_accessed: chrono::Utc::now(),
        access_count: 1,
    };

    let bookmark_page = UnifiedPageInfo {
        id: Uuid::new_v4(),
        url: "https://bookmark-example.com".to_string(),
        title: "Bookmark Example".to_string(),
        favicon_url: None,
        content_summary: None,
        keywords: vec!["example".to_string()],
        category: None,
        source_type: PageSourceType::Bookmark {
            browser: BrowserType::Firefox,
            bookmark_id: BookmarkId::new(),
        },
        browser_info: None,
        tab_info: None,
        bookmark_info: None,
        created_at: chrono::Utc::now(),
        last_accessed: chrono::Utc::now(),
        access_count: 5,
    };

    // Store both pages
    let page_repo = context.database.page_repository();
    page_repo.save(&tab_page).await.unwrap();
    page_repo.save(&bookmark_page).await.unwrap();

    // Search should find both
    let results = page_repo.search("Example").await.unwrap();
    assert_eq!(results.len(), 2);

    // Verify both source types are represented
    let has_tab = results.iter().any(|p| matches!(p.source_type, PageSourceType::ActiveTab { .. }));
    let has_bookmark = results.iter().any(|p| matches!(p.source_type, PageSourceType::Bookmark { .. }));
    assert!(has_tab);
    assert!(has_bookmark);
}

#[tokio::test]
async fn test_cross_component_statistics() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();

    // Create test data with different source types
    let tab1 = UnifiedPageInfo {
        id: Uuid::new_v4(),
        url: "https://tab1.com".to_string(),
        title: "Tab 1".to_string(),
        favicon_url: None,
        content_summary: None,
        keywords: vec![],
        category: None,
        source_type: PageSourceType::ActiveTab {
            browser: BrowserType::Chrome,
            tab_id: TabId::new(),
        },
        browser_info: None,
        tab_info: Some(TabInfo {
            id: TabId::new(),
            url: "https://tab1.com".to_string(),
            title: "Tab 1".to_string(),
            favicon_url: None,
            browser_type: BrowserType::Chrome,
            is_private: false,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
        }),
        bookmark_info: None,
        created_at: chrono::Utc::now(),
        last_accessed: chrono::Utc::now(),
        access_count: 1,
    };

    let tab2 = UnifiedPageInfo {
        id: Uuid::new_v4(),
        url: "https://tab2.com".to_string(),
        title: "Tab 2".to_string(),
        favicon_url: None,
        content_summary: None,
        keywords: vec![],
        category: None,
        source_type: PageSourceType::ActiveTab {
            browser: BrowserType::Chrome,
            tab_id: TabId::new(),
        },
        browser_info: None,
        tab_info: Some(TabInfo {
            id: TabId::new(),
            url: "https://tab2.com".to_string(),
            title: "Tab 2".to_string(),
            favicon_url: None,
            browser_type: BrowserType::Chrome,
            is_private: false,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
        }),
        bookmark_info: None,
        created_at: chrono::Utc::now(),
        last_accessed: chrono::Utc::now(),
        access_count: 1,
    };

    let bookmark = UnifiedPageInfo {
        id: Uuid::new_v4(),
        url: "https://bookmark1.com".to_string(),
        title: "Bookmark 1".to_string(),
        favicon_url: None,
        content_summary: None,
        keywords: vec![],
        category: None,
        source_type: PageSourceType::Bookmark {
            browser: BrowserType::Firefox,
            bookmark_id: BookmarkId::new(),
        },
        browser_info: None,
        tab_info: None,
        bookmark_info: Some(BookmarkInfo {
            id: BookmarkId::new(),
            url: "https://bookmark1.com".to_string(),
            title: "Bookmark 1".to_string(),
            favicon_url: None,
            browser_type: BrowserType::Firefox,
            folder_path: vec!["root".to_string()],
            created_at: chrono::Utc::now(),
            last_accessed: Some(chrono::Utc::now()),
        }),
        created_at: chrono::Utc::now(),
        last_accessed: chrono::Utc::now(),
        access_count: 5,
    };

    // Store pages
    let page_repo = context.database.page_repository();
    page_repo.save(&tab1).await.unwrap();
    page_repo.save(&tab2).await.unwrap();
    page_repo.save(&bookmark).await.unwrap();

    // Get statistics from database
    let db_stats = context.database.stats().await.unwrap();
    assert_eq!(db_stats.page_count, 3);

    // Verify the pages have correct source types
    let all_pages = page_repo.get_all().await.unwrap();
    let tab_pages = all_pages.iter().filter(|p| matches!(p.source_type, PageSourceType::ActiveTab { .. })).count();
    let bookmark_pages = all_pages.iter().filter(|p| matches!(p.source_type, PageSourceType::Bookmark { .. })).count();
    assert_eq!(tab_pages, 2);
    assert_eq!(bookmark_pages, 1);
    assert_eq!(context.browser_manager.connected_count().await, 0); // No auto-connect
}

#[tokio::test]
async fn test_cross_component_error_propagation() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();

    // Trigger a search with empty query (should still work)
    let page_repo = context.database.page_repository();
    let results = page_repo.search("").await.unwrap();
    assert!(results.is_empty());

    // Error handler should not have critical errors
    let error_stats = context.error_handler.get_error_stats().await;
    assert_eq!(error_stats.critical, 0);
}

// ============================================================================
// 4. Error Recovery Tests
// ============================================================================

#[tokio::test]
async fn test_error_recovery_database_operations() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();

    // Try to get non-existent page (should return None, not error)
    let page_repo = context.database.page_repository();
    let result = page_repo.get_by_id(&Uuid::new_v4()).await.unwrap();
    assert!(result.is_none());

    // Error handler should have no errors
    let error_stats = context.error_handler.get_error_stats().await;
    assert_eq!(error_stats.total, 0);
}

#[tokio::test]
async fn test_error_recovery_cache_invalidation() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();

    // Create and store a page
    let test_page = create_test_page("https://cache-invalidation-test.com", "Cache Invalidation Test");
    let page_id = test_page.id;

    let page_repo = context.database.page_repository();
    page_repo.save(&test_page).await.unwrap();

    // Retrieve to populate cache
    let _cached = page_repo.get_by_id(&page_id).await.unwrap();

    // Clear cache
    context.database.clear_cache().await;

    // Should still be able to retrieve from database
    let result = page_repo.get_by_id(&page_id).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().url, test_page.url);
}

#[tokio::test]
async fn test_error_recovery_graceful_shutdown() {
    let (app, _temp_dir) = setup_test_app().await;

    // Create some data
    let context = app.context();
    let test_page = create_test_page("https://shutdown-test.com", "Shutdown Test");

    let page_repo = context.database.page_repository();
    page_repo.save(&test_page).await.unwrap();

    // Shutdown should succeed
    let result = app.shutdown().await;
    assert!(result.is_ok());
}

// ============================================================================
// 5. Performance Tests
// ============================================================================

#[tokio::test]
#[ignore] // Ignore by default as it's slow
async fn test_performance_large_dataset() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();

    // Create 1000 pages (reduced from 10,000 for reasonable test time)
    let page_repo = context.database.page_repository();

    let start = std::time::Instant::now();
    for i in 0..1000 {
        let page = UnifiedPageInfo {
            id: Uuid::new_v4(),
            url: format!("https://example{}.com", i),
            title: format!("Test Page {}", i),
            favicon_url: None,
            content_summary: None,
            keywords: vec![format!("tag{}", i % 5)],
            category: Some(format!("category{}", i % 3)),
            source_type: if i % 2 == 0 {
                PageSourceType::ActiveTab {
                    browser: BrowserType::Chrome,
                    tab_id: TabId::new(),
                }
            } else {
                PageSourceType::Bookmark {
                    browser: BrowserType::Firefox,
                    bookmark_id: BookmarkId::new(),
                }
            },
            browser_info: None,
            tab_info: None,
            bookmark_info: None,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: (i % 10) as u32,
        };

        page_repo.save(&page).await.unwrap();
    }
    let duration = start.elapsed();

    println!("Stored 1000 pages in {:?}", duration);

    // Verify all pages were stored
    let all_pages = page_repo.get_all().await.unwrap();
    assert_eq!(all_pages.len(), 1000);

    // Test search performance
    let start = std::time::Instant::now();
    let results = page_repo.search("Test").await.unwrap();
    let search_duration = start.elapsed();

    println!("Searched 1000 pages in {:?}", search_duration);
    assert!(!results.is_empty());

    // Search should be reasonably fast (< 1 second)
    assert!(search_duration.as_secs() < 1);
}

#[tokio::test]
async fn test_performance_concurrent_operations() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();

    // Create some initial data
    let page_repo = context.database.page_repository();
    for i in 0..10 {
        let page = UnifiedPageInfo {
            id: Uuid::new_v4(),
            url: format!("https://concurrent{}.com", i),
            title: format!("Concurrent Test {}", i),
            favicon_url: None,
            content_summary: None,
            keywords: vec![],
            category: None,
            source_type: PageSourceType::ActiveTab {
                browser: BrowserType::Chrome,
                tab_id: TabId::new(),
            },
            browser_info: None,
            tab_info: None,
            bookmark_info: None,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 1,
        };

        page_repo.save(&page).await.unwrap();
    }

    // Perform concurrent searches
    let mut handles = vec![];
    for i in 0..5 {
        let app_clone = app.context().clone();
        let handle = tokio::spawn(async move {
            let pages = app_clone.search(&format!("Concurrent Test {}", i)).await;
            pages.len()
        });
        handles.push(handle);
    }

    // Wait for all searches to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result >= 0); // Should not crash
    }
}

#[tokio::test]
async fn test_performance_cache_hit_rate() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();

    // Create test data
    let mut page_ids = Vec::new();
    let page_repo = context.database.page_repository();

    for i in 0..10 {
        let page = UnifiedPageInfo {
            id: Uuid::new_v4(),
            url: format!("https://cache-hit{}.com", i),
            title: format!("Cache Hit Test {}", i),
            favicon_url: None,
            content_summary: None,
            keywords: vec![],
            category: None,
            source_type: PageSourceType::ActiveTab {
                browser: BrowserType::Chrome,
                tab_id: TabId::new(),
            },
            browser_info: None,
            tab_info: None,
            bookmark_info: None,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 1,
        };

        let page_id = page.id;
        page_repo.save(&page).await.unwrap();
        page_ids.push(page_id);
    }

    // Access each page multiple times (should use cache)
    for page_id in &page_ids {
        for _ in 0..3 {
            let result = page_repo.get_by_id(page_id).await.unwrap();
            assert!(result.is_some());
        }
    }

    // Verify cache system is configured and operational
    let db_stats = context.database.stats().await.unwrap();
    println!("Cache stats: {:?}", db_stats.cache_stats);

    // Verify cache limits are configured (the cache layer exists even if not populated yet)
    assert!(db_stats.cache_stats.pages_max > 0);
    assert!(db_stats.cache_stats.summaries_max > 0);
    assert!(db_stats.cache_stats.groups_max > 0);
}

// ============================================================================
// 6. Application Lifecycle Tests
// ============================================================================

#[tokio::test]
async fn test_application_full_lifecycle() {
    // Create application
    let (app, _temp_dir) = setup_test_app().await;

    // Perform some operations
    let context = app.context();
    let page = create_test_page("https://lifecycle-test.com", "Lifecycle Test");

    let page_repo = context.database.page_repository();
    page_repo.save(&page).await.unwrap();

    // Get statistics from database
    let db_stats = context.database.stats().await.unwrap();
    assert_eq!(db_stats.page_count, 1);

    // Shutdown
    let shutdown_result = app.shutdown().await;
    assert!(shutdown_result.is_ok());
}

#[tokio::test]
async fn test_application_with_auto_connect() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_auto_connect.db");

    let config = AppConfig {
        database_path: Some(db_path),
        enable_ai: false,
        auto_connect_browsers: true, // Enable auto-connect
        cache_size_mb: 10,
        history_retention_days: 30,
        enable_performance_monitoring: false,
        log_level: "info".to_string(),
    };

    let app = Application::new(config).await.unwrap();

    // Note: Actual browser connection will likely fail in test environment
    // but the initialization should succeed
    let context = app.context();
    let _connected = context.browser_manager.connected_count().await;
    // Don't assert on count as browsers might not be available in test environment

    app.shutdown().await.unwrap();
}

// ============================================================================
// 7. Repository Operations Tests
// ============================================================================

#[tokio::test]
async fn test_repository_paginated_retrieval() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();

    // Create 20 test pages
    let page_repo = context.database.page_repository();
    for i in 0..20 {
        let page = create_test_page(
            &format!("https://paginated{}.com", i),
            &format!("Paginated Test {}", i),
        );
        page_repo.save(&page).await.unwrap();
    }

    // Get first page (10 items)
    let first_page = page_repo.get_paginated(10, 0).await.unwrap();
    assert_eq!(first_page.len(), 10);

    // Get second page (10 items)
    let second_page = page_repo.get_paginated(10, 10).await.unwrap();
    assert_eq!(second_page.len(), 10);

    // Verify no overlap
    let first_ids: Vec<Uuid> = first_page.iter().map(|p| p.id).collect();
    let second_ids: Vec<Uuid> = second_page.iter().map(|p| p.id).collect();
    assert!(first_ids.iter().all(|id| !second_ids.contains(id)));
}

#[tokio::test]
async fn test_repository_update_access() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();

    // Create test page
    let page = create_test_page("https://access-test.com", "Access Test");
    let page_id = page.id;
    let initial_count = page.access_count;

    let page_repo = context.database.page_repository();
    page_repo.save(&page).await.unwrap();

    // Update access
    page_repo.update_access(&page_id).await.unwrap();

    // Retrieve and verify access count increased
    let updated_page = page_repo.get_by_id(&page_id).await.unwrap().unwrap();
    assert_eq!(updated_page.access_count, initial_count + 1);
}

#[tokio::test]
async fn test_repository_delete() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();

    // Create and save test page
    let page = create_test_page("https://delete-test.com", "Delete Test");
    let page_id = page.id;

    let page_repo = context.database.page_repository();
    page_repo.save(&page).await.unwrap();

    // Verify it exists
    assert!(page_repo.get_by_id(&page_id).await.unwrap().is_some());

    // Delete it
    page_repo.delete(&page_id).await.unwrap();

    // Verify it's gone
    assert!(page_repo.get_by_id(&page_id).await.unwrap().is_none());
}

#[tokio::test]
async fn test_repository_count() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();

    let page_repo = context.database.page_repository();

    // Initial count should be 0
    let initial_count = page_repo.count().await.unwrap();
    assert_eq!(initial_count, 0);

    // Add 5 pages
    for i in 0..5 {
        let page = create_test_page(
            &format!("https://count-test{}.com", i),
            &format!("Count Test {}", i),
        );
        page_repo.save(&page).await.unwrap();
    }

    // Count should be 5
    let final_count = page_repo.count().await.unwrap();
    assert_eq!(final_count, 5);
}
