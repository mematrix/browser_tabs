/// Performance benchmarks for webpage-manager
///
/// Task 11.3: Performance Optimization
/// These tests measure and compare performance improvements

use integration::*;
use web_page_manager_core::types::*;
use data_access::repository::PageRepository;
use data_access::batch::BatchPageOperations;
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
        log_level: "info".to_string(),
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

/// Benchmark: Individual vs Batch Insert Performance
#[tokio::test]
async fn bench_individual_vs_batch_insert() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();
    let page_repo = context.database.page_repository();

    // Create 100 test pages
    let test_pages: Vec<UnifiedPageInfo> = (0..100)
        .map(|i| create_test_page(&format!("https://bench{}.com", i), &format!("Bench Page {}", i)))
        .collect();

    // Test 1: Individual inserts
    let individual_pages: Vec<UnifiedPageInfo> = (0..100)
        .map(|i| create_test_page(&format!("https://individual{}.com", i), &format!("Individual {}", i)))
        .collect();

    let start = std::time::Instant::now();
    for page in &individual_pages {
        page_repo.save(page).await.unwrap();
    }
    let individual_duration = start.elapsed();

    println!("Individual inserts (100 pages): {:?}", individual_duration);

    // Test 2: Batch insert
    let batch_pages: Vec<UnifiedPageInfo> = (0..100)
        .map(|i| create_test_page(&format!("https://batch{}.com", i), &format!("Batch {}", i)))
        .collect();

    let batch_ops = context.database.batch_operations();
    let start = std::time::Instant::now();
    batch_ops.batch_save(&batch_pages).await.unwrap();
    let batch_duration = start.elapsed();

    println!("Batch insert (100 pages): {:?}", batch_duration);

    // Calculate speedup
    let speedup = individual_duration.as_secs_f64() / batch_duration.as_secs_f64();
    println!("Batch speedup: {:.2}x faster", speedup);

    // Batch should be significantly faster
    assert!(batch_duration < individual_duration);
    assert!(speedup > 5.0, "Batch operations should be at least 5x faster");
}

/// Benchmark: Large Dataset Performance (1000 pages)
#[tokio::test]
#[ignore] // Ignore by default as it's slow
async fn bench_large_dataset_optimized() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();
    let page_repo = context.database.page_repository();
    let batch_ops = context.database.batch_operations();

    // Create 1000 pages
    let pages: Vec<UnifiedPageInfo> = (0..1000)
        .map(|i| UnifiedPageInfo {
            id: Uuid::new_v4(),
            url: format!("https://optimized{}.com", i),
            title: format!("Optimized Page {}", i),
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
        })
        .collect();

    // Batch insert
    let start = std::time::Instant::now();
    batch_ops.batch_save(&pages).await.unwrap();
    let insert_duration = start.elapsed();

    println!("Batch inserted 1000 pages in {:?}", insert_duration);

    // Verify all pages were stored
    let all_pages = page_repo.get_all().await.unwrap();
    assert_eq!(all_pages.len(), 1000);

    // Test search performance
    let start = std::time::Instant::now();
    let results = page_repo.search("Optimized").await.unwrap();
    let search_duration = start.elapsed();

    println!("Searched 1000 pages in {:?}", search_duration);
    assert!(!results.is_empty());

    // Search should be fast (< 10ms)
    assert!(search_duration.as_millis() < 10, "Search should be under 10ms");

    // Insert should be reasonably fast (< 5 seconds with optimizations)
    assert!(insert_duration.as_secs() < 5, "Batch insert should be under 5 seconds");
}

/// Benchmark: Search Performance with Different Query Types
#[tokio::test]
async fn bench_search_performance() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();
    let batch_ops = context.database.batch_operations();
    let page_repo = context.database.page_repository();

    // Create diverse test data
    let pages: Vec<UnifiedPageInfo> = (0..500)
        .map(|i| UnifiedPageInfo {
            id: Uuid::new_v4(),
            url: format!("https://search{}.com", i),
            title: format!("Search Test {} for {}", i, if i % 3 == 0 { "Rust" } else if i % 3 == 1 { "Python" } else { "JavaScript" }),
            favicon_url: None,
            content_summary: None,
            keywords: vec![
                format!("programming"),
                format!("{}", if i % 3 == 0 { "rust" } else if i % 3 == 1 { "python" } else { "javascript" }),
            ],
            category: Some("programming".to_string()),
            source_type: PageSourceType::Bookmark {
                browser: BrowserType::Chrome,
                bookmark_id: BookmarkId::new(),
            },
            browser_info: None,
            tab_info: None,
            bookmark_info: None,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 0,
        })
        .collect();

    batch_ops.batch_save(&pages).await.unwrap();

    // Test 1: Single word search
    let start = std::time::Instant::now();
    let results1 = page_repo.search("Rust").await.unwrap();
    let duration1 = start.elapsed();
    println!("Single word search 'Rust': {:?}, {} results", duration1, results1.len());
    assert!(duration1.as_millis() < 10);

    // Test 2: Common word search
    let start = std::time::Instant::now();
    let results2 = page_repo.search("programming").await.unwrap();
    let duration2 = start.elapsed();
    println!("Common word search 'programming': {:?}, {} results", duration2, results2.len());
    assert!(duration2.as_millis() < 10);

    // Test 3: Prefix search
    let start = std::time::Instant::now();
    let results3 = page_repo.search("Sear").await.unwrap();
    let duration3 = start.elapsed();
    println!("Prefix search 'Sear': {:?}, {} results", duration3, results3.len());
    assert!(duration3.as_millis() < 10);
}

/// Benchmark: Concurrent Operations
#[tokio::test]
async fn bench_concurrent_operations() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();
    let batch_ops = context.database.batch_operations();

    // Prepare test data
    let pages: Vec<UnifiedPageInfo> = (0..100)
        .map(|i| create_test_page(&format!("https://concurrent{}.com", i), &format!("Concurrent Test {}", i)))
        .collect();

    batch_ops.batch_save(&pages).await.unwrap();

    // Perform 5 concurrent searches
    let start = std::time::Instant::now();
    let mut handles = vec![];

    for i in 0..5 {
        let app_clone = app.context().clone();
        let handle = tokio::spawn(async move {
            let page_repo = app_clone.database.page_repository();
            let results = page_repo.search(&format!("Concurrent Test {}", i)).await;
            results.map(|r| r.len()).unwrap_or(0)
        });
        handles.push(handle);
    }

    // Wait for all searches to complete
    let mut total_results = 0;
    for handle in handles {
        let result = handle.await.unwrap();
        total_results += result;
    }

    let duration = start.elapsed();
    println!("5 concurrent searches completed in {:?}, {} total results", duration, total_results);

    // Concurrent operations should complete quickly
    assert!(duration.as_millis() < 100);
}

/// Benchmark: Batch Delete Performance
#[tokio::test]
async fn bench_batch_delete() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();
    let batch_ops = context.database.batch_operations();
    let page_repo = context.database.page_repository();

    // Create and save pages
    let pages: Vec<UnifiedPageInfo> = (0..200)
        .map(|i| create_test_page(&format!("https://delete{}.com", i), &format!("Delete Test {}", i)))
        .collect();

    let ids: Vec<Uuid> = pages.iter().map(|p| p.id).collect();

    batch_ops.batch_save(&pages).await.unwrap();

    // Verify pages exist
    let count_before = page_repo.count().await.unwrap();
    assert_eq!(count_before, 200);

    // Batch delete
    let start = std::time::Instant::now();
    let deleted = batch_ops.batch_delete(&ids).await.unwrap();
    let duration = start.elapsed();

    println!("Batch deleted 200 pages in {:?}", duration);
    assert_eq!(deleted, 200);

    // Verify pages are deleted
    let count_after = page_repo.count().await.unwrap();
    assert_eq!(count_after, 0);

    // Batch delete should be fast (< 50ms)
    assert!(duration.as_millis() < 50);
}

/// Benchmark: Cache Hit Rate
#[tokio::test]
async fn bench_cache_effectiveness() {
    let (app, _temp_dir) = setup_test_app().await;
    let context = app.context();
    let page_repo = context.database.page_repository();

    // Create and save page
    let page = create_test_page("https://cache-test.com", "Cache Test");
    let page_id = page.id;

    page_repo.save(&page).await.unwrap();

    // First access (cold - from database)
    let start = std::time::Instant::now();
    let _result1 = page_repo.get_by_id(&page_id).await.unwrap();
    let cold_duration = start.elapsed();

    // Subsequent accesses (warm - should use cache potentially)
    let start = std::time::Instant::now();
    for _ in 0..10 {
        let _result = page_repo.get_by_id(&page_id).await.unwrap();
    }
    let warm_total = start.elapsed();
    let warm_avg = warm_total / 10;

    println!("Cold access: {:?}", cold_duration);
    println!("Warm access (avg of 10): {:?}", warm_avg);
    println!("Cache stats: {:?}", context.database.cache().stats().await);

    // All accesses should be fast
    assert!(cold_duration.as_millis() < 10);
    assert!(warm_avg.as_micros() < 5000); // Less than 5ms average
}
