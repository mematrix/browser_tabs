#![allow(unused_imports)]

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_float, c_double};
use std::ptr;
use serde::{Deserialize, Serialize};

/// C-compatible AI processor interface
#[repr(C)]
pub struct CAIProcessor {
    _private: [u8; 0],
}

/// C-compatible page content for input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageContentInput {
    pub html: String,
    pub text: String,
    pub title: String,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    pub images: Vec<String>,
    pub links: Vec<String>,
}

/// C-compatible content summary
#[repr(C)]
pub struct CContentSummary {
    pub summary_text: *mut c_char,
    pub key_points: *mut *mut c_char,
    pub key_points_count: usize,
    pub content_type: c_int,
    pub language: *mut c_char,
    pub reading_time_minutes: u32,
    pub confidence_score: c_float,
}

/// C-compatible category info
#[repr(C)]
pub struct CCategoryInfo {
    pub primary_category: *mut c_char,
    pub secondary_categories: *mut *mut c_char,
    pub secondary_count: usize,
    pub confidence: c_float,
}

/// C-compatible processing mode
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum CProcessingMode {
    Basic = 0,
    Enhanced = 1,
    Auto = 2,
}

/// Content type enum matching C++ side
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CContentType {
    Article = 0,
    Video = 1,
    Documentation = 2,
    SocialMedia = 3,
    Shopping = 4,
    News = 5,
    Reference = 6,
    Other = 7,
}

/// Internal AI processor state
struct AIProcessorState {
    mode: CProcessingMode,
}

/// Create AI processor instance
#[no_mangle]
pub extern "C" fn ai_processor_create() -> *mut CAIProcessor {
    let state = Box::new(AIProcessorState {
        mode: CProcessingMode::Auto,
    });
    Box::into_raw(state) as *mut CAIProcessor
}

/// Destroy AI processor instance
#[no_mangle]
pub extern "C" fn ai_processor_destroy(processor: *mut CAIProcessor) {
    if !processor.is_null() {
        unsafe {
            let _ = Box::from_raw(processor as *mut AIProcessorState);
        }
    }
}


/// Generate content summary from page content JSON
#[no_mangle]
pub extern "C" fn ai_processor_generate_summary(
    processor: *mut CAIProcessor,
    content_json: *const c_char,
) -> CContentSummary {
    let empty_summary = CContentSummary {
        summary_text: ptr::null_mut(),
        key_points: ptr::null_mut(),
        key_points_count: 0,
        content_type: CContentType::Other as c_int,
        language: ptr::null_mut(),
        reading_time_minutes: 0,
        confidence_score: 0.0,
    };
    
    if processor.is_null() || content_json.is_null() {
        return empty_summary;
    }
    
    unsafe {
        let content_str = match CStr::from_ptr(content_json).to_str() {
            Ok(s) => s,
            Err(_) => return empty_summary,
        };
        
        // Parse the JSON content
        let content: PageContentInput = match serde_json::from_str(content_str) {
            Ok(c) => c,
            Err(_) => return empty_summary,
        };
        
        // Generate summary using extractive summarization
        let summary_text = generate_extractive_summary(&content.text, 3);
        let key_points = extract_key_points(&content.text, 5);
        let content_type = classify_content_type(&content);
        let language = detect_language(&content.text);
        let reading_time = estimate_reading_time(&content.text);
        
        // Calculate confidence score
        let mut confidence = 0.5f32;
        if !summary_text.is_empty() { confidence += 0.15; }
        if !key_points.is_empty() { confidence += 0.1; }
        if !content.title.is_empty() { confidence += 0.1; }
        if content.description.is_some() { confidence += 0.1; }
        if content.text.len() > 500 { confidence += 0.05; }
        confidence = confidence.min(0.95);
        
        // Convert to C strings
        let summary_c = CString::new(summary_text).unwrap_or_default();
        let language_c = CString::new(language).unwrap_or_default();
        
        // Convert key points to C string array
        let mut key_points_ptrs: Vec<*mut c_char> = key_points
            .into_iter()
            .filter_map(|kp| CString::new(kp).ok())
            .map(|cs| cs.into_raw())
            .collect();
        
        let key_points_count = key_points_ptrs.len();
        let key_points_ptr = if key_points_count > 0 {
            let ptr = key_points_ptrs.as_mut_ptr();
            std::mem::forget(key_points_ptrs);
            ptr
        } else {
            ptr::null_mut()
        };
        
        CContentSummary {
            summary_text: summary_c.into_raw(),
            key_points: key_points_ptr,
            key_points_count,
            content_type: content_type as c_int,
            language: language_c.into_raw(),
            reading_time_minutes: reading_time,
            confidence_score: confidence,
        }
    }
}

/// Extract keywords from content
#[no_mangle]
pub extern "C" fn ai_processor_extract_keywords(
    processor: *mut CAIProcessor,
    content_json: *const c_char,
    keywords_out: *mut *mut *mut c_char,
    count_out: *mut usize,
) -> c_int {
    if processor.is_null() || content_json.is_null() || keywords_out.is_null() || count_out.is_null() {
        return -1;
    }
    
    unsafe {
        let content_str = match CStr::from_ptr(content_json).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        };
        
        // Parse the JSON content
        let content: PageContentInput = match serde_json::from_str(content_str) {
            Ok(c) => c,
            Err(_) => return -1,
        };
        
        // Extract keywords
        let mut keywords = content.keywords.clone();
        let extracted = extract_keywords_from_text(&content.text, 15);
        
        // Merge keywords, avoiding duplicates
        for kw in extracted {
            if !keywords.contains(&kw) {
                keywords.push(kw);
            }
        }
        
        // Also extract from title
        let title_keywords = extract_keywords_from_text(&content.title, 5);
        for kw in title_keywords {
            if !keywords.contains(&kw) {
                keywords.insert(0, kw);
            }
        }
        
        // Limit to 20 keywords
        keywords.truncate(20);
        
        // Convert to C strings
        let c_keywords: Vec<*mut c_char> = keywords
            .into_iter()
            .filter_map(|kw| CString::new(kw).ok())
            .map(|cs| cs.into_raw())
            .collect();
        
        let count = c_keywords.len();
        
        if count > 0 {
            let keywords_array = c_keywords.into_boxed_slice();
            *keywords_out = Box::into_raw(keywords_array) as *mut *mut c_char;
            *count_out = count;
        } else {
            *keywords_out = ptr::null_mut();
            *count_out = 0;
        }
        
        0 // Success
    }
}

/// Classify content and return category info
#[no_mangle]
pub extern "C" fn ai_processor_classify_content(
    processor: *mut CAIProcessor,
    content_json: *const c_char,
) -> CCategoryInfo {
    let empty_category = CCategoryInfo {
        primary_category: ptr::null_mut(),
        secondary_categories: ptr::null_mut(),
        secondary_count: 0,
        confidence: 0.0,
    };
    
    if processor.is_null() || content_json.is_null() {
        return empty_category;
    }
    
    unsafe {
        let content_str = match CStr::from_ptr(content_json).to_str() {
            Ok(s) => s,
            Err(_) => return empty_category,
        };
        
        let content: PageContentInput = match serde_json::from_str(content_str) {
            Ok(c) => c,
            Err(_) => return empty_category,
        };
        
        let content_type = classify_content_type(&content);
        let (primary, secondary) = get_category_info(content_type);
        
        let primary_c = CString::new(primary).unwrap_or_default();
        
        let mut secondary_ptrs: Vec<*mut c_char> = secondary
            .into_iter()
            .filter_map(|s| CString::new(s).ok())
            .map(|cs| cs.into_raw())
            .collect();
        
        let secondary_count = secondary_ptrs.len();
        let secondary_ptr = if secondary_count > 0 {
            let ptr = secondary_ptrs.as_mut_ptr();
            std::mem::forget(secondary_ptrs);
            ptr
        } else {
            ptr::null_mut()
        };
        
        CCategoryInfo {
            primary_category: primary_c.into_raw(),
            secondary_categories: secondary_ptr,
            secondary_count,
            confidence: 0.75,
        }
    }
}


/// Calculate content similarity between two content JSONs
#[no_mangle]
pub extern "C" fn ai_processor_calculate_similarity(
    processor: *mut CAIProcessor,
    content_a_json: *const c_char,
    content_b_json: *const c_char,
) -> c_double {
    if processor.is_null() || content_a_json.is_null() || content_b_json.is_null() {
        return 0.0;
    }
    
    unsafe {
        let content_a_str = match CStr::from_ptr(content_a_json).to_str() {
            Ok(s) => s,
            Err(_) => return 0.0,
        };
        
        let content_b_str = match CStr::from_ptr(content_b_json).to_str() {
            Ok(s) => s,
            Err(_) => return 0.0,
        };
        
        let content_a: PageContentInput = match serde_json::from_str(content_a_str) {
            Ok(c) => c,
            Err(_) => return 0.0,
        };
        
        let content_b: PageContentInput = match serde_json::from_str(content_b_str) {
            Ok(c) => c,
            Err(_) => return 0.0,
        };
        
        // Calculate cosine similarity
        calculate_cosine_similarity(&content_a.text, &content_b.text)
    }
}

/// Set processing mode
#[no_mangle]
pub extern "C" fn ai_processor_set_mode(
    processor: *mut CAIProcessor,
    mode: CProcessingMode,
) -> c_int {
    if processor.is_null() {
        return -1;
    }
    
    unsafe {
        let state = &mut *(processor as *mut AIProcessorState);
        state.mode = mode;
    }
    
    0 // Success
}

/// Free C string
#[no_mangle]
pub extern "C" fn ai_processor_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

/// Free keyword array
#[no_mangle]
pub extern "C" fn ai_processor_free_keywords(keywords: *mut *mut c_char, count: usize) {
    if !keywords.is_null() && count > 0 {
        unsafe {
            let keywords_slice = std::slice::from_raw_parts_mut(keywords, count);
            for keyword in keywords_slice {
                if !keyword.is_null() {
                    let _ = CString::from_raw(*keyword);
                }
            }
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(keywords, count) as *mut [*mut c_char]);
        }
    }
}

/// Free content summary
#[no_mangle]
pub extern "C" fn ai_processor_free_summary(summary: CContentSummary) {
    if !summary.summary_text.is_null() {
        unsafe { let _ = CString::from_raw(summary.summary_text); }
    }
    if !summary.language.is_null() {
        unsafe { let _ = CString::from_raw(summary.language); }
    }
    if !summary.key_points.is_null() && summary.key_points_count > 0 {
        unsafe {
            let key_points_slice = std::slice::from_raw_parts_mut(summary.key_points, summary.key_points_count);
            for kp in key_points_slice {
                if !kp.is_null() {
                    let _ = CString::from_raw(*kp);
                }
            }
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(summary.key_points, summary.key_points_count) as *mut [*mut c_char]);
        }
    }
}

/// Free category info
#[no_mangle]
pub extern "C" fn ai_processor_free_category(category: CCategoryInfo) {
    if !category.primary_category.is_null() {
        unsafe { let _ = CString::from_raw(category.primary_category); }
    }
    if !category.secondary_categories.is_null() && category.secondary_count > 0 {
        unsafe {
            let secondary_slice = std::slice::from_raw_parts_mut(category.secondary_categories, category.secondary_count);
            for s in secondary_slice {
                if !s.is_null() {
                    let _ = CString::from_raw(*s);
                }
            }
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(category.secondary_categories, category.secondary_count) as *mut [*mut c_char]);
        }
    }
}

/// C-compatible page structure
#[repr(C)]
pub struct CPageStructure {
    pub heading_count: usize,
    pub paragraph_count: usize,
    pub list_count: usize,
    pub table_count: usize,
    pub form_count: usize,
    pub media_count: usize,
    pub headings: *mut *mut c_char,
    pub headings_count: usize,
    pub sections: *mut *mut c_char,
    pub sections_count: usize,
    pub has_navigation: c_int,
    pub has_sidebar: c_int,
    pub has_footer: c_int,
    pub content_density: c_float,
}

/// C-compatible entity info
#[repr(C)]
pub struct CEntityInfo {
    pub name: *mut c_char,
    pub entity_type: *mut c_char,
    pub confidence: c_float,
    pub positions: *mut usize,
    pub positions_count: usize,
}

/// C-compatible cross recommendation
#[repr(C)]
pub struct CCrossRecommendation {
    pub source_id: *mut c_char,
    pub target_id: *mut c_char,
    pub relevance_score: c_float,
    pub reason: *mut c_char,
    pub common_topics: *mut *mut c_char,
    pub common_topics_count: usize,
}

/// C-compatible group suggestion
#[repr(C)]
pub struct CGroupSuggestion {
    pub group_name: *mut c_char,
    pub description: *mut c_char,
    pub page_ids: *mut *mut c_char,
    pub page_ids_count: usize,
    pub similarity_score: c_float,
}

/// Analyze page structure from HTML content
#[no_mangle]
pub extern "C" fn ai_processor_analyze_page_structure(
    processor: *mut CAIProcessor,
    html: *const c_char,
) -> CPageStructure {
    let empty_structure = CPageStructure {
        heading_count: 0,
        paragraph_count: 0,
        list_count: 0,
        table_count: 0,
        form_count: 0,
        media_count: 0,
        headings: ptr::null_mut(),
        headings_count: 0,
        sections: ptr::null_mut(),
        sections_count: 0,
        has_navigation: 0,
        has_sidebar: 0,
        has_footer: 0,
        content_density: 0.0,
    };
    
    if processor.is_null() || html.is_null() {
        return empty_structure;
    }
    
    unsafe {
        let html_str = match CStr::from_ptr(html).to_str() {
            Ok(s) => s,
            Err(_) => return empty_structure,
        };
        
        let structure = analyze_page_structure_internal(html_str);
        structure
    }
}

/// Extract entities from text
#[no_mangle]
pub extern "C" fn ai_processor_extract_entities(
    processor: *mut CAIProcessor,
    text: *const c_char,
    entities_out: *mut *mut CEntityInfo,
    count_out: *mut usize,
) -> c_int {
    if processor.is_null() || text.is_null() || entities_out.is_null() || count_out.is_null() {
        return -1;
    }
    
    unsafe {
        let text_str = match CStr::from_ptr(text).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        };
        
        let entities = extract_entities_internal(text_str);
        
        if entities.is_empty() {
            *entities_out = ptr::null_mut();
            *count_out = 0;
            return 0;
        }
        
        let c_entities: Vec<CEntityInfo> = entities
            .into_iter()
            .map(|(name, entity_type, confidence, positions)| {
                let name_c = CString::new(name).unwrap_or_default();
                let type_c = CString::new(entity_type).unwrap_or_default();
                
                let positions_ptr = if !positions.is_empty() {
                    let mut pos_vec = positions.into_boxed_slice();
                    let ptr = pos_vec.as_mut_ptr();
                    std::mem::forget(pos_vec);
                    ptr
                } else {
                    ptr::null_mut()
                };
                
                CEntityInfo {
                    name: name_c.into_raw(),
                    entity_type: type_c.into_raw(),
                    confidence,
                    positions: positions_ptr,
                    positions_count: 0, // Will be set properly
                }
            })
            .collect();
        
        let count = c_entities.len();
        let entities_box = c_entities.into_boxed_slice();
        *entities_out = Box::into_raw(entities_box) as *mut CEntityInfo;
        *count_out = count;
        
        0
    }
}

/// Analyze sentiment of text
#[no_mangle]
pub extern "C" fn ai_processor_analyze_sentiment(
    processor: *mut CAIProcessor,
    text: *const c_char,
    label_out: *mut *mut c_char,
    score_out: *mut c_float,
) -> c_int {
    if processor.is_null() || text.is_null() || label_out.is_null() || score_out.is_null() {
        return -1;
    }
    
    unsafe {
        let text_str = match CStr::from_ptr(text).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        };
        
        let (label, score) = analyze_sentiment_internal(text_str);
        
        let label_c = CString::new(label).unwrap_or_default();
        *label_out = label_c.into_raw();
        *score_out = score;
        
        0
    }
}

/// Suggest groups from multiple page contents
#[no_mangle]
pub extern "C" fn ai_processor_suggest_groups(
    processor: *mut CAIProcessor,
    contents_json: *const c_char,
    similarity_threshold: c_double,
    suggestions_out: *mut *mut CGroupSuggestion,
    count_out: *mut usize,
) -> c_int {
    if processor.is_null() || contents_json.is_null() || suggestions_out.is_null() || count_out.is_null() {
        return -1;
    }
    
    unsafe {
        let contents_str = match CStr::from_ptr(contents_json).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        };
        
        let contents: Vec<PageContentInput> = match serde_json::from_str(contents_str) {
            Ok(c) => c,
            Err(_) => return -1,
        };
        
        let suggestions = suggest_groups_internal(&contents, similarity_threshold);
        
        if suggestions.is_empty() {
            *suggestions_out = ptr::null_mut();
            *count_out = 0;
            return 0;
        }
        
        let c_suggestions: Vec<CGroupSuggestion> = suggestions
            .into_iter()
            .map(|(name, description, page_ids, score)| {
                let name_c = CString::new(name).unwrap_or_default();
                let desc_c = CString::new(description).unwrap_or_default();
                
                let mut page_ids_ptrs: Vec<*mut c_char> = page_ids
                    .into_iter()
                    .filter_map(|id| CString::new(id).ok())
                    .map(|cs| cs.into_raw())
                    .collect();
                
                let page_ids_count = page_ids_ptrs.len();
                let page_ids_ptr = if page_ids_count > 0 {
                    let ptr = page_ids_ptrs.as_mut_ptr();
                    std::mem::forget(page_ids_ptrs);
                    ptr
                } else {
                    ptr::null_mut()
                };
                
                CGroupSuggestion {
                    group_name: name_c.into_raw(),
                    description: desc_c.into_raw(),
                    page_ids: page_ids_ptr,
                    page_ids_count,
                    similarity_score: score,
                }
            })
            .collect();
        
        let count = c_suggestions.len();
        let suggestions_box = c_suggestions.into_boxed_slice();
        *suggestions_out = Box::into_raw(suggestions_box) as *mut CGroupSuggestion;
        *count_out = count;
        
        0
    }
}

/// Generate cross-content recommendations
#[no_mangle]
pub extern "C" fn ai_processor_generate_cross_recommendations(
    processor: *mut CAIProcessor,
    contents_json: *const c_char,
    min_relevance: c_float,
    recommendations_out: *mut *mut CCrossRecommendation,
    count_out: *mut usize,
) -> c_int {
    if processor.is_null() || contents_json.is_null() || recommendations_out.is_null() || count_out.is_null() {
        return -1;
    }
    
    unsafe {
        let contents_str = match CStr::from_ptr(contents_json).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        };
        
        let contents: Vec<PageContentInput> = match serde_json::from_str(contents_str) {
            Ok(c) => c,
            Err(_) => return -1,
        };
        
        let recommendations = generate_cross_recommendations_internal(&contents, min_relevance);
        
        if recommendations.is_empty() {
            *recommendations_out = ptr::null_mut();
            *count_out = 0;
            return 0;
        }
        
        let c_recommendations: Vec<CCrossRecommendation> = recommendations
            .into_iter()
            .map(|(source_id, target_id, score, reason, common_topics)| {
                let source_c = CString::new(source_id).unwrap_or_default();
                let target_c = CString::new(target_id).unwrap_or_default();
                let reason_c = CString::new(reason).unwrap_or_default();
                
                let mut topics_ptrs: Vec<*mut c_char> = common_topics
                    .into_iter()
                    .filter_map(|t| CString::new(t).ok())
                    .map(|cs| cs.into_raw())
                    .collect();
                
                let topics_count = topics_ptrs.len();
                let topics_ptr = if topics_count > 0 {
                    let ptr = topics_ptrs.as_mut_ptr();
                    std::mem::forget(topics_ptrs);
                    ptr
                } else {
                    ptr::null_mut()
                };
                
                CCrossRecommendation {
                    source_id: source_c.into_raw(),
                    target_id: target_c.into_raw(),
                    relevance_score: score,
                    reason: reason_c.into_raw(),
                    common_topics: topics_ptr,
                    common_topics_count: topics_count,
                }
            })
            .collect();
        
        let count = c_recommendations.len();
        let recommendations_box = c_recommendations.into_boxed_slice();
        *recommendations_out = Box::into_raw(recommendations_box) as *mut CCrossRecommendation;
        *count_out = count;
        
        0
    }
}

/// Free page structure
#[no_mangle]
pub extern "C" fn ai_processor_free_page_structure(structure: CPageStructure) {
    if !structure.headings.is_null() && structure.headings_count > 0 {
        unsafe {
            let headings_slice = std::slice::from_raw_parts_mut(structure.headings, structure.headings_count);
            for h in headings_slice {
                if !h.is_null() {
                    let _ = CString::from_raw(*h);
                }
            }
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(structure.headings, structure.headings_count) as *mut [*mut c_char]);
        }
    }
    if !structure.sections.is_null() && structure.sections_count > 0 {
        unsafe {
            let sections_slice = std::slice::from_raw_parts_mut(structure.sections, structure.sections_count);
            for s in sections_slice {
                if !s.is_null() {
                    let _ = CString::from_raw(*s);
                }
            }
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(structure.sections, structure.sections_count) as *mut [*mut c_char]);
        }
    }
}

/// Free entity info array
#[no_mangle]
pub extern "C" fn ai_processor_free_entities(entities: *mut CEntityInfo, count: usize) {
    if !entities.is_null() && count > 0 {
        unsafe {
            let entities_slice = std::slice::from_raw_parts_mut(entities, count);
            for entity in entities_slice {
                if !entity.name.is_null() {
                    let _ = CString::from_raw(entity.name);
                }
                if !entity.entity_type.is_null() {
                    let _ = CString::from_raw(entity.entity_type);
                }
                if !entity.positions.is_null() && entity.positions_count > 0 {
                    let _ = Box::from_raw(std::slice::from_raw_parts_mut(entity.positions, entity.positions_count) as *mut [usize]);
                }
            }
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(entities, count) as *mut [CEntityInfo]);
        }
    }
}

/// Free group suggestion array
#[no_mangle]
pub extern "C" fn ai_processor_free_group_suggestions(suggestions: *mut CGroupSuggestion, count: usize) {
    if !suggestions.is_null() && count > 0 {
        unsafe {
            let suggestions_slice = std::slice::from_raw_parts_mut(suggestions, count);
            for suggestion in suggestions_slice {
                if !suggestion.group_name.is_null() {
                    let _ = CString::from_raw(suggestion.group_name);
                }
                if !suggestion.description.is_null() {
                    let _ = CString::from_raw(suggestion.description);
                }
                if !suggestion.page_ids.is_null() && suggestion.page_ids_count > 0 {
                    let page_ids_slice = std::slice::from_raw_parts_mut(suggestion.page_ids, suggestion.page_ids_count);
                    for id in page_ids_slice {
                        if !id.is_null() {
                            let _ = CString::from_raw(*id);
                        }
                    }
                    let _ = Box::from_raw(std::slice::from_raw_parts_mut(suggestion.page_ids, suggestion.page_ids_count) as *mut [*mut c_char]);
                }
            }
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(suggestions, count) as *mut [CGroupSuggestion]);
        }
    }
}

/// Free cross recommendation array
#[no_mangle]
pub extern "C" fn ai_processor_free_cross_recommendations(recommendations: *mut CCrossRecommendation, count: usize) {
    if !recommendations.is_null() && count > 0 {
        unsafe {
            let recommendations_slice = std::slice::from_raw_parts_mut(recommendations, count);
            for rec in recommendations_slice {
                if !rec.source_id.is_null() {
                    let _ = CString::from_raw(rec.source_id);
                }
                if !rec.target_id.is_null() {
                    let _ = CString::from_raw(rec.target_id);
                }
                if !rec.reason.is_null() {
                    let _ = CString::from_raw(rec.reason);
                }
                if !rec.common_topics.is_null() && rec.common_topics_count > 0 {
                    let topics_slice = std::slice::from_raw_parts_mut(rec.common_topics, rec.common_topics_count);
                    for topic in topics_slice {
                        if !topic.is_null() {
                            let _ = CString::from_raw(*topic);
                        }
                    }
                    let _ = Box::from_raw(std::slice::from_raw_parts_mut(rec.common_topics, rec.common_topics_count) as *mut [*mut c_char]);
                }
            }
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(recommendations, count) as *mut [CCrossRecommendation]);
        }
    }
}


// ============================================================================
// Internal helper functions for AI processing
// ============================================================================

/// Common stop words to filter out
const STOP_WORDS: &[&str] = &[
    "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
    "of", "with", "by", "from", "as", "is", "was", "are", "were", "been",
    "be", "have", "has", "had", "do", "does", "did", "will", "would", "could",
    "should", "may", "might", "must", "shall", "can", "need", "dare", "ought",
    "used", "this", "that", "these", "those", "i", "you", "he", "she", "it",
    "we", "they", "what", "which", "who", "whom", "whose", "where", "when",
    "why", "how", "all", "each", "every", "both", "few", "more", "most",
    "other", "some", "such", "no", "nor", "not", "only", "own", "same",
    "so", "than", "too", "very", "just", "also", "now", "here", "there",
];

/// Tokenize text into words, filtering stop words
fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut word = String::new();
    
    for c in text.chars() {
        if c.is_alphanumeric() {
            word.push(c.to_ascii_lowercase());
        } else if !word.is_empty() {
            if word.len() > 2 && !STOP_WORDS.contains(&word.as_str()) {
                tokens.push(word.clone());
            }
            word.clear();
        }
    }
    
    if !word.is_empty() && word.len() > 2 && !STOP_WORDS.contains(&word.as_str()) {
        tokens.push(word);
    }
    
    tokens
}

/// Split text into sentences
fn split_into_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    
    for c in text.chars() {
        current.push(c);
        
        if c == '.' || c == '!' || c == '?' {
            let trimmed = current.trim().to_string();
            if trimmed.len() > 10 {
                sentences.push(trimmed);
            }
            current.clear();
        }
    }
    
    if !current.is_empty() {
        let trimmed = current.trim().to_string();
        if trimmed.len() > 10 {
            sentences.push(trimmed);
        }
    }
    
    sentences
}

/// Calculate word frequency
fn calculate_word_frequency(tokens: &[String]) -> std::collections::HashMap<String, usize> {
    let mut freq = std::collections::HashMap::new();
    for token in tokens {
        *freq.entry(token.clone()).or_insert(0) += 1;
    }
    freq
}

/// Score a sentence based on word frequency
fn score_sentence(sentence: &str, word_freq: &std::collections::HashMap<String, usize>, max_freq: usize) -> f64 {
    let tokens = tokenize(sentence);
    if tokens.is_empty() {
        return 0.0;
    }
    
    let mut score = 0.0;
    for token in &tokens {
        if let Some(&freq) = word_freq.get(token) {
            score += freq as f64 / max_freq as f64;
        }
    }
    
    // Normalize by sentence length
    let length_factor = if tokens.len() < 5 {
        0.5
    } else if tokens.len() > 30 {
        0.7
    } else {
        1.0
    };
    
    (score / tokens.len() as f64) * length_factor
}

/// Generate extractive summary
fn generate_extractive_summary(text: &str, max_sentences: usize) -> String {
    if text.is_empty() {
        return String::new();
    }
    
    let sentences = split_into_sentences(text);
    if sentences.is_empty() {
        return if text.len() <= 200 {
            text.to_string()
        } else {
            format!("{}...", &text[..200])
        };
    }
    
    if sentences.len() <= max_sentences {
        return sentences.join(" ");
    }
    
    let all_tokens = tokenize(text);
    let word_freq = calculate_word_frequency(&all_tokens);
    let max_freq = word_freq.values().max().copied().unwrap_or(1);
    
    let mut scored: Vec<(f64, usize)> = sentences
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut score = score_sentence(s, &word_freq, max_freq);
            if i < 3 {
                score *= 1.2;
            }
            (score, i)
        })
        .collect();
    
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    
    let mut selected: Vec<usize> = scored.iter().take(max_sentences).map(|(_, i)| *i).collect();
    selected.sort();
    
    selected.iter().map(|&i| sentences[i].clone()).collect::<Vec<_>>().join(" ")
}

/// Extract key points from text
fn extract_key_points(text: &str, max_points: usize) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    
    let sentences = split_into_sentences(text);
    if sentences.is_empty() {
        return Vec::new();
    }
    
    let all_tokens = tokenize(text);
    let word_freq = calculate_word_frequency(&all_tokens);
    let max_freq = word_freq.values().max().copied().unwrap_or(1);
    
    let mut scored: Vec<(f64, String)> = sentences
        .iter()
        .map(|s| (score_sentence(s, &word_freq, max_freq), s.clone()))
        .collect();
    
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    
    scored
        .into_iter()
        .take(max_points)
        .map(|(_, s)| {
            if s.len() > 150 {
                format!("{}...", &s[..147])
            } else {
                s
            }
        })
        .collect()
}

/// Extract keywords from text
fn extract_keywords_from_text(text: &str, max_keywords: usize) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    
    let tokens = tokenize(text);
    let word_freq = calculate_word_frequency(&tokens);
    
    let mut sorted: Vec<(String, usize)> = word_freq.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    
    sorted
        .into_iter()
        .take(max_keywords)
        .filter(|(_, count)| *count > 1 || max_keywords > 10)
        .map(|(word, _)| word)
        .collect()
}

/// Classify content type
fn classify_content_type(content: &PageContentInput) -> CContentType {
    let lower_title = content.title.to_lowercase();
    let lower_text = content.text.to_lowercase();
    let sample_text = if lower_text.len() > 1000 {
        &lower_text[..1000]
    } else {
        &lower_text
    };
    
    // Check for video content
    if lower_title.contains("video") || lower_title.contains("watch") ||
       lower_title.contains("youtube") || lower_title.contains("vimeo") {
        return CContentType::Video;
    }
    
    // Check for documentation
    if lower_title.contains("documentation") || lower_title.contains("docs") ||
       lower_title.contains("api reference") || lower_title.contains("manual") ||
       (sample_text.contains("function") && sample_text.contains("parameter")) {
        return CContentType::Documentation;
    }
    
    // Check for social media
    if lower_title.contains("twitter") || lower_title.contains("facebook") ||
       lower_title.contains("instagram") || lower_title.contains("linkedin") ||
       lower_title.contains("reddit") {
        return CContentType::SocialMedia;
    }
    
    // Check for shopping
    if lower_title.contains("buy") || lower_title.contains("shop") ||
       lower_title.contains("cart") || lower_title.contains("price") ||
       sample_text.contains("add to cart") || sample_text.contains("checkout") {
        return CContentType::Shopping;
    }
    
    // Check for news
    if lower_title.contains("news") || lower_title.contains("breaking") ||
       lower_title.contains("headline") || sample_text.contains("reporter") {
        return CContentType::News;
    }
    
    // Check for reference
    if lower_title.contains("wikipedia") || lower_title.contains("encyclopedia") ||
       (sample_text.contains("references") && sample_text.contains("citation")) {
        return CContentType::Reference;
    }
    
    CContentType::Article
}

/// Get category info from content type
fn get_category_info(content_type: CContentType) -> (String, Vec<String>) {
    match content_type {
        CContentType::Article => ("Articles".to_string(), vec!["Reading".to_string(), "Information".to_string()]),
        CContentType::Video => ("Media".to_string(), vec!["Video".to_string(), "Entertainment".to_string()]),
        CContentType::Documentation => ("Documentation".to_string(), vec!["Reference".to_string(), "Technical".to_string()]),
        CContentType::SocialMedia => ("Social".to_string(), vec!["Social Media".to_string(), "Communication".to_string()]),
        CContentType::Shopping => ("Shopping".to_string(), vec!["E-commerce".to_string(), "Products".to_string()]),
        CContentType::News => ("News".to_string(), vec!["Current Events".to_string(), "Information".to_string()]),
        CContentType::Reference => ("Reference".to_string(), vec!["Knowledge".to_string(), "Information".to_string()]),
        CContentType::Other => ("Other".to_string(), vec![]),
    }
}

/// Detect language from text
fn detect_language(text: &str) -> String {
    let mut chinese_chars = 0;
    let mut latin_chars = 0;
    let mut cyrillic_chars = 0;
    
    for c in text.chars() {
        if c.is_ascii_alphabetic() {
            latin_chars += 1;
        } else if c >= '\u{4E00}' && c <= '\u{9FFF}' {
            chinese_chars += 1;
        } else if c >= '\u{0400}' && c <= '\u{04FF}' {
            cyrillic_chars += 1;
        }
    }
    
    let lower_text = text.to_lowercase();
    
    // Check for language-specific words
    let spanish_score = ["que", "para", "como", "pero"]
        .iter()
        .filter(|w| lower_text.contains(*w))
        .count();
    
    let french_score = ["que", "pour", "avec", "dans"]
        .iter()
        .filter(|w| lower_text.contains(*w))
        .count();
    
    let german_score = ["und", "der", "die", "das"]
        .iter()
        .filter(|w| lower_text.contains(*w))
        .count();
    
    if chinese_chars > latin_chars {
        return "zh".to_string();
    }
    
    if cyrillic_chars > latin_chars {
        return "ru".to_string();
    }
    
    if latin_chars > 0 {
        if german_score >= 3 { return "de".to_string(); }
        if french_score >= 3 { return "fr".to_string(); }
        if spanish_score >= 3 { return "es".to_string(); }
    }
    
    "en".to_string()
}

/// Estimate reading time in minutes
fn estimate_reading_time(text: &str) -> u32 {
    let lang = detect_language(text);
    
    if lang == "zh" || lang == "ja" || lang == "ko" {
        // Character-based languages: ~300 chars/minute
        let char_count = text.chars().filter(|c| !c.is_whitespace()).count();
        std::cmp::max(1, (char_count / 300) as u32)
    } else {
        // Word-based languages: ~200 words/minute
        let word_count = text.split_whitespace().count();
        std::cmp::max(1, (word_count / 200) as u32)
    }
}

/// Calculate cosine similarity between two texts
fn calculate_cosine_similarity(text_a: &str, text_b: &str) -> f64 {
    let tokens_a = tokenize(text_a);
    let tokens_b = tokenize(text_b);
    
    if tokens_a.is_empty() || tokens_b.is_empty() {
        return 0.0;
    }
    
    let freq_a = calculate_word_frequency(&tokens_a);
    let freq_b = calculate_word_frequency(&tokens_b);
    
    // Collect all unique terms
    let mut all_terms: std::collections::HashSet<&String> = std::collections::HashSet::new();
    for term in freq_a.keys() {
        all_terms.insert(term);
    }
    for term in freq_b.keys() {
        all_terms.insert(term);
    }
    
    // Calculate dot product and magnitudes
    let mut dot_product = 0.0;
    let mut magnitude_a = 0.0;
    let mut magnitude_b = 0.0;
    
    let len_a = tokens_a.len() as f64;
    let len_b = tokens_b.len() as f64;
    
    for term in all_terms {
        let val_a = freq_a.get(term).map(|&v| v as f64 / len_a).unwrap_or(0.0);
        let val_b = freq_b.get(term).map(|&v| v as f64 / len_b).unwrap_or(0.0);
        
        dot_product += val_a * val_b;
        magnitude_a += val_a * val_a;
        magnitude_b += val_b * val_b;
    }
    
    magnitude_a = magnitude_a.sqrt();
    magnitude_b = magnitude_b.sqrt();
    
    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }
    
    dot_product / (magnitude_a * magnitude_b)
}

/// Analyze page structure from HTML
fn analyze_page_structure_internal(html: &str) -> CPageStructure {
    use regex::Regex;
    
    let mut structure = CPageStructure {
        heading_count: 0,
        paragraph_count: 0,
        list_count: 0,
        table_count: 0,
        form_count: 0,
        media_count: 0,
        headings: ptr::null_mut(),
        headings_count: 0,
        sections: ptr::null_mut(),
        sections_count: 0,
        has_navigation: 0,
        has_sidebar: 0,
        has_footer: 0,
        content_density: 0.0,
    };
    
    if html.is_empty() {
        return structure;
    }
    
    // Count headings
    for i in 1..=6 {
        let pattern = format!(r"(?i)<h{}", i);
        if let Ok(re) = Regex::new(&pattern) {
            structure.heading_count += re.find_iter(html).count();
        }
    }
    
    // Extract heading texts
    let mut headings = Vec::new();
    if let Ok(re) = Regex::new(r"(?i)<h[1-6][^>]*>([^<]*)</h[1-6]>") {
        for cap in re.captures_iter(html) {
            if let Some(text) = cap.get(1) {
                let heading = text.as_str().trim().to_string();
                if !heading.is_empty() {
                    headings.push(heading);
                }
            }
        }
    }
    
    // Count paragraphs
    if let Ok(re) = Regex::new(r"(?i)<p[^>]*>") {
        structure.paragraph_count = re.find_iter(html).count();
    }
    
    // Count lists
    if let Ok(re) = Regex::new(r"(?i)<(ul|ol)[^>]*>") {
        structure.list_count = re.find_iter(html).count();
    }
    
    // Count tables
    if let Ok(re) = Regex::new(r"(?i)<table[^>]*>") {
        structure.table_count = re.find_iter(html).count();
    }
    
    // Count forms
    if let Ok(re) = Regex::new(r"(?i)<form[^>]*>") {
        structure.form_count = re.find_iter(html).count();
    }
    
    // Count media elements
    if let Ok(re) = Regex::new(r"(?i)<(img|video|audio)[^>]*>") {
        structure.media_count = re.find_iter(html).count();
    }
    
    // Check for navigation
    if let Ok(re) = Regex::new(r#"(?i)<nav[^>]*>|class=["'][^"']*nav[^"']*["']"#) {
        structure.has_navigation = if re.is_match(html) { 1 } else { 0 };
    }
    
    // Check for sidebar
    if let Ok(re) = Regex::new(r#"(?i)class=["'][^"']*sidebar[^"']*["']|<aside[^>]*>"#) {
        structure.has_sidebar = if re.is_match(html) { 1 } else { 0 };
    }
    
    // Check for footer
    if let Ok(re) = Regex::new(r#"(?i)<footer[^>]*>|class=["'][^"']*footer[^"']*["']"#) {
        structure.has_footer = if re.is_match(html) { 1 } else { 0 };
    }
    
    // Calculate content density
    let text = strip_html_tags(html);
    if !html.is_empty() {
        structure.content_density = text.len() as f32 / html.len() as f32;
    }
    
    // Convert headings to C strings
    if !headings.is_empty() {
        let mut headings_ptrs: Vec<*mut c_char> = headings
            .iter()
            .filter_map(|h| CString::new(h.as_str()).ok())
            .map(|cs| cs.into_raw())
            .collect();
        
        structure.headings_count = headings_ptrs.len();
        if structure.headings_count > 0 {
            let ptr = headings_ptrs.as_mut_ptr();
            std::mem::forget(headings_ptrs);
            structure.headings = ptr;
        }
        
        // Use headings as sections
        let mut sections_ptrs: Vec<*mut c_char> = headings
            .into_iter()
            .filter_map(|h| CString::new(h).ok())
            .map(|cs| cs.into_raw())
            .collect();
        
        structure.sections_count = sections_ptrs.len();
        if structure.sections_count > 0 {
            let ptr = sections_ptrs.as_mut_ptr();
            std::mem::forget(sections_ptrs);
            structure.sections = ptr;
        }
    }
    
    structure
}

/// Strip HTML tags from text
fn strip_html_tags(html: &str) -> String {
    use regex::Regex;
    
    let mut result = html.to_string();
    
    // Remove script and style tags with content
    if let Ok(re) = Regex::new(r"(?is)<script[^>]*>.*?</script>") {
        result = re.replace_all(&result, " ").to_string();
    }
    if let Ok(re) = Regex::new(r"(?is)<style[^>]*>.*?</style>") {
        result = re.replace_all(&result, " ").to_string();
    }
    
    // Remove all HTML tags
    if let Ok(re) = Regex::new(r"<[^>]*>") {
        result = re.replace_all(&result, " ").to_string();
    }
    
    // Normalize whitespace
    if let Ok(re) = Regex::new(r"\s+") {
        result = re.replace_all(&result, " ").to_string();
    }
    
    result.trim().to_string()
}

/// Extract entities from text
fn extract_entities_internal(text: &str) -> Vec<(String, String, f32, Vec<usize>)> {
    use regex::Regex;
    use std::collections::HashMap;
    
    let mut entities: HashMap<String, (String, f32, Vec<usize>)> = HashMap::new();
    
    // Extract potential person names (capitalized word sequences)
    if let Ok(re) = Regex::new(r"\b([A-Z][a-z]+(?:\s+[A-Z][a-z]+)+)\b") {
        for cap in re.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                let name = m.as_str().to_string();
                let pos = m.start();
                
                entities.entry(name.clone())
                    .and_modify(|(_, conf, positions)| {
                        *conf = (*conf + 0.1).min(0.95);
                        positions.push(pos);
                    })
                    .or_insert(("person".to_string(), 0.6, vec![pos]));
            }
        }
    }
    
    // Extract potential organizations
    if let Ok(re) = Regex::new(r"\b([A-Z][A-Za-z]*(?:\s+[A-Z][A-Za-z]*)*\s+(?:Inc|Corp|Ltd|LLC|Company|Corporation|Foundation|Institute|University))\b") {
        for cap in re.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                let org = m.as_str().to_string();
                let pos = m.start();
                
                entities.entry(org.clone())
                    .and_modify(|(entity_type, conf, positions)| {
                        *entity_type = "organization".to_string();
                        *conf = (*conf + 0.1).min(0.95);
                        positions.push(pos);
                    })
                    .or_insert(("organization".to_string(), 0.75, vec![pos]));
            }
        }
    }
    
    // Extract URLs as website entities
    if let Ok(re) = Regex::new(r"https?://([a-zA-Z0-9.-]+)") {
        for cap in re.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                let domain = m.as_str().to_string();
                let pos = m.start();
                
                entities.entry(domain.clone())
                    .or_insert(("website".to_string(), 0.9, vec![pos]));
            }
        }
    }
    
    // Convert to vector and sort by confidence
    let mut result: Vec<(String, String, f32, Vec<usize>)> = entities
        .into_iter()
        .map(|(name, (entity_type, conf, positions))| (name, entity_type, conf, positions))
        .collect();
    
    result.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    
    result
}

/// Analyze sentiment of text
fn analyze_sentiment_internal(text: &str) -> (String, f32) {
    const POSITIVE_WORDS: &[&str] = &[
        "good", "great", "excellent", "amazing", "wonderful", "fantastic",
        "awesome", "best", "love", "happy", "beautiful", "perfect",
        "brilliant", "outstanding", "superb", "incredible", "positive",
        "success", "successful", "win", "winner", "benefit", "helpful",
        "easy", "simple", "fast", "efficient", "effective", "recommend",
        "like", "enjoy", "pleased", "satisfied", "impressive", "innovative"
    ];
    
    const NEGATIVE_WORDS: &[&str] = &[
        "bad", "terrible", "awful", "horrible", "worst", "hate", "poor",
        "disappointing", "disappointed", "fail", "failure", "problem",
        "issue", "bug", "error", "wrong", "broken", "slow", "difficult",
        "hard", "complicated", "confusing", "frustrating", "annoying",
        "useless", "waste", "expensive", "overpriced", "scam", "fake",
        "never", "cannot", "impossible", "unfortunately", "sadly"
    ];
    
    let tokens = tokenize(text);
    
    let positive_count = tokens.iter()
        .filter(|t| POSITIVE_WORDS.contains(&t.as_str()))
        .count();
    
    let negative_count = tokens.iter()
        .filter(|t| NEGATIVE_WORDS.contains(&t.as_str()))
        .count();
    
    let total = positive_count + negative_count;
    let score = if total > 0 {
        (positive_count as f32 - negative_count as f32) / total as f32
    } else {
        0.0
    };
    
    let label = if score > 0.3 {
        "positive"
    } else if score < -0.3 {
        "negative"
    } else {
        "neutral"
    };
    
    (label.to_string(), score)
}

/// Suggest groups from page contents
fn suggest_groups_internal(contents: &[PageContentInput], similarity_threshold: f64) -> Vec<(String, String, Vec<String>, f32)> {
    if contents.is_empty() {
        return Vec::new();
    }
    
    let mut suggestions = Vec::new();
    let mut assigned = vec![false; contents.len()];
    
    for i in 0..contents.len() {
        if assigned[i] {
            continue;
        }
        
        let mut group_indices = vec![i];
        assigned[i] = true;
        
        for j in (i + 1)..contents.len() {
            if assigned[j] {
                continue;
            }
            
            let similarity = calculate_cosine_similarity(&contents[i].text, &contents[j].text);
            
            if similarity >= similarity_threshold {
                group_indices.push(j);
                assigned[j] = true;
            }
        }
        
        // Only create group if more than one page
        if group_indices.len() > 1 {
            // Find common words for group name
            let texts: Vec<&str> = group_indices.iter()
                .map(|&idx| contents[idx].text.as_str())
                .collect();
            
            let common_words = find_common_words(&texts, 3);
            
            let group_name = if !common_words.is_empty() {
                common_words.join(" & ")
            } else {
                format!("Group {}", suggestions.len() + 1)
            };
            
            let description = format!("A collection of {} related pages", group_indices.len());
            let page_ids: Vec<String> = group_indices.iter().map(|&idx| idx.to_string()).collect();
            
            suggestions.push((group_name, description, page_ids, similarity_threshold as f32));
        }
    }
    
    suggestions
}

/// Find common words across multiple texts
fn find_common_words(texts: &[&str], max_words: usize) -> Vec<String> {
    use std::collections::HashMap;
    
    let mut word_counts: HashMap<String, usize> = HashMap::new();
    
    for text in texts {
        let mut seen_in_doc: std::collections::HashSet<String> = std::collections::HashSet::new();
        let tokens = tokenize(text);
        
        for token in tokens {
            if token.len() > 3 && !seen_in_doc.contains(&token) {
                *word_counts.entry(token.clone()).or_insert(0) += 1;
                seen_in_doc.insert(token);
            }
        }
    }
    
    let mut sorted: Vec<(String, usize)> = word_counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    
    sorted.into_iter()
        .take(max_words)
        .map(|(word, _)| word)
        .collect()
}

/// Generate cross-content recommendations
fn generate_cross_recommendations_internal(contents: &[PageContentInput], min_relevance: f32) -> Vec<(String, String, f32, String, Vec<String>)> {
    if contents.len() < 2 {
        return Vec::new();
    }
    
    let mut recommendations = Vec::new();
    
    for i in 0..contents.len() {
        for j in (i + 1)..contents.len() {
            // Calculate content similarity
            let text_sim = calculate_cosine_similarity(&contents[i].text, &contents[j].text);
            
            // Calculate keyword overlap
            let keyword_sim = calculate_jaccard_similarity(&contents[i].keywords, &contents[j].keywords);
            
            // Combined relevance score
            let relevance = (0.6 * text_sim + 0.4 * keyword_sim) as f32;
            
            if relevance >= min_relevance {
                // Find common keywords
                let common_topics: Vec<String> = contents[i].keywords.iter()
                    .filter(|kw| contents[j].keywords.contains(kw))
                    .cloned()
                    .collect();
                
                // Generate reason
                let reason = if !common_topics.is_empty() {
                    let topic_str = if common_topics.len() > 1 {
                        format!("{} and {} more topics", common_topics[0], common_topics.len() - 1)
                    } else {
                        common_topics[0].clone()
                    };
                    format!("Both pages discuss: {}", topic_str)
                } else if relevance > 0.7 {
                    "Highly similar content".to_string()
                } else {
                    "Related content".to_string()
                };
                
                recommendations.push((
                    i.to_string(),
                    j.to_string(),
                    relevance,
                    reason,
                    common_topics,
                ));
            }
        }
    }
    
    // Sort by relevance
    recommendations.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    
    recommendations
}

/// Calculate Jaccard similarity between two keyword sets
fn calculate_jaccard_similarity(keywords_a: &[String], keywords_b: &[String]) -> f64 {
    if keywords_a.is_empty() && keywords_b.is_empty() {
        return 1.0;
    }
    
    if keywords_a.is_empty() || keywords_b.is_empty() {
        return 0.0;
    }
    
    let set_a: std::collections::HashSet<&String> = keywords_a.iter().collect();
    let set_b: std::collections::HashSet<&String> = keywords_b.iter().collect();
    
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    
    if union == 0 {
        return 0.0;
    }
    
    intersection as f64 / union as f64
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_basic() {
        let tokens = tokenize("Hello world, this is a test.");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
        // Stop words should be filtered
        assert!(!tokens.contains(&"this".to_string()));
        assert!(!tokens.contains(&"is".to_string()));
    }

    #[test]
    fn test_tokenize_filters_short_words() {
        let tokens = tokenize("I am a big cat");
        // "I", "am", "a" should be filtered (too short or stop words)
        assert!(!tokens.contains(&"i".to_string()));
        assert!(!tokens.contains(&"am".to_string()));
        assert!(tokens.contains(&"big".to_string()));
        assert!(tokens.contains(&"cat".to_string()));
    }

    #[test]
    fn test_split_into_sentences() {
        let text = "This is the first sentence. This is the second one! And a third?";
        let sentences = split_into_sentences(text);
        assert_eq!(sentences.len(), 3);
        assert!(sentences[0].contains("first"));
        assert!(sentences[1].contains("second"));
        assert!(sentences[2].contains("third"));
    }

    #[test]
    fn test_calculate_word_frequency() {
        let tokens = vec!["hello".to_string(), "world".to_string(), "hello".to_string()];
        let freq = calculate_word_frequency(&tokens);
        assert_eq!(freq.get("hello"), Some(&2));
        assert_eq!(freq.get("world"), Some(&1));
    }

    #[test]
    fn test_generate_extractive_summary_short_text() {
        let text = "Short text.";
        let summary = generate_extractive_summary(text, 3);
        assert_eq!(summary, "Short text.");
    }

    #[test]
    fn test_generate_extractive_summary_long_text() {
        let text = "The quick brown fox jumps over the lazy dog. \
                    This is a longer sentence that contains more information. \
                    Another sentence here with different content. \
                    Yet another sentence to make the text longer. \
                    Final sentence in this paragraph.";
        let summary = generate_extractive_summary(text, 2);
        // Should return 2 sentences
        let sentence_count = summary.matches('.').count();
        assert!(sentence_count <= 2);
    }

    #[test]
    fn test_extract_key_points() {
        let text = "Machine learning is a subset of artificial intelligence. \
                    Deep learning uses neural networks with many layers. \
                    Natural language processing helps computers understand text.";
        let key_points = extract_key_points(text, 2);
        assert!(key_points.len() <= 2);
    }

    #[test]
    fn test_extract_keywords_from_text() {
        let text = "Rust programming language is fast and safe. \
                    Rust provides memory safety without garbage collection. \
                    Programming in Rust is enjoyable.";
        let keywords = extract_keywords_from_text(text, 5);
        assert!(keywords.contains(&"rust".to_string()));
        assert!(keywords.contains(&"programming".to_string()));
    }

    #[test]
    fn test_classify_content_type_video() {
        let content = PageContentInput {
            html: String::new(),
            text: "Watch this amazing video".to_string(),
            title: "YouTube - Amazing Video".to_string(),
            description: None,
            keywords: vec![],
            images: vec![],
            links: vec![],
        };
        assert_eq!(classify_content_type(&content), CContentType::Video);
    }

    #[test]
    fn test_classify_content_type_documentation() {
        let content = PageContentInput {
            html: String::new(),
            text: "This function takes a parameter and returns a value".to_string(),
            title: "API Documentation".to_string(),
            description: None,
            keywords: vec![],
            images: vec![],
            links: vec![],
        };
        assert_eq!(classify_content_type(&content), CContentType::Documentation);
    }

    #[test]
    fn test_classify_content_type_shopping() {
        let content = PageContentInput {
            html: String::new(),
            text: "Add to cart and checkout now".to_string(),
            title: "Buy Now - Great Deals".to_string(),
            description: None,
            keywords: vec![],
            images: vec![],
            links: vec![],
        };
        assert_eq!(classify_content_type(&content), CContentType::Shopping);
    }

    #[test]
    fn test_classify_content_type_news() {
        let content = PageContentInput {
            html: String::new(),
            text: "The reporter covered the story".to_string(),
            title: "Breaking News Today".to_string(),
            description: None,
            keywords: vec![],
            images: vec![],
            links: vec![],
        };
        assert_eq!(classify_content_type(&content), CContentType::News);
    }

    #[test]
    fn test_detect_language_english() {
        let text = "This is a sample English text for testing language detection.";
        assert_eq!(detect_language(text), "en");
    }

    #[test]
    fn test_detect_language_chinese() {
        let text = "这是一段中文文本用于测试语言检测功能";
        assert_eq!(detect_language(text), "zh");
    }

    #[test]
    fn test_estimate_reading_time_short() {
        let text = "Short text.";
        assert_eq!(estimate_reading_time(text), 1); // Minimum 1 minute
    }

    #[test]
    fn test_estimate_reading_time_long() {
        // 400 words should be about 2 minutes
        let words: Vec<&str> = (0..400).map(|_| "word").collect();
        let text = words.join(" ");
        assert_eq!(estimate_reading_time(&text), 2);
    }

    #[test]
    fn test_calculate_cosine_similarity_identical() {
        let text = "The quick brown fox jumps over the lazy dog";
        let similarity = calculate_cosine_similarity(text, text);
        assert!((similarity - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_cosine_similarity_different() {
        let text_a = "Machine learning artificial intelligence";
        let text_b = "Cooking recipes food preparation";
        let similarity = calculate_cosine_similarity(text_a, text_b);
        assert!(similarity < 0.5);
    }

    #[test]
    fn test_calculate_cosine_similarity_similar() {
        let text_a = "Machine learning is a subset of artificial intelligence";
        let text_b = "Artificial intelligence includes machine learning techniques";
        let similarity = calculate_cosine_similarity(text_a, text_b);
        assert!(similarity > 0.3);
    }

    #[test]
    fn test_get_category_info() {
        let (primary, secondary) = get_category_info(CContentType::Article);
        assert_eq!(primary, "Articles");
        assert!(secondary.contains(&"Reading".to_string()));
    }

    #[test]
    fn test_ai_processor_create_destroy() {
        let processor = ai_processor_create();
        assert!(!processor.is_null());
        ai_processor_destroy(processor);
    }

    #[test]
    fn test_ai_processor_set_mode() {
        let processor = ai_processor_create();
        let result = ai_processor_set_mode(processor, CProcessingMode::Enhanced);
        assert_eq!(result, 0);
        ai_processor_destroy(processor);
    }

    #[test]
    fn test_ai_processor_generate_summary() {
        let processor = ai_processor_create();
        
        let content = PageContentInput {
            html: String::new(),
            text: "This is a test article about programming. \
                   Programming is the process of creating software. \
                   Software development requires careful planning.".to_string(),
            title: "Programming Article".to_string(),
            description: Some("An article about programming".to_string()),
            keywords: vec!["programming".to_string()],
            images: vec![],
            links: vec![],
        };
        
        let content_json = serde_json::to_string(&content).unwrap();
        let content_c = CString::new(content_json).unwrap();
        
        let summary = ai_processor_generate_summary(processor, content_c.as_ptr());
        
        assert!(!summary.summary_text.is_null());
        assert!(summary.confidence_score > 0.0);
        assert!(summary.reading_time_minutes >= 1);
        
        ai_processor_free_summary(summary);
        ai_processor_destroy(processor);
    }

    #[test]
    fn test_ai_processor_calculate_similarity() {
        let processor = ai_processor_create();
        
        let content_a = PageContentInput {
            html: String::new(),
            text: "Machine learning artificial intelligence neural networks".to_string(),
            title: "AI Article".to_string(),
            description: None,
            keywords: vec![],
            images: vec![],
            links: vec![],
        };
        
        let content_b = PageContentInput {
            html: String::new(),
            text: "Deep learning neural networks machine learning algorithms".to_string(),
            title: "ML Article".to_string(),
            description: None,
            keywords: vec![],
            images: vec![],
            links: vec![],
        };
        
        let json_a = CString::new(serde_json::to_string(&content_a).unwrap()).unwrap();
        let json_b = CString::new(serde_json::to_string(&content_b).unwrap()).unwrap();
        
        let similarity = ai_processor_calculate_similarity(processor, json_a.as_ptr(), json_b.as_ptr());
        
        assert!(similarity > 0.0);
        assert!(similarity <= 1.0);
        
        ai_processor_destroy(processor);
    }
}
