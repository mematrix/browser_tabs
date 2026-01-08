// Feature: web-page-manager, Property 16: 数据继承完整性 (Data Inheritance Integrity)
// Validates: Requirements 6.3
//
// Property: For any tab that is converted to a bookmark, the new bookmark's
// UnifiedPageInfo should automatically inherit the content summary and keywords
// from the original tab's UnifiedPageInfo, and the inherited data should be
// consistent with the original analysis result.

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use web_page_manager_core::*;
use chrono::Utc;

// Strategy for generating BrowserType
fn arb_browser_type() -> impl Strategy<Value = BrowserType> {
    prop_oneof![
        Just(BrowserType::Chrome),
        Just(BrowserType::Firefox),
        Just(BrowserType::Edge),
        Just(BrowserType::Safari),
    ]
}

// Strategy for generating ContentType
fn arb_content_type() -> impl Strategy<Value = ContentType> {
    prop_oneof![
        Just(ContentType::Article),
        Just(ContentType::Video),
        Just(ContentType::Documentation),
        Just(ContentType::SocialMedia),
        Just(ContentType::Shopping),
        Just(ContentType::News),
        Just(ContentType::Reference),
        "[a-zA-Z]{1,20}".prop_map(ContentType::Other),
    ]
}

// Strategy for generating ContentSummary
fn arb_content_summary() -> impl Strategy<Value = ContentSummary> {
    (
        "[a-zA-Z0-9 ]{10,200}",  // summary_text
        prop::collection::vec("[a-zA-Z0-9 ]{5,50}", 1..5),  // key_points
        arb_content_type(),
        prop_oneof![Just("en"), Just("zh"), Just("ja"), Just("es")],  // language
        1u32..60u32,  // reading_time_minutes
        0.0f32..1.0f32,  // confidence_score
    )
        .prop_map(|(summary_text, key_points, content_type, language, reading_time_minutes, confidence_score)| {
            ContentSummary {
                summary_text,
                key_points,
                content_type,
                language: language.to_string(),
                reading_time_minutes,
                confidence_score,
                generated_at: Utc::now(),
            }
        })
}

// Strategy for generating TabInfo
fn arb_tab_info() -> impl Strategy<Value = TabInfo> {
    (
        "https?://[a-z0-9.-]+\\.[a-z]{2,4}/[a-zA-Z0-9/_-]*",  // url
        "[a-zA-Z0-9 ]{1,100}",  // title
        prop::option::of("https?://[a-z0-9.-]+\\.[a-z]{2,4}/favicon\\.ico"),  // favicon_url
        arb_browser_type(),
    )
        .prop_map(|(url, title, favicon_url, browser_type)| {
            let now = Utc::now();
            TabInfo {
                id: TabId::new(),
                url,
                title,
                favicon_url,
                browser_type,
                is_private: false,  // Only non-private tabs can be bookmarked
                created_at: now,
                last_accessed: now,
            }
        })
}

// Strategy for generating UnifiedPageInfo with content summary
fn arb_unified_page_with_summary(tab: TabInfo) -> impl Strategy<Value = UnifiedPageInfo> {
    (
        prop::option::of(arb_content_summary()),
        prop::collection::vec("[a-zA-Z0-9]{3,20}", 0..10),  // keywords
        prop::option::of("[a-zA-Z]{5,30}"),  // category
    )
        .prop_map(move |(content_summary, keywords, category)| {
            let now = Utc::now();
            UnifiedPageInfo {
                id: Uuid::new_v4(),
                url: tab.url.clone(),
                title: tab.title.clone(),
                favicon_url: tab.favicon_url.clone(),
                content_summary,
                keywords,
                category,
                source_type: PageSourceType::ActiveTab {
                    browser: tab.browser_type,
                    tab_id: tab.id.clone(),
                },
                browser_info: None,
                tab_info: Some(tab.clone()),
                bookmark_info: None,
                created_at: now,
                last_accessed: now,
                access_count: 1,
            }
        })
}

// Strategy for generating folder paths
fn arb_folder_path() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec("[a-zA-Z0-9 ]{1,30}", 0..5)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    /// Feature: web-page-manager, Property 16: 数据继承完整性
    /// Validates: Requirements 6.3
    ///
    /// For any tab with analyzed content, when creating a bookmark from that tab,
    /// the bookmark's UnifiedPageInfo should inherit the content summary and keywords
    /// from the original tab's UnifiedPageInfo.
    #[test]
    fn prop_data_inheritance_integrity(
        tab in arb_tab_info(),
        folder_path in arb_folder_path(),
    ) {
        // Generate a unified page for the tab
        let unified_page_strategy = arb_unified_page_with_summary(tab.clone());
        let unified_page = unified_page_strategy
            .new_tree(&mut proptest::test_runner::TestRunner::default())
            .unwrap()
            .current();
        
        // Create bookmark from tab
        let (bookmark, bookmark_unified_page) = create_bookmark_from_tab(
            &tab,
            &unified_page,
            folder_path.clone(),
        );
        
        // Property 1: Content summary should be inherited (if present)
        // The bookmark's content summary should be identical to the original
        match (&unified_page.content_summary, &bookmark_unified_page.content_summary) {
            (Some(original), Some(inherited)) => {
                prop_assert_eq!(
                    &original.summary_text,
                    &inherited.summary_text,
                    "Content summary text should be inherited"
                );
                prop_assert_eq!(
                    &original.key_points,
                    &inherited.key_points,
                    "Key points should be inherited"
                );
                prop_assert_eq!(
                    original.reading_time_minutes,
                    inherited.reading_time_minutes,
                    "Reading time should be inherited"
                );
                prop_assert_eq!(
                    original.confidence_score,
                    inherited.confidence_score,
                    "Confidence score should be inherited"
                );
                prop_assert_eq!(
                    &original.language,
                    &inherited.language,
                    "Language should be inherited"
                );
            }
            (None, None) => {
                // Both are None, which is correct
            }
            (Some(_), None) => {
                prop_assert!(false, "Content summary should be inherited when present in original");
            }
            (None, Some(_)) => {
                prop_assert!(false, "Content summary should not appear if not present in original");
            }
        }
        
        // Property 2: Keywords should be inherited
        prop_assert_eq!(
            &unified_page.keywords,
            &bookmark_unified_page.keywords,
            "Keywords should be inherited from original unified page"
        );
        
        // Property 3: Category should be inherited
        prop_assert_eq!(
            &unified_page.category,
            &bookmark_unified_page.category,
            "Category should be inherited from original unified page"
        );
        
        // Property 4: Basic tab info should be preserved in bookmark
        prop_assert_eq!(
            &tab.url,
            &bookmark.url,
            "URL should be preserved in bookmark"
        );
        prop_assert_eq!(
            &tab.title,
            &bookmark.title,
            "Title should be preserved in bookmark"
        );
        prop_assert_eq!(
            &tab.favicon_url,
            &bookmark.favicon_url,
            "Favicon URL should be preserved in bookmark"
        );
        prop_assert_eq!(
            tab.browser_type,
            bookmark.browser_type,
            "Browser type should be preserved in bookmark"
        );
        
        // Property 5: Folder path should be set correctly
        prop_assert_eq!(
            &folder_path,
            &bookmark.folder_path,
            "Folder path should be set correctly"
        );
        
        // Property 6: Source type should be Bookmark
        match &bookmark_unified_page.source_type {
            PageSourceType::Bookmark { browser, bookmark_id } => {
                prop_assert_eq!(
                    *browser,
                    tab.browser_type,
                    "Browser type in source should match tab's browser"
                );
                prop_assert_eq!(
                    bookmark_id,
                    &bookmark.id,
                    "Bookmark ID in source should match created bookmark"
                );
            }
            _ => {
                prop_assert!(false, "Source type should be Bookmark");
            }
        }
        
        // Property 7: Bookmark info should be set in unified page
        prop_assert!(
            bookmark_unified_page.bookmark_info.is_some(),
            "Bookmark info should be set in unified page"
        );
        
        // Property 8: Tab info should not be set in bookmark's unified page
        prop_assert!(
            bookmark_unified_page.tab_info.is_none(),
            "Tab info should not be set in bookmark's unified page"
        );
    }
}
