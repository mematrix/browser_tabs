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
