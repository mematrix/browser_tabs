// Feature: web-page-manager, Property 6: AI内容分析完整性 (AI Content Analysis Completeness)
// Validates: Requirements 2.3
//
// Property: For any accessible Web page content, the AI processor should generate
// a complete analysis result containing summary, keywords, and classification,
// and the analysis quality should meet the minimum confidence requirement.
//
// This property test validates:
// 1. Summary generation produces non-empty results for non-empty content
// 2. Keyword extraction produces relevant keywords from content
// 3. Content classification produces valid category information
// 4. Confidence scores are within valid range [0.0, 1.0]
// 5. Analysis results are consistent for the same input

use proptest::prelude::*;
use ai_processor_ffi::{
    PageContentInput, CContentType,
    ai_processor_create, ai_processor_destroy, ai_processor_generate_summary,
    ai_processor_extract_keywords, ai_processor_classify_content,
    ai_processor_free_summary, ai_processor_free_keywords, ai_processor_free_category,
};
use std::ffi::{CStr, CString};
use std::ptr;

// ============================================================================
// Test Data Generators
// ============================================================================

/// Strategy for generating valid article content
fn arb_article_content() -> impl Strategy<Value = PageContentInput> {
    (
        // Generate meaningful text content (multiple sentences)
        prop::collection::vec(
            "[A-Z][a-z]{3,15}( [a-z]{2,10}){3,8}\\.",
            3..10
        ),
        // Title
        "[A-Z][a-z]{2,10}( [A-Z][a-z]{2,10}){1,4}",
        // Optional description
        prop::option::of("[A-Z][a-z]{3,15}( [a-z]{2,10}){5,15}\\."),
        // Keywords
        prop::collection::vec("[a-z]{4,12}", 0..5),
    )
        .prop_map(|(sentences, title, description, keywords)| {
            let text = sentences.join(" ");
            PageContentInput {
                html: format!("<html><head><title>{}</title></head><body><p>{}</p></body></html>", title, text),
                text,
                title,
                description,
                keywords,
                images: vec![],
                links: vec![],
            }
        })
}

/// Strategy for generating video-related content
fn arb_video_content() -> impl Strategy<Value = PageContentInput> {
    (
        "[A-Z][a-z]{2,10}( [a-z]{2,10}){2,6}",
        prop::collection::vec("[a-z]{4,10}", 0..3),
    )
        .prop_map(|(base_title, keywords)| {
            let title = format!("YouTube - {} Video", base_title);
            let text = format!("Watch this amazing video about {}. Subscribe for more content.", base_title);
            PageContentInput {
                html: format!("<html><head><title>{}</title></head><body>{}</body></html>", title, text),
                text,
                title,
                description: Some("A video description".to_string()),
                keywords,
                images: vec!["https://example.com/thumbnail.jpg".to_string()],
                links: vec![],
            }
        })
}

/// Strategy for generating documentation content
fn arb_documentation_content() -> impl Strategy<Value = PageContentInput> {
    (
        "[A-Z][a-z]{3,10}",
        prop::collection::vec("[a-z]{4,10}", 0..3),
    )
        .prop_map(|(api_name, keywords)| {
            let title = format!("{} API Documentation", api_name);
            let text = format!(
                "This function takes a parameter and returns a value. \
                 The {} API provides methods for data processing. \
                 See the reference guide for more details.",
                api_name
            );
            PageContentInput {
                html: format!("<html><head><title>{}</title></head><body><h1>{}</h1><p>{}</p></body></html>", title, title, text),
                text,
                title,
                description: Some(format!("Documentation for {} API", api_name)),
                keywords,
                images: vec![],
                links: vec!["https://docs.example.com/api".to_string()],
            }
        })
}

/// Strategy for generating shopping content
fn arb_shopping_content() -> impl Strategy<Value = PageContentInput> {
    (
        "[A-Z][a-z]{3,12}",
        "[0-9]{2,4}\\.[0-9]{2}",
        prop::collection::vec("[a-z]{4,10}", 0..3),
    )
        .prop_map(|(product_name, price, keywords)| {
            let title = format!("Buy {} - Great Deals", product_name);
            let text = format!(
                "Add to cart and checkout now. {} is available for ${}. \
                 Free shipping on orders over $50.",
                product_name, price
            );
            PageContentInput {
                html: format!("<html><head><title>{}</title></head><body>{}</body></html>", title, text),
                text,
                title,
                description: Some(format!("Buy {} at the best price", product_name)),
                keywords,
                images: vec!["https://shop.example.com/product.jpg".to_string()],
                links: vec![],
            }
        })
}

/// Strategy for generating news content
fn arb_news_content() -> impl Strategy<Value = PageContentInput> {
    (
        "[A-Z][a-z]{3,15}( [a-z]{2,10}){2,5}",
        prop::collection::vec("[a-z]{4,10}", 0..3),
    )
        .prop_map(|(headline, keywords)| {
            let title = format!("Breaking News: {}", headline);
            let text = format!(
                "The reporter covered the story about {}. \
                 This is a developing situation. \
                 More updates to follow.",
                headline
            );
            PageContentInput {
                html: format!("<html><head><title>{}</title></head><body><article>{}</article></body></html>", title, text),
                text,
                title,
                description: Some(format!("Latest news about {}", headline)),
                keywords,
                images: vec![],
                links: vec![],
            }
        })
}

/// Strategy for generating mixed content types
fn arb_any_content() -> impl Strategy<Value = PageContentInput> {
    prop_oneof![
        3 => arb_article_content(),
        1 => arb_video_content(),
        1 => arb_documentation_content(),
        1 => arb_shopping_content(),
        1 => arb_news_content(),
    ]
}

/// Strategy for generating minimal valid content
fn arb_minimal_content() -> impl Strategy<Value = PageContentInput> {
    "[A-Z][a-z]{5,20}".prop_map(|text| {
        PageContentInput {
            html: String::new(),
            text: text.clone(),
            title: text,
            description: None,
            keywords: vec![],
            images: vec![],
            links: vec![],
        }
    })
}

/// Strategy for generating content with substantial text
fn arb_substantial_content() -> impl Strategy<Value = PageContentInput> {
    prop::collection::vec(
        "[A-Z][a-z]{3,12}( [a-z]{2,10}){5,15}\\.",
        5..20
    )
        .prop_map(|sentences| {
            let text = sentences.join(" ");
            let title = "Substantial Content Article".to_string();
            PageContentInput {
                html: format!("<html><body><p>{}</p></body></html>", text),
                text,
                title,
                description: Some("A substantial article with lots of content".to_string()),
                keywords: vec!["article".to_string(), "content".to_string()],
                images: vec![],
                links: vec![],
            }
        })
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert PageContentInput to JSON string for FFI
fn content_to_json(content: &PageContentInput) -> CString {
    let json = serde_json::to_string(content).unwrap_or_default();
    CString::new(json).unwrap_or_default()
}

/// Safely extract string from C pointer
unsafe fn extract_c_string(ptr: *mut std::os::raw::c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
    }
}

/// Safely extract string array from C pointers
unsafe fn extract_c_string_array(ptr: *mut *mut std::os::raw::c_char, count: usize) -> Vec<String> {
    if ptr.is_null() || count == 0 {
        return vec![];
    }
    
    let slice = std::slice::from_raw_parts(ptr, count);
    slice.iter()
        .filter_map(|&p| {
            if p.is_null() {
                None
            } else {
                Some(CStr::from_ptr(p).to_string_lossy().into_owned())
            }
        })
        .collect()
}

// ============================================================================
// Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: web-page-manager, Property 6: AI内容分析完整性
    /// Validates: Requirements 2.3
    ///
    /// Sub-property 6a: Summary generation produces non-empty results for non-empty content
    /// For any page content with non-empty text, the AI processor should generate
    /// a non-empty summary.
    #[test]
    fn prop_summary_generated_for_nonempty_content(content in arb_any_content()) {
        unsafe {
            let processor = ai_processor_create();
            prop_assert!(!processor.is_null(), "Processor should be created");
            
            let json = content_to_json(&content);
            let summary = ai_processor_generate_summary(processor, json.as_ptr());
            
            // For non-empty content, summary should be generated
            if !content.text.is_empty() {
                let summary_text = extract_c_string(summary.summary_text);
                prop_assert!(
                    summary_text.is_some() && !summary_text.as_ref().unwrap().is_empty(),
                    "Summary should be non-empty for content with text: {:?}",
                    content.text.chars().take(50).collect::<String>()
                );
            }
            
            ai_processor_free_summary(summary);
            ai_processor_destroy(processor);
        }
    }

    /// Feature: web-page-manager, Property 6: AI内容分析完整性
    /// Validates: Requirements 2.3
    ///
    /// Sub-property 6b: Confidence scores are within valid range
    /// For any content analysis, the confidence score should be between 0.0 and 1.0.
    #[test]
    fn prop_confidence_score_in_valid_range(content in arb_any_content()) {
        unsafe {
            let processor = ai_processor_create();
            prop_assert!(!processor.is_null(), "Processor should be created");
            
            let json = content_to_json(&content);
            let summary = ai_processor_generate_summary(processor, json.as_ptr());
            
            // Confidence score should be in [0.0, 1.0]
            prop_assert!(
                summary.confidence_score >= 0.0 && summary.confidence_score <= 1.0,
                "Confidence score {} should be in range [0.0, 1.0]",
                summary.confidence_score
            );
            
            ai_processor_free_summary(summary);
            ai_processor_destroy(processor);
        }
    }

    /// Feature: web-page-manager, Property 6: AI内容分析完整性
    /// Validates: Requirements 2.3
    ///
    /// Sub-property 6c: Content classification produces valid category
    /// For any page content, the classification should produce a valid primary category.
    #[test]
    fn prop_classification_produces_valid_category(content in arb_any_content()) {
        unsafe {
            let processor = ai_processor_create();
            prop_assert!(!processor.is_null(), "Processor should be created");
            
            let json = content_to_json(&content);
            let category = ai_processor_classify_content(processor, json.as_ptr());
            
            // Primary category should be non-null and non-empty
            let primary = extract_c_string(category.primary_category);
            prop_assert!(
                primary.is_some() && !primary.as_ref().unwrap().is_empty(),
                "Primary category should be non-empty"
            );
            
            // Category confidence should be in valid range
            prop_assert!(
                category.confidence >= 0.0 && category.confidence <= 1.0,
                "Category confidence {} should be in range [0.0, 1.0]",
                category.confidence
            );
            
            ai_processor_free_category(category);
            ai_processor_destroy(processor);
        }
    }

    /// Feature: web-page-manager, Property 6: AI内容分析完整性
    /// Validates: Requirements 2.3
    ///
    /// Sub-property 6d: Keyword extraction produces relevant keywords
    /// For any content with substantial text, keyword extraction should produce
    /// at least some keywords.
    #[test]
    fn prop_keywords_extracted_from_substantial_content(content in arb_substantial_content()) {
        unsafe {
            let processor = ai_processor_create();
            prop_assert!(!processor.is_null(), "Processor should be created");
            
            let json = content_to_json(&content);
            let mut keywords_ptr: *mut *mut std::os::raw::c_char = ptr::null_mut();
            let mut count: usize = 0;
            
            let result = ai_processor_extract_keywords(
                processor,
                json.as_ptr(),
                &mut keywords_ptr,
                &mut count,
            );
            
            prop_assert_eq!(result, 0, "Keyword extraction should succeed");
            
            // For substantial content, we should get some keywords
            prop_assert!(
                count > 0,
                "Should extract at least one keyword from substantial content"
            );
            
            // Keywords should be non-empty strings
            let keywords = extract_c_string_array(keywords_ptr, count);
            for kw in &keywords {
                prop_assert!(
                    !kw.is_empty(),
                    "Each keyword should be non-empty"
                );
            }
            
            ai_processor_free_keywords(keywords_ptr, count);
            ai_processor_destroy(processor);
        }
    }

    /// Feature: web-page-manager, Property 6: AI内容分析完整性
    /// Validates: Requirements 2.3
    ///
    /// Sub-property 6e: Analysis is deterministic for same input
    /// For any content, analyzing it twice should produce consistent results.
    #[test]
    fn prop_analysis_is_deterministic(content in arb_any_content()) {
        unsafe {
            let processor = ai_processor_create();
            prop_assert!(!processor.is_null(), "Processor should be created");
            
            let json = content_to_json(&content);
            
            // First analysis
            let summary1 = ai_processor_generate_summary(processor, json.as_ptr());
            let summary_text1 = extract_c_string(summary1.summary_text);
            let content_type1 = summary1.content_type;
            let language1 = extract_c_string(summary1.language);
            
            // Second analysis
            let summary2 = ai_processor_generate_summary(processor, json.as_ptr());
            let summary_text2 = extract_c_string(summary2.summary_text);
            let content_type2 = summary2.content_type;
            let language2 = extract_c_string(summary2.language);
            
            // Results should be identical
            prop_assert_eq!(
                summary_text1, summary_text2,
                "Summary text should be deterministic"
            );
            prop_assert_eq!(
                content_type1, content_type2,
                "Content type should be deterministic"
            );
            prop_assert_eq!(
                language1, language2,
                "Language detection should be deterministic"
            );
            
            ai_processor_free_summary(summary1);
            ai_processor_free_summary(summary2);
            ai_processor_destroy(processor);
        }
    }

    /// Feature: web-page-manager, Property 6: AI内容分析完整性
    /// Validates: Requirements 2.3
    ///
    /// Sub-property 6f: Reading time estimation is positive
    /// For any non-empty content, the estimated reading time should be at least 1 minute.
    #[test]
    fn prop_reading_time_is_positive(content in arb_any_content()) {
        unsafe {
            let processor = ai_processor_create();
            prop_assert!(!processor.is_null(), "Processor should be created");
            
            let json = content_to_json(&content);
            let summary = ai_processor_generate_summary(processor, json.as_ptr());
            
            // Reading time should be at least 1 minute for any content
            prop_assert!(
                summary.reading_time_minutes >= 1,
                "Reading time should be at least 1 minute, got {}",
                summary.reading_time_minutes
            );
            
            ai_processor_free_summary(summary);
            ai_processor_destroy(processor);
        }
    }

    /// Feature: web-page-manager, Property 6: AI内容分析完整性
    /// Validates: Requirements 2.3
    ///
    /// Sub-property 6g: Content type classification is valid
    /// For any content, the classified content type should be a valid enum value.
    #[test]
    fn prop_content_type_is_valid(content in arb_any_content()) {
        unsafe {
            let processor = ai_processor_create();
            prop_assert!(!processor.is_null(), "Processor should be created");
            
            let json = content_to_json(&content);
            let summary = ai_processor_generate_summary(processor, json.as_ptr());
            
            // Content type should be a valid enum value (0-7)
            prop_assert!(
                summary.content_type >= 0 && summary.content_type <= 7,
                "Content type {} should be a valid enum value (0-7)",
                summary.content_type
            );
            
            ai_processor_free_summary(summary);
            ai_processor_destroy(processor);
        }
    }

    /// Feature: web-page-manager, Property 6: AI内容分析完整性
    /// Validates: Requirements 2.3
    ///
    /// Sub-property 6h: Video content is correctly classified
    /// Content with video-related keywords should be classified as Video type.
    #[test]
    fn prop_video_content_classified_correctly(content in arb_video_content()) {
        unsafe {
            let processor = ai_processor_create();
            prop_assert!(!processor.is_null(), "Processor should be created");
            
            let json = content_to_json(&content);
            let summary = ai_processor_generate_summary(processor, json.as_ptr());
            
            // Video content should be classified as Video (type 1)
            prop_assert_eq!(
                summary.content_type,
                CContentType::Video as i32,
                "Video content should be classified as Video type"
            );
            
            ai_processor_free_summary(summary);
            ai_processor_destroy(processor);
        }
    }

    /// Feature: web-page-manager, Property 6: AI内容分析完整性
    /// Validates: Requirements 2.3
    ///
    /// Sub-property 6i: Documentation content is correctly classified
    /// Content with documentation-related keywords should be classified as Documentation type.
    #[test]
    fn prop_documentation_content_classified_correctly(content in arb_documentation_content()) {
        unsafe {
            let processor = ai_processor_create();
            prop_assert!(!processor.is_null(), "Processor should be created");
            
            let json = content_to_json(&content);
            let summary = ai_processor_generate_summary(processor, json.as_ptr());
            
            // Documentation content should be classified as Documentation (type 2)
            prop_assert_eq!(
                summary.content_type,
                CContentType::Documentation as i32,
                "Documentation content should be classified as Documentation type"
            );
            
            ai_processor_free_summary(summary);
            ai_processor_destroy(processor);
        }
    }

    /// Feature: web-page-manager, Property 6: AI内容分析完整性
    /// Validates: Requirements 2.3
    ///
    /// Sub-property 6j: Shopping content is correctly classified
    /// Content with shopping-related keywords should be classified as Shopping type.
    #[test]
    fn prop_shopping_content_classified_correctly(content in arb_shopping_content()) {
        unsafe {
            let processor = ai_processor_create();
            prop_assert!(!processor.is_null(), "Processor should be created");
            
            let json = content_to_json(&content);
            let summary = ai_processor_generate_summary(processor, json.as_ptr());
            
            // Shopping content should be classified as Shopping (type 4)
            prop_assert_eq!(
                summary.content_type,
                CContentType::Shopping as i32,
                "Shopping content should be classified as Shopping type"
            );
            
            ai_processor_free_summary(summary);
            ai_processor_destroy(processor);
        }
    }

    /// Feature: web-page-manager, Property 6: AI内容分析完整性
    /// Validates: Requirements 2.3
    ///
    /// Sub-property 6k: News content is correctly classified
    /// Content with news-related keywords should be classified as News type.
    #[test]
    fn prop_news_content_classified_correctly(content in arb_news_content()) {
        unsafe {
            let processor = ai_processor_create();
            prop_assert!(!processor.is_null(), "Processor should be created");
            
            let json = content_to_json(&content);
            let summary = ai_processor_generate_summary(processor, json.as_ptr());
            
            // News content should be classified as News (type 5)
            prop_assert_eq!(
                summary.content_type,
                CContentType::News as i32,
                "News content should be classified as News type"
            );
            
            ai_processor_free_summary(summary);
            ai_processor_destroy(processor);
        }
    }

    /// Feature: web-page-manager, Property 6: AI内容分析完整性
    /// Validates: Requirements 2.3
    ///
    /// Sub-property 6l: Language detection produces valid language code
    /// For any content, language detection should produce a valid language code.
    #[test]
    fn prop_language_detection_produces_valid_code(content in arb_any_content()) {
        unsafe {
            let processor = ai_processor_create();
            prop_assert!(!processor.is_null(), "Processor should be created");
            
            let json = content_to_json(&content);
            let summary = ai_processor_generate_summary(processor, json.as_ptr());
            
            let language = extract_c_string(summary.language);
            prop_assert!(
                language.is_some(),
                "Language should be detected"
            );
            
            let lang = language.unwrap();
            // Language code should be a valid 2-letter code
            let valid_codes = ["en", "zh", "ja", "ko", "es", "fr", "de", "ru", "ar"];
            prop_assert!(
                valid_codes.contains(&lang.as_str()),
                "Language code '{}' should be a valid code",
                lang
            );
            
            ai_processor_free_summary(summary);
            ai_processor_destroy(processor);
        }
    }

    /// Feature: web-page-manager, Property 6: AI内容分析完整性
    /// Validates: Requirements 2.3
    ///
    /// Sub-property 6m: Key points are extracted from substantial content
    /// For content with multiple sentences, key points should be extracted.
    #[test]
    fn prop_key_points_extracted(content in arb_substantial_content()) {
        unsafe {
            let processor = ai_processor_create();
            prop_assert!(!processor.is_null(), "Processor should be created");
            
            let json = content_to_json(&content);
            let summary = ai_processor_generate_summary(processor, json.as_ptr());
            
            // For substantial content, we should have key points
            let key_points = extract_c_string_array(summary.key_points, summary.key_points_count);
            prop_assert!(
                !key_points.is_empty(),
                "Should extract key points from substantial content"
            );
            
            // Each key point should be non-empty
            for kp in &key_points {
                prop_assert!(
                    !kp.is_empty(),
                    "Each key point should be non-empty"
                );
            }
            
            ai_processor_free_summary(summary);
            ai_processor_destroy(processor);
        }
    }

    /// Feature: web-page-manager, Property 6: AI内容分析完整性
    /// Validates: Requirements 2.3
    ///
    /// Sub-property 6n: Higher quality content produces higher confidence
    /// Content with title, description, and substantial text should have higher
    /// confidence than minimal content.
    #[test]
    fn prop_quality_affects_confidence(
        minimal in arb_minimal_content(),
        substantial in arb_substantial_content()
    ) {
        unsafe {
            let processor = ai_processor_create();
            prop_assert!(!processor.is_null(), "Processor should be created");
            
            let minimal_json = content_to_json(&minimal);
            let substantial_json = content_to_json(&substantial);
            
            let minimal_summary = ai_processor_generate_summary(processor, minimal_json.as_ptr());
            let substantial_summary = ai_processor_generate_summary(processor, substantial_json.as_ptr());
            
            // Substantial content should generally have higher or equal confidence
            // (allowing for some variance in generated content)
            prop_assert!(
                substantial_summary.confidence_score >= minimal_summary.confidence_score - 0.1,
                "Substantial content confidence ({}) should be >= minimal content confidence ({}) - 0.1",
                substantial_summary.confidence_score,
                minimal_summary.confidence_score
            );
            
            ai_processor_free_summary(minimal_summary);
            ai_processor_free_summary(substantial_summary);
            ai_processor_destroy(processor);
        }
    }

    /// Feature: web-page-manager, Property 6: AI内容分析完整性
    /// Validates: Requirements 2.3
    ///
    /// Sub-property 6o: Category has secondary categories
    /// For classified content, secondary categories should be provided.
    #[test]
    fn prop_category_has_secondary_categories(content in arb_any_content()) {
        unsafe {
            let processor = ai_processor_create();
            prop_assert!(!processor.is_null(), "Processor should be created");
            
            let json = content_to_json(&content);
            let category = ai_processor_classify_content(processor, json.as_ptr());
            
            // Most content types should have secondary categories
            // (except "Other" which may have none)
            let primary = extract_c_string(category.primary_category);
            if primary.as_ref().map(|p| p != "Other").unwrap_or(false) {
                let secondary = extract_c_string_array(
                    category.secondary_categories,
                    category.secondary_count
                );
                prop_assert!(
                    !secondary.is_empty(),
                    "Non-Other categories should have secondary categories"
                );
            }
            
            ai_processor_free_category(category);
            ai_processor_destroy(processor);
        }
    }
}

// ============================================================================
// Unit Tests for Edge Cases
// ============================================================================

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_empty_content_handling() {
        unsafe {
            let processor = ai_processor_create();
            assert!(!processor.is_null());
            
            let content = PageContentInput {
                html: String::new(),
                text: String::new(),
                title: String::new(),
                description: None,
                keywords: vec![],
                images: vec![],
                links: vec![],
            };
            
            let json = content_to_json(&content);
            let summary = ai_processor_generate_summary(processor, json.as_ptr());
            
            // Should handle empty content gracefully
            assert!(summary.confidence_score >= 0.0);
            assert!(summary.confidence_score <= 1.0);
            
            ai_processor_free_summary(summary);
            ai_processor_destroy(processor);
        }
    }

    #[test]
    fn test_null_processor_handling() {
        unsafe {
            let content = PageContentInput {
                html: String::new(),
                text: "Test content".to_string(),
                title: "Test".to_string(),
                description: None,
                keywords: vec![],
                images: vec![],
                links: vec![],
            };
            
            let json = content_to_json(&content);
            
            // Should handle null processor gracefully
            let summary = ai_processor_generate_summary(ptr::null_mut(), json.as_ptr());
            assert!(summary.summary_text.is_null());
            assert_eq!(summary.confidence_score, 0.0);
        }
    }

    #[test]
    fn test_processor_lifecycle() {
        unsafe {
            // Create and destroy multiple times
            for _ in 0..5 {
                let processor = ai_processor_create();
                assert!(!processor.is_null());
                ai_processor_destroy(processor);
            }
        }
    }
}
