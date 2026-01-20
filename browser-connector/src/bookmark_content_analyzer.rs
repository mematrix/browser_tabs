//! Bookmark Content Analyzer Module for Web Page Manager
//!
//! This module provides functionality to fetch, validate, and analyze bookmark content,
//! including web page content extraction, accessibility verification, and metadata extraction.
//!
//! # Features
//! - Web page content fetching with configurable timeouts
//! - Bookmark accessibility validation
//! - Page metadata extraction (title, description, author, etc.)
//! - Batch processing support for multiple bookmarks
//!
//! # Requirements
//! - Requirement 2.2: Validate bookmark accessibility and generate status reports
//! - Requirement 2.3: Generate page content summaries, keyword tags, and classification suggestions

use web_page_manager_core::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::time::Instant;

/// Configuration for the bookmark content analyzer
#[derive(Debug, Clone)]
pub struct BookmarkContentAnalyzerConfig {
    /// Timeout for HTTP requests in seconds
    pub request_timeout_secs: u64,
    /// Maximum number of concurrent requests
    pub max_concurrent_requests: usize,
    /// Maximum content size to fetch in bytes
    pub max_content_size: usize,
    /// User agent string for HTTP requests
    pub user_agent: String,
    /// Whether to follow redirects
    pub follow_redirects: bool,
    /// Maximum number of redirects to follow
    pub max_redirects: usize,
}

impl Default for BookmarkContentAnalyzerConfig {
    fn default() -> Self {
        Self {
            request_timeout_secs: 15,
            max_concurrent_requests: 10,
            max_content_size: 5 * 1024 * 1024, // 5MB
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
            follow_redirects: true,
            max_redirects: 5,
        }
    }
}

/// Result of fetching bookmark content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkContentResult {
    pub bookmark: BookmarkInfo,
    pub status: AccessibilityStatus,
    pub content: Option<PageContent>,
    pub metadata: Option<PageMetadata>,
    pub response_time_ms: u64,
    pub final_url: Option<String>,
    pub fetched_at: DateTime<Utc>,
}

/// Batch analysis result for multiple bookmarks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchAnalysisResult {
    pub total_bookmarks: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<BookmarkContentResult>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub total_duration_ms: u64,
}

/// Bookmark content analyzer for fetching and validating bookmark content
///
/// Implements Requirements 2.2 and 2.3:
/// - Validate bookmark accessibility
/// - Extract page content and metadata
pub struct BookmarkContentAnalyzer {
    client: reqwest::Client,
    config: BookmarkContentAnalyzerConfig,
}

impl BookmarkContentAnalyzer {
    /// Create a new bookmark content analyzer with default configuration
    pub fn new() -> Self {
        Self::with_config(BookmarkContentAnalyzerConfig::default())
    }

    /// Create a new bookmark content analyzer with custom configuration
    pub fn with_config(config: BookmarkContentAnalyzerConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.request_timeout_secs))
            .redirect(if config.follow_redirects {
                reqwest::redirect::Policy::limited(config.max_redirects)
            } else {
                reqwest::redirect::Policy::none()
            })
            .user_agent(&config.user_agent)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { client, config }
    }

    /// Get the current configuration
    pub fn config(&self) -> &BookmarkContentAnalyzerConfig {
        &self.config
    }

    /// Fetch content for a single bookmark
    ///
    /// This method fetches the web page content, validates accessibility,
    /// and extracts metadata from the page.
    pub async fn fetch_bookmark_content(&self, bookmark: &BookmarkInfo) -> BookmarkContentResult {
        let start = Instant::now();
        let fetched_at = Utc::now();

        // Validate URL scheme
        if !Self::is_valid_url(&bookmark.url) {
            return BookmarkContentResult {
                bookmark: bookmark.clone(),
                status: AccessibilityStatus::NetworkError("Invalid URL scheme".to_string()),
                content: None,
                metadata: None,
                response_time_ms: start.elapsed().as_millis() as u64,
                final_url: None,
                fetched_at,
            };
        }

        // Fetch the page content
        match self.fetch_page(&bookmark.url).await {
            Ok((status, content, final_url)) => {
                let metadata = content.as_ref().map(|c| self.extract_metadata(c));
                
                BookmarkContentResult {
                    bookmark: bookmark.clone(),
                    status,
                    content,
                    metadata,
                    response_time_ms: start.elapsed().as_millis() as u64,
                    final_url,
                    fetched_at,
                }
            }
            Err(status) => {
                BookmarkContentResult {
                    bookmark: bookmark.clone(),
                    status,
                    content: None,
                    metadata: None,
                    response_time_ms: start.elapsed().as_millis() as u64,
                    final_url: None,
                    fetched_at,
                }
            }
        }
    }

    /// Validate bookmark accessibility without fetching full content
    ///
    /// This is a lightweight check that only performs a HEAD request
    /// to verify the URL is accessible.
    pub async fn validate_accessibility(&self, url: &str) -> (AccessibilityStatus, Option<String>) {
        if !Self::is_valid_url(url) {
            return (AccessibilityStatus::NetworkError("Invalid URL scheme".to_string()), None);
        }

        match self.client.head(url).send().await {
            Ok(response) => {
                let final_url = response.url().to_string();
                let redirect_url = if final_url != url {
                    Some(final_url)
                } else {
                    None
                };

                let status = Self::status_code_to_accessibility(response.status().as_u16());
                (status, redirect_url)
            }
            Err(e) => (Self::error_to_accessibility(&e), None),
        }
    }

    /// Extract page metadata from content
    ///
    /// Extracts various metadata fields from the HTML content including:
    /// - Title
    /// - Description (meta description or og:description)
    /// - Author
    /// - Published/Modified dates
    /// - Language
    /// - Open Graph image
    /// - Canonical URL
    /// - Site name
    pub fn extract_metadata(&self, content: &PageContent) -> PageMetadata {
        let html = &content.html;
        
        PageMetadata {
            title: Self::extract_title(html).unwrap_or_else(|| content.title.clone()),
            description: Self::extract_meta_content(html, "description")
                .or_else(|| Self::extract_og_content(html, "description"))
                .or_else(|| content.description.clone()),
            author: Self::extract_meta_content(html, "author")
                .or_else(|| Self::extract_meta_content(html, "article:author")),
            published_date: Self::extract_date(html, "article:published_time")
                .or_else(|| Self::extract_date(html, "datePublished")),
            modified_date: Self::extract_date(html, "article:modified_time")
                .or_else(|| Self::extract_date(html, "dateModified")),
            language: Self::extract_language(html),
            og_image: Self::extract_og_content(html, "image"),
            canonical_url: Self::extract_canonical_url(html),
            site_name: Self::extract_og_content(html, "site_name"),
        }
    }

    /// Fetch content for multiple bookmarks in batch
    ///
    /// This method processes bookmarks concurrently up to the configured
    /// maximum concurrent requests limit.
    pub async fn fetch_batch(&self, bookmarks: &[BookmarkInfo]) -> BatchAnalysisResult {
        use futures_util::stream::{self, StreamExt};

        let started_at = Utc::now();
        let start = Instant::now();

        let results: Vec<BookmarkContentResult> = stream::iter(bookmarks)
            .map(|bookmark| self.fetch_bookmark_content(bookmark))
            .buffer_unordered(self.config.max_concurrent_requests)
            .collect()
            .await;

        let successful = results.iter()
            .filter(|r| matches!(r.status, AccessibilityStatus::Accessible))
            .count();
        let failed = results.len() - successful;

        BatchAnalysisResult {
            total_bookmarks: bookmarks.len(),
            successful,
            failed,
            results,
            started_at,
            completed_at: Utc::now(),
            total_duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// Check if a URL is valid for fetching
    fn is_valid_url(url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }

    /// Fetch a page and return its content
    async fn fetch_page(&self, url: &str) -> std::result::Result<(AccessibilityStatus, Option<PageContent>, Option<String>), AccessibilityStatus> {
        let response = self.client.get(url).send().await
            .map_err(|e| Self::error_to_accessibility(&e))?;

        let final_url = response.url().to_string();
        let redirect_url = if final_url != url {
            Some(final_url)
        } else {
            None
        };

        let status_code = response.status().as_u16();
        let accessibility = Self::status_code_to_accessibility(status_code);

        if !matches!(accessibility, AccessibilityStatus::Accessible) {
            return Ok((accessibility, None, redirect_url));
        }

        // Check content length
        if let Some(content_length) = response.content_length() {
            if content_length > self.config.max_content_size as u64 {
                return Ok((
                    AccessibilityStatus::NetworkError("Content too large".to_string()),
                    None,
                    redirect_url,
                ));
            }
        }

        // Fetch the body
        let html = response.text().await
            .map_err(|e| AccessibilityStatus::NetworkError(e.to_string()))?;

        // Truncate if too large
        let html = if html.len() > self.config.max_content_size {
            html[..self.config.max_content_size].to_string()
        } else {
            html
        };

        let content = self.parse_html_content(&html);
        Ok((AccessibilityStatus::Accessible, Some(content), redirect_url))
    }

    /// Parse HTML content and extract structured information
    fn parse_html_content(&self, html: &str) -> PageContent {
        let title = Self::extract_title(html).unwrap_or_default();
        let description = Self::extract_meta_content(html, "description");
        let text = Self::extract_text_content(html);
        let keywords = Self::extract_keywords(html);
        let images = Self::extract_images(html);
        let links = Self::extract_links(html);

        PageContent {
            html: html.to_string(),
            text,
            title,
            description,
            keywords,
            images,
            links,
            extracted_at: Utc::now(),
        }
    }

    /// Convert HTTP status code to AccessibilityStatus
    fn status_code_to_accessibility(status_code: u16) -> AccessibilityStatus {
        match status_code {
            200..=299 => AccessibilityStatus::Accessible,
            301 | 302 | 307 | 308 => AccessibilityStatus::Accessible,
            403 => AccessibilityStatus::Forbidden,
            404 => AccessibilityStatus::NotFound,
            _ => AccessibilityStatus::NetworkError(format!("HTTP {}", status_code)),
        }
    }

    /// Convert reqwest error to AccessibilityStatus
    fn error_to_accessibility(e: &reqwest::Error) -> AccessibilityStatus {
        if e.is_timeout() {
            AccessibilityStatus::Timeout
        } else if e.is_connect() {
            AccessibilityStatus::NetworkError("Connection failed".to_string())
        } else {
            AccessibilityStatus::NetworkError(e.to_string())
        }
    }

    /// Extract title from HTML
    fn extract_title(html: &str) -> Option<String> {
        // Try <title> tag first
        if let Some(start) = html.find("<title") {
            if let Some(tag_end) = html[start..].find('>') {
                let content_start = start + tag_end + 1;
                if let Some(end) = html[content_start..].find("</title>") {
                    let title = &html[content_start..content_start + end];
                    let title = Self::decode_html_entities(title.trim());
                    if !title.is_empty() {
                        return Some(title);
                    }
                }
            }
        }

        // Try og:title
        Self::extract_og_content(html, "title")
    }

    /// Extract meta content by name
    fn extract_meta_content(html: &str, name: &str) -> Option<String> {
        let patterns = [
            format!(r#"<meta name="{}" content=""#, name),
            format!(r#"<meta name='{}' content='"#, name),
            format!(r#"<meta content="" name="{}""#, name),
            format!(r#"<meta content='' name='{}'"#, name),
        ];

        for pattern in &patterns {
            if let Some(start) = html.to_lowercase().find(&pattern.to_lowercase()) {
                let quote_char = if pattern.contains('"') { '"' } else { '\'' };
                let content_start = start + pattern.len();
                if let Some(end) = html[content_start..].find(quote_char) {
                    let content = &html[content_start..content_start + end];
                    let content = Self::decode_html_entities(content.trim());
                    if !content.is_empty() {
                        return Some(content);
                    }
                }
            }
        }
        None
    }

    /// Extract Open Graph content
    fn extract_og_content(html: &str, property: &str) -> Option<String> {
        let og_property = format!("og:{}", property);
        let patterns = [
            format!(r#"<meta property="{}" content=""#, og_property),
            format!(r#"<meta property='{}' content='"#, og_property),
            format!(r#"<meta content="" property="{}""#, og_property),
            format!(r#"<meta content='' property='{}'"#, og_property),
        ];

        for pattern in &patterns {
            if let Some(start) = html.to_lowercase().find(&pattern.to_lowercase()) {
                let quote_char = if pattern.contains('"') { '"' } else { '\'' };
                let content_start = start + pattern.len();
                if let Some(end) = html[content_start..].find(quote_char) {
                    let content = &html[content_start..content_start + end];
                    let content = Self::decode_html_entities(content.trim());
                    if !content.is_empty() {
                        return Some(content);
                    }
                }
            }
        }
        None
    }

    /// Extract date from meta tags
    fn extract_date(html: &str, property: &str) -> Option<DateTime<Utc>> {
        let content = Self::extract_meta_content(html, property)
            .or_else(|| Self::extract_og_content(html, property))?;
        
        // Try parsing ISO 8601 format
        DateTime::parse_from_rfc3339(&content)
            .map(|dt| dt.with_timezone(&Utc))
            .ok()
    }

    /// Extract language from HTML
    fn extract_language(html: &str) -> Option<String> {
        // Try html lang attribute
        let patterns = [
            r#"<html lang=""#,
            r#"<html lang='"#,
        ];

        for pattern in &patterns {
            if let Some(start) = html.to_lowercase().find(pattern) {
                let quote_char = if pattern.contains('"') { '"' } else { '\'' };
                let content_start = start + pattern.len();
                if let Some(end) = html[content_start..].find(quote_char) {
                    let lang = &html[content_start..content_start + end];
                    if !lang.is_empty() {
                        return Some(lang.to_string());
                    }
                }
            }
        }

        // Try Content-Language meta tag
        Self::extract_meta_content(html, "content-language")
    }

    /// Extract canonical URL
    fn extract_canonical_url(html: &str) -> Option<String> {
        let patterns = [
            r#"<link rel="canonical" href=""#,
            r#"<link rel='canonical' href='"#,
            r#"<link href="" rel="canonical""#,
            r#"<link href='' rel='canonical'"#,
        ];

        for pattern in &patterns {
            if let Some(start) = html.to_lowercase().find(&pattern.to_lowercase()) {
                let quote_char = if pattern.contains('"') { '"' } else { '\'' };
                let content_start = start + pattern.len();
                if let Some(end) = html[content_start..].find(quote_char) {
                    let url = &html[content_start..content_start + end];
                    if !url.is_empty() {
                        return Some(url.to_string());
                    }
                }
            }
        }
        None
    }

    /// Extract text content from HTML (strip tags)
    fn extract_text_content(html: &str) -> String {
        let mut text = String::new();
        let mut in_tag = false;
        let mut in_script = false;
        let mut in_style = false;

        let html_lower = html.to_lowercase();
        let chars: Vec<char> = html.chars().collect();
        let chars_lower: Vec<char> = html_lower.chars().collect();

        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '<' {
                // Check for script/style tags
                let remaining: String = chars_lower[i..].iter().collect();
                if remaining.starts_with("<script") {
                    in_script = true;
                } else if remaining.starts_with("</script") {
                    in_script = false;
                } else if remaining.starts_with("<style") {
                    in_style = true;
                } else if remaining.starts_with("</style") {
                    in_style = false;
                }
                in_tag = true;
            } else if chars[i] == '>' {
                in_tag = false;
            } else if !in_tag && !in_script && !in_style {
                text.push(chars[i]);
            }
            i += 1;
        }

        // Clean up whitespace
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        Self::decode_html_entities(&text)
    }

    /// Extract keywords from meta tags
    fn extract_keywords(html: &str) -> Vec<String> {
        Self::extract_meta_content(html, "keywords")
            .map(|k| k.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
            .unwrap_or_default()
    }

    /// Extract image URLs from HTML
    fn extract_images(html: &str) -> Vec<String> {
        let mut images = Vec::new();
        let html_lower = html.to_lowercase();
        
        // Extract from img src
        let mut pos = 0;
        while let Some(start) = html_lower[pos..].find("<img") {
            let img_start = pos + start;
            if let Some(end) = html_lower[img_start..].find('>') {
                let img_tag = &html[img_start..img_start + end + 1];
                if let Some(src) = Self::extract_attribute(img_tag, "src") {
                    if !src.is_empty() {
                        images.push(src);
                    }
                }
                pos = img_start + end + 1;
            } else {
                break;
            }
        }

        // Also get og:image
        if let Some(og_image) = Self::extract_og_content(html, "image") {
            if !images.contains(&og_image) {
                images.insert(0, og_image);
            }
        }

        images
    }

    /// Extract links from HTML
    fn extract_links(html: &str) -> Vec<String> {
        let mut links = Vec::new();
        let html_lower = html.to_lowercase();
        
        let mut pos = 0;
        while let Some(start) = html_lower[pos..].find("<a ") {
            let a_start = pos + start;
            if let Some(end) = html_lower[a_start..].find('>') {
                let a_tag = &html[a_start..a_start + end + 1];
                if let Some(href) = Self::extract_attribute(a_tag, "href") {
                    if !href.is_empty() && !href.starts_with('#') && !href.starts_with("javascript:") {
                        links.push(href);
                    }
                }
                pos = a_start + end + 1;
            } else {
                break;
            }
        }

        links
    }

    /// Extract attribute value from a tag
    fn extract_attribute(tag: &str, attr: &str) -> Option<String> {
        let patterns = [
            format!(r#"{}=""#, attr),
            format!(r#"{}='"#, attr),
        ];

        for pattern in &patterns {
            if let Some(start) = tag.to_lowercase().find(&pattern.to_lowercase()) {
                let quote_char = if pattern.contains('"') { '"' } else { '\'' };
                let content_start = start + pattern.len();
                if let Some(end) = tag[content_start..].find(quote_char) {
                    return Some(tag[content_start..content_start + end].to_string());
                }
            }
        }
        None
    }

    /// Decode common HTML entities
    fn decode_html_entities(text: &str) -> String {
        text.replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&apos;", "'")
            .replace("&nbsp;", " ")
            .replace("&#x27;", "'")
            .replace("&#x2F;", "/")
            .replace("&mdash;", "—")
            .replace("&ndash;", "–")
            .replace("&hellip;", "…")
            .replace("&copy;", "©")
            .replace("&reg;", "®")
            .replace("&trade;", "™")
    }
}

impl Default for BookmarkContentAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Batch Bookmark Processing and Deduplication
// Implements Requirements 2.4 and 2.5:
// - Detect duplicate or similar bookmarks and provide merge suggestions
// - Display intelligently categorized bookmark library and recommended actions
// ============================================================================

/// Configuration for batch bookmark analysis
#[derive(Debug, Clone)]
pub struct BatchAnalysisConfig {
    /// Similarity threshold for detecting duplicates (0.0 - 1.0)
    pub similarity_threshold: f32,
    /// Whether to detect exact URL duplicates
    pub detect_exact_duplicates: bool,
    /// Whether to detect similar content duplicates
    pub detect_similar_content: bool,
    /// Whether to detect redirect chain duplicates
    pub detect_redirect_chains: bool,
    /// Maximum number of concurrent content fetches
    pub max_concurrent_fetches: usize,
}

impl Default for BatchAnalysisConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.8,
            detect_exact_duplicates: true,
            detect_similar_content: true,
            detect_redirect_chains: true,
            max_concurrent_fetches: 10,
        }
    }
}

/// Result of batch bookmark analysis including duplicates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchBookmarkAnalysis {
    /// Total number of bookmarks analyzed
    pub total_bookmarks: usize,
    /// Number of unique bookmarks (no duplicates)
    pub unique_bookmarks: usize,
    /// Number of duplicate groups found
    pub duplicate_groups_count: usize,
    /// Detected duplicate groups
    pub duplicate_groups: Vec<DuplicateGroup>,
    /// Merge suggestions for duplicate groups
    pub merge_suggestions: Vec<MergeSuggestion>,
    /// Individual bookmark analysis results
    pub bookmark_results: Vec<BookmarkContentResult>,
    /// Analysis started at
    pub started_at: DateTime<Utc>,
    /// Analysis completed at
    pub completed_at: DateTime<Utc>,
    /// Total duration in milliseconds
    pub total_duration_ms: u64,
}

/// Suggestion for merging duplicate bookmarks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeSuggestion {
    /// ID of the duplicate group this suggestion applies to
    pub group_id: web_page_manager_core::Uuid,
    /// The bookmark recommended to keep
    pub keep_bookmark: BookmarkInfo,
    /// Bookmarks recommended to remove
    pub remove_bookmarks: Vec<BookmarkInfo>,
    /// Reason for the suggestion
    pub reason: String,
    /// Confidence score for this suggestion (0.0 - 1.0)
    pub confidence: f32,
    /// Merged metadata from all bookmarks in the group
    pub merged_metadata: MergedBookmarkMetadata,
}

/// Merged metadata from multiple bookmarks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedBookmarkMetadata {
    /// Best title from all bookmarks
    pub best_title: String,
    /// Combined keywords from all bookmarks
    pub combined_keywords: Vec<String>,
    /// Best folder path suggestion
    pub suggested_folder_path: Vec<String>,
    /// Combined description
    pub combined_description: Option<String>,
}

/// Batch bookmark processor for analyzing and deduplicating bookmarks
///
/// Implements Requirements 2.4 and 2.5:
/// - Detect duplicate or similar bookmarks
/// - Provide automatic merge and deduplication suggestions
pub struct BatchBookmarkProcessor {
    analyzer: BookmarkContentAnalyzer,
    config: BatchAnalysisConfig,
}

impl BatchBookmarkProcessor {
    /// Create a new batch bookmark processor with default configuration
    pub fn new() -> Self {
        Self {
            analyzer: BookmarkContentAnalyzer::new(),
            config: BatchAnalysisConfig::default(),
        }
    }

    /// Create a new batch bookmark processor with custom configuration
    pub fn with_config(config: BatchAnalysisConfig) -> Self {
        Self {
            analyzer: BookmarkContentAnalyzer::new(),
            config,
        }
    }

    /// Create a new batch bookmark processor with custom analyzer and config
    pub fn with_analyzer_and_config(
        analyzer: BookmarkContentAnalyzer,
        config: BatchAnalysisConfig,
    ) -> Self {
        Self { analyzer, config }
    }

    /// Get the current configuration
    pub fn config(&self) -> &BatchAnalysisConfig {
        &self.config
    }

    /// Analyze a batch of bookmarks and detect duplicates
    ///
    /// This method:
    /// 1. Fetches content for all bookmarks
    /// 2. Detects exact URL duplicates
    /// 3. Detects similar content duplicates
    /// 4. Detects redirect chain duplicates
    /// 5. Generates merge suggestions
    pub async fn analyze_batch(&self, bookmarks: &[BookmarkInfo]) -> BatchBookmarkAnalysis {
        let started_at = Utc::now();
        let start = std::time::Instant::now();

        // Fetch content for all bookmarks
        let batch_result = self.analyzer.fetch_batch(bookmarks).await;
        let bookmark_results = batch_result.results;

        // Detect duplicates
        let mut duplicate_groups = Vec::new();

        // 1. Detect exact URL duplicates
        if self.config.detect_exact_duplicates {
            let exact_duplicates = self.detect_exact_url_duplicates(bookmarks);
            duplicate_groups.extend(exact_duplicates);
        }

        // 2. Detect redirect chain duplicates
        if self.config.detect_redirect_chains {
            let redirect_duplicates = self.detect_redirect_duplicates(&bookmark_results);
            duplicate_groups.extend(redirect_duplicates);
        }

        // 3. Detect similar content duplicates
        if self.config.detect_similar_content {
            let content_duplicates = self.detect_similar_content_duplicates(&bookmark_results);
            duplicate_groups.extend(content_duplicates);
        }

        // Merge overlapping duplicate groups
        let duplicate_groups = self.merge_overlapping_groups(duplicate_groups);

        // Generate merge suggestions
        let merge_suggestions = self.generate_merge_suggestions(&duplicate_groups, &bookmark_results);

        // Calculate unique bookmarks
        let duplicated_bookmark_ids: std::collections::HashSet<_> = duplicate_groups
            .iter()
            .flat_map(|g| g.bookmarks.iter().skip(1).map(|b| &b.id))
            .collect();
        let unique_bookmarks = bookmarks.len() - duplicated_bookmark_ids.len();

        BatchBookmarkAnalysis {
            total_bookmarks: bookmarks.len(),
            unique_bookmarks,
            duplicate_groups_count: duplicate_groups.len(),
            duplicate_groups,
            merge_suggestions,
            bookmark_results,
            started_at,
            completed_at: Utc::now(),
            total_duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// Detect bookmarks with exact same URLs
    fn detect_exact_url_duplicates(&self, bookmarks: &[BookmarkInfo]) -> Vec<DuplicateGroup> {
        use std::collections::HashMap;

        let mut url_groups: HashMap<String, Vec<BookmarkInfo>> = HashMap::new();

        for bookmark in bookmarks {
            let normalized_url = Self::normalize_url(&bookmark.url);
            url_groups
                .entry(normalized_url)
                .or_default()
                .push(bookmark.clone());
        }

        url_groups
            .into_iter()
            .filter(|(_, group)| group.len() > 1)
            .map(|(_, bookmarks)| {
                let suggested_keep = Self::select_best_bookmark(&bookmarks);
                DuplicateGroup {
                    id: web_page_manager_core::Uuid::new_v4(),
                    bookmarks,
                    duplicate_type: DuplicateType::ExactUrl,
                    similarity_score: 1.0,
                    suggested_keep,
                }
            })
            .collect()
    }

    /// Detect bookmarks that redirect to the same final URL
    fn detect_redirect_duplicates(&self, results: &[BookmarkContentResult]) -> Vec<DuplicateGroup> {
        use std::collections::HashMap;

        let mut final_url_groups: HashMap<String, Vec<BookmarkInfo>> = HashMap::new();

        for result in results {
            if let Some(ref final_url) = result.final_url {
                // Only consider if the final URL is different from the original
                if final_url != &result.bookmark.url {
                    let normalized = Self::normalize_url(final_url);
                    final_url_groups
                        .entry(normalized)
                        .or_default()
                        .push(result.bookmark.clone());
                }
            }
        }

        // Also group bookmarks whose original URL matches another's final URL
        for result in results {
            if let Some(ref final_url) = result.final_url {
                let normalized_final = Self::normalize_url(final_url);
                
                // Find bookmarks whose original URL matches this final URL
                for other_result in results {
                    if other_result.bookmark.id != result.bookmark.id {
                        let normalized_original = Self::normalize_url(&other_result.bookmark.url);
                        if normalized_original == normalized_final {
                            final_url_groups
                                .entry(normalized_final.clone())
                                .or_default()
                                .push(result.bookmark.clone());
                            break;
                        }
                    }
                }
            }
        }

        final_url_groups
            .into_iter()
            .filter(|(_, group)| group.len() > 1)
            .map(|(_, mut bookmarks)| {
                // Deduplicate bookmarks in the group
                bookmarks.sort_by(|a, b| a.id.0.cmp(&b.id.0));
                bookmarks.dedup_by(|a, b| a.id == b.id);
                
                if bookmarks.len() < 2 {
                    return None;
                }
                
                let suggested_keep = Self::select_best_bookmark(&bookmarks);
                Some(DuplicateGroup {
                    id: web_page_manager_core::Uuid::new_v4(),
                    bookmarks,
                    duplicate_type: DuplicateType::RedirectChain,
                    similarity_score: 0.95,
                    suggested_keep,
                })
            })
            .flatten()
            .collect()
    }

    /// Detect bookmarks with similar content
    fn detect_similar_content_duplicates(&self, results: &[BookmarkContentResult]) -> Vec<DuplicateGroup> {
        let mut groups: Vec<DuplicateGroup> = Vec::new();
        let mut processed: std::collections::HashSet<Uuid> = std::collections::HashSet::new();

        for (i, result_a) in results.iter().enumerate() {
            if processed.contains(&result_a.bookmark.id.0) {
                continue;
            }

            let content_a = match &result_a.content {
                Some(c) => c,
                None => continue,
            };

            let mut similar_bookmarks = vec![result_a.bookmark.clone()];
            let mut max_similarity = 0.0f32;

            for result_b in results.iter().skip(i + 1) {
                if processed.contains(&result_b.bookmark.id.0) {
                    continue;
                }

                let content_b = match &result_b.content {
                    Some(c) => c,
                    None => continue,
                };

                let similarity = self.calculate_content_similarity(content_a, content_b);
                
                if similarity >= self.config.similarity_threshold {
                    similar_bookmarks.push(result_b.bookmark.clone());
                    processed.insert(result_b.bookmark.id.0.clone());
                    max_similarity = max_similarity.max(similarity);
                }
            }

            if similar_bookmarks.len() > 1 {
                processed.insert(result_a.bookmark.id.0.clone());
                let suggested_keep = Self::select_best_bookmark(&similar_bookmarks);
                
                groups.push(DuplicateGroup {
                    id: web_page_manager_core::Uuid::new_v4(),
                    bookmarks: similar_bookmarks,
                    duplicate_type: DuplicateType::SameContent,
                    similarity_score: max_similarity,
                    suggested_keep,
                });
            }
        }

        groups
    }

    /// Calculate similarity between two page contents
    fn calculate_content_similarity(&self, content_a: &PageContent, content_b: &PageContent) -> f32 {
        let mut total_score = 0.0f32;
        let mut weight_sum = 0.0f32;

        // Title similarity (weight: 0.3)
        let title_sim = Self::string_similarity(&content_a.title, &content_b.title);
        total_score += title_sim * 0.3;
        weight_sum += 0.3;

        // Text content similarity using Jaccard index (weight: 0.4)
        let text_sim = Self::jaccard_similarity(&content_a.text, &content_b.text);
        total_score += text_sim * 0.4;
        weight_sum += 0.4;

        // Keywords overlap (weight: 0.2)
        let keywords_sim = Self::set_similarity(&content_a.keywords, &content_b.keywords);
        total_score += keywords_sim * 0.2;
        weight_sum += 0.2;

        // Description similarity (weight: 0.1)
        if let (Some(desc_a), Some(desc_b)) = (&content_a.description, &content_b.description) {
            let desc_sim = Self::string_similarity(desc_a, desc_b);
            total_score += desc_sim * 0.1;
            weight_sum += 0.1;
        }

        if weight_sum > 0.0 {
            total_score / weight_sum
        } else {
            0.0
        }
    }

    /// Calculate string similarity using Levenshtein distance ratio
    fn string_similarity(a: &str, b: &str) -> f32 {
        if a.is_empty() && b.is_empty() {
            return 1.0;
        }
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }

        let a_lower = a.to_lowercase();
        let b_lower = b.to_lowercase();

        if a_lower == b_lower {
            return 1.0;
        }

        let max_len = a_lower.len().max(b_lower.len());
        let distance = Self::levenshtein_distance(&a_lower, &b_lower);
        
        1.0 - (distance as f32 / max_len as f32)
    }

    /// Calculate Levenshtein distance between two strings
    fn levenshtein_distance(a: &str, b: &str) -> usize {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let a_len = a_chars.len();
        let b_len = b_chars.len();

        if a_len == 0 {
            return b_len;
        }
        if b_len == 0 {
            return a_len;
        }

        let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

        for i in 0..=a_len {
            matrix[i][0] = i;
        }
        for j in 0..=b_len {
            matrix[0][j] = j;
        }

        for i in 1..=a_len {
            for j in 1..=b_len {
                let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }

        matrix[a_len][b_len]
    }

    /// Calculate Jaccard similarity between two text strings
    fn jaccard_similarity(a: &str, b: &str) -> f32 {
        let words_a: std::collections::HashSet<_> = Self::tokenize(a).into_iter().collect();
        let words_b: std::collections::HashSet<_> = Self::tokenize(b).into_iter().collect();

        if words_a.is_empty() && words_b.is_empty() {
            return 1.0;
        }
        if words_a.is_empty() || words_b.is_empty() {
            return 0.0;
        }

        let intersection = words_a.intersection(&words_b).count();
        let union = words_a.union(&words_b).count();

        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }

    /// Calculate set similarity between two keyword lists
    fn set_similarity(a: &[String], b: &[String]) -> f32 {
        if a.is_empty() && b.is_empty() {
            return 1.0;
        }
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }

        let set_a: std::collections::HashSet<_> = a.iter().map(|s| s.to_lowercase()).collect();
        let set_b: std::collections::HashSet<_> = b.iter().map(|s| s.to_lowercase()).collect();

        let intersection = set_a.intersection(&set_b).count();
        let union = set_a.union(&set_b).count();

        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }

    /// Tokenize text into words
    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() > 2)
            .map(|s| s.to_string())
            .collect()
    }

    /// Normalize URL for comparison
    fn normalize_url(url: &str) -> String {
        let mut normalized = url.to_lowercase();
        
        // Remove fragment first
        if let Some(fragment_start) = normalized.find('#') {
            normalized = normalized[..fragment_start].to_string();
        }
        
        // Remove common tracking parameters
        if let Some(query_start) = normalized.find('?') {
            let base = &normalized[..query_start];
            let query = &normalized[query_start + 1..];
            
            let filtered_params: Vec<&str> = query
                .split('&')
                .filter(|param| {
                    let param_lower = param.to_lowercase();
                    !param_lower.starts_with("utm_")
                        && !param_lower.starts_with("ref=")
                        && !param_lower.starts_with("source=")
                        && !param_lower.starts_with("fbclid=")
                        && !param_lower.starts_with("gclid=")
                })
                .collect();
            
            if filtered_params.is_empty() {
                normalized = base.to_string();
            } else {
                normalized = format!("{}?{}", base, filtered_params.join("&"));
            }
        }
        
        // Remove trailing slash (after query params are processed)
        if normalized.ends_with('/') {
            normalized.pop();
        }
        
        // Remove www. prefix
        normalized = normalized.replace("://www.", "://");
        
        normalized
    }

    /// Select the best bookmark to keep from a group
    fn select_best_bookmark(bookmarks: &[BookmarkInfo]) -> Option<BookmarkId> {
        if bookmarks.is_empty() {
            return None;
        }

        // Score each bookmark based on various factors
        let mut best_score = 0i32;
        let mut best_id = bookmarks[0].id.clone();

        for bookmark in bookmarks {
            let mut score = 0i32;

            // Prefer bookmarks with longer titles (more descriptive)
            score += (bookmark.title.len() / 10) as i32;

            // Prefer bookmarks with favicon
            if bookmark.favicon_url.is_some() {
                score += 5;
            }

            // Prefer bookmarks that have been accessed
            if bookmark.last_accessed.is_some() {
                score += 3;
            }

            // Prefer bookmarks with deeper folder paths (more organized)
            score += bookmark.folder_path.len() as i32;

            // Prefer older bookmarks (established)
            // Using created_at timestamp - older is better
            let age_days = (Utc::now() - bookmark.created_at).num_days();
            score += (age_days / 30) as i32; // +1 point per month

            if score > best_score {
                best_score = score;
                best_id = bookmark.id.clone();
            }
        }

        Some(best_id)
    }

    /// Merge overlapping duplicate groups
    fn merge_overlapping_groups(&self, groups: Vec<DuplicateGroup>) -> Vec<DuplicateGroup> {
        if groups.is_empty() {
            return groups;
        }

        let mut merged_groups: Vec<DuplicateGroup> = Vec::new();
        let mut processed_ids: std::collections::HashSet<web_page_manager_core::Uuid> = std::collections::HashSet::new();

        for group in &groups {
            if processed_ids.contains(&group.id) {
                continue;
            }

            let mut merged_bookmarks: Vec<BookmarkInfo> = group.bookmarks.clone();
            let mut merged_type = group.duplicate_type.clone();
            let mut max_similarity = group.similarity_score;

            // Find overlapping groups
            for other_group in &groups {
                if other_group.id == group.id || processed_ids.contains(&other_group.id) {
                    continue;
                }

                let has_overlap = group.bookmarks.iter().any(|b| {
                    other_group.bookmarks.iter().any(|ob| ob.id == b.id)
                });

                if has_overlap {
                    // Merge the groups
                    for bookmark in &other_group.bookmarks {
                        if !merged_bookmarks.iter().any(|b| b.id == bookmark.id) {
                            merged_bookmarks.push(bookmark.clone());
                        }
                    }
                    
                    // Use the more specific duplicate type
                    if matches!(other_group.duplicate_type, DuplicateType::ExactUrl) {
                        merged_type = DuplicateType::ExactUrl;
                    }
                    
                    max_similarity = max_similarity.max(other_group.similarity_score);
                    processed_ids.insert(other_group.id);
                }
            }

            processed_ids.insert(group.id);

            let suggested_keep = Self::select_best_bookmark(&merged_bookmarks);
            merged_groups.push(DuplicateGroup {
                id: web_page_manager_core::Uuid::new_v4(),
                bookmarks: merged_bookmarks,
                duplicate_type: merged_type,
                similarity_score: max_similarity,
                suggested_keep,
            });
        }

        merged_groups
    }

    /// Generate merge suggestions for duplicate groups
    fn generate_merge_suggestions(
        &self,
        groups: &[DuplicateGroup],
        results: &[BookmarkContentResult],
    ) -> Vec<MergeSuggestion> {
        groups
            .iter()
            .filter_map(|group| {
                if group.bookmarks.len() < 2 {
                    return None;
                }

                let keep_id = group.suggested_keep.as_ref()?;
                let keep_bookmark = group.bookmarks.iter().find(|b| &b.id == keep_id)?.clone();
                
                let remove_bookmarks: Vec<BookmarkInfo> = group
                    .bookmarks
                    .iter()
                    .filter(|b| &b.id != keep_id)
                    .cloned()
                    .collect();

                let reason = match &group.duplicate_type {
                    DuplicateType::ExactUrl => "These bookmarks have identical URLs".to_string(),
                    DuplicateType::SameContent => format!(
                        "These bookmarks have similar content ({}% similarity)",
                        (group.similarity_score * 100.0) as u32
                    ),
                    DuplicateType::SimilarTitle => "These bookmarks have similar titles".to_string(),
                    DuplicateType::RedirectChain => {
                        "These bookmarks redirect to the same destination".to_string()
                    }
                };

                let merged_metadata = self.create_merged_metadata(&group.bookmarks, results);

                let confidence = match &group.duplicate_type {
                    DuplicateType::ExactUrl => 0.99,
                    DuplicateType::RedirectChain => 0.95,
                    DuplicateType::SameContent => group.similarity_score * 0.9,
                    DuplicateType::SimilarTitle => group.similarity_score * 0.7,
                };

                Some(MergeSuggestion {
                    group_id: group.id,
                    keep_bookmark,
                    remove_bookmarks,
                    reason,
                    confidence,
                    merged_metadata,
                })
            })
            .collect()
    }

    /// Create merged metadata from all bookmarks in a group
    fn create_merged_metadata(
        &self,
        bookmarks: &[BookmarkInfo],
        results: &[BookmarkContentResult],
    ) -> MergedBookmarkMetadata {
        // Find the best title (longest non-empty)
        let best_title = bookmarks
            .iter()
            .map(|b| &b.title)
            .filter(|t| !t.is_empty())
            .max_by_key(|t| t.len())
            .cloned()
            .unwrap_or_default();

        // Combine all keywords from content results
        let mut combined_keywords: Vec<String> = Vec::new();
        for bookmark in bookmarks {
            if let Some(result) = results.iter().find(|r| r.bookmark.id == bookmark.id) {
                if let Some(content) = &result.content {
                    for keyword in &content.keywords {
                        if !combined_keywords.contains(keyword) {
                            combined_keywords.push(keyword.clone());
                        }
                    }
                }
            }
        }
        combined_keywords.truncate(20); // Limit to 20 keywords

        // Find the best folder path (deepest)
        let suggested_folder_path = bookmarks
            .iter()
            .map(|b| &b.folder_path)
            .max_by_key(|p| p.len())
            .cloned()
            .unwrap_or_default();

        // Combine descriptions
        let combined_description = results
            .iter()
            .filter(|r| bookmarks.iter().any(|b| b.id == r.bookmark.id))
            .filter_map(|r| r.content.as_ref())
            .filter_map(|c| c.description.as_ref())
            .max_by_key(|d| d.len())
            .cloned();

        MergedBookmarkMetadata {
            best_title,
            combined_keywords,
            suggested_folder_path,
            combined_description,
        }
    }
}

impl Default for BatchBookmarkProcessor {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export DuplicateGroup and DuplicateType from core
pub use web_page_manager_core::{DuplicateGroup, DuplicateType};


#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_bookmark(url: &str, title: &str) -> BookmarkInfo {
        BookmarkInfo {
            id: BookmarkId::new(),
            url: url.to_string(),
            title: title.to_string(),
            favicon_url: None,
            browser_type: BrowserType::Chrome,
            folder_path: vec!["Test".to_string()],
            created_at: Utc::now(),
            last_accessed: None,
        }
    }

    #[test]
    fn test_analyzer_creation() {
        let analyzer = BookmarkContentAnalyzer::new();
        assert_eq!(analyzer.config().request_timeout_secs, 15);
        assert_eq!(analyzer.config().max_concurrent_requests, 10);
    }

    #[test]
    fn test_analyzer_with_config() {
        let config = BookmarkContentAnalyzerConfig {
            request_timeout_secs: 30,
            max_concurrent_requests: 5,
            ..Default::default()
        };
        let analyzer = BookmarkContentAnalyzer::with_config(config);
        assert_eq!(analyzer.config().request_timeout_secs, 30);
        assert_eq!(analyzer.config().max_concurrent_requests, 5);
    }

    #[test]
    fn test_is_valid_url() {
        assert!(BookmarkContentAnalyzer::is_valid_url("https://example.com"));
        assert!(BookmarkContentAnalyzer::is_valid_url("http://example.com"));
        assert!(!BookmarkContentAnalyzer::is_valid_url("ftp://example.com"));
        assert!(!BookmarkContentAnalyzer::is_valid_url("file:///path/to/file"));
        assert!(!BookmarkContentAnalyzer::is_valid_url("javascript:void(0)"));
    }

    #[test]
    fn test_extract_title() {
        let html = r#"<html><head><title>Test Page Title</title></head><body></body></html>"#;
        let title = BookmarkContentAnalyzer::extract_title(html);
        assert_eq!(title, Some("Test Page Title".to_string()));
    }

    #[test]
    fn test_extract_title_with_whitespace() {
        let html = r#"<html><head><title>  Test Page Title  </title></head></html>"#;
        let title = BookmarkContentAnalyzer::extract_title(html);
        assert_eq!(title, Some("Test Page Title".to_string()));
    }

    #[test]
    fn test_extract_meta_description() {
        let html = r#"<html><head><meta name="description" content="This is a test description"></head></html>"#;
        let desc = BookmarkContentAnalyzer::extract_meta_content(html, "description");
        assert_eq!(desc, Some("This is a test description".to_string()));
    }

    #[test]
    fn test_extract_og_content() {
        let html = r#"<html><head><meta property="og:title" content="OG Title"></head></html>"#;
        let og_title = BookmarkContentAnalyzer::extract_og_content(html, "title");
        assert_eq!(og_title, Some("OG Title".to_string()));
    }

    #[test]
    fn test_extract_language() {
        let html = r#"<html lang="en-US"><head></head><body></body></html>"#;
        let lang = BookmarkContentAnalyzer::extract_language(html);
        assert_eq!(lang, Some("en-US".to_string()));
    }

    #[test]
    fn test_extract_canonical_url() {
        let html = r#"<html><head><link rel="canonical" href="https://example.com/page"></head></html>"#;
        let canonical = BookmarkContentAnalyzer::extract_canonical_url(html);
        assert_eq!(canonical, Some("https://example.com/page".to_string()));
    }

    #[test]
    fn test_extract_keywords() {
        let html = r#"<html><head><meta name="keywords" content="rust, programming, web"></head></html>"#;
        let keywords = BookmarkContentAnalyzer::extract_keywords(html);
        assert_eq!(keywords, vec!["rust", "programming", "web"]);
    }

    #[test]
    fn test_extract_text_content() {
        let html = r#"<html><body><h1>Title</h1><p>This is a paragraph.</p></body></html>"#;
        let text = BookmarkContentAnalyzer::extract_text_content(html);
        assert!(text.contains("Title"));
        assert!(text.contains("This is a paragraph."));
    }

    #[test]
    fn test_extract_text_content_strips_script() {
        let html = r#"<html><body><script>var x = 1;</script><p>Content</p></body></html>"#;
        let text = BookmarkContentAnalyzer::extract_text_content(html);
        assert!(!text.contains("var x"));
        assert!(text.contains("Content"));
    }

    #[test]
    fn test_extract_text_content_strips_style() {
        let html = r#"<html><body><style>.class { color: red; }</style><p>Content</p></body></html>"#;
        let text = BookmarkContentAnalyzer::extract_text_content(html);
        assert!(!text.contains("color"));
        assert!(text.contains("Content"));
    }

    #[test]
    fn test_extract_images() {
        let html = r#"<html><body><img src="image1.jpg"><img src="image2.png"></body></html>"#;
        let images = BookmarkContentAnalyzer::extract_images(html);
        assert_eq!(images.len(), 2);
        assert!(images.contains(&"image1.jpg".to_string()));
        assert!(images.contains(&"image2.png".to_string()));
    }

    #[test]
    fn test_extract_links() {
        let html = r#"<html><body><a href="https://example.com">Link 1</a><a href="/page">Link 2</a></body></html>"#;
        let links = BookmarkContentAnalyzer::extract_links(html);
        assert_eq!(links.len(), 2);
        assert!(links.contains(&"https://example.com".to_string()));
        assert!(links.contains(&"/page".to_string()));
    }

    #[test]
    fn test_extract_links_filters_javascript() {
        let html = r#"<html><body><a href="javascript:void(0)">JS Link</a><a href="https://example.com">Real Link</a></body></html>"#;
        let links = BookmarkContentAnalyzer::extract_links(html);
        assert_eq!(links.len(), 1);
        assert!(links.contains(&"https://example.com".to_string()));
    }

    #[test]
    fn test_extract_links_filters_anchors() {
        let html = "<html><body><a href=\"#section\">Anchor</a><a href=\"https://example.com\">Real Link</a></body></html>";
        let links = BookmarkContentAnalyzer::extract_links(html);
        assert_eq!(links.len(), 1);
        assert!(links.contains(&"https://example.com".to_string()));
    }

    #[test]
    fn test_decode_html_entities() {
        assert_eq!(BookmarkContentAnalyzer::decode_html_entities("&amp;"), "&");
        assert_eq!(BookmarkContentAnalyzer::decode_html_entities("&lt;"), "<");
        assert_eq!(BookmarkContentAnalyzer::decode_html_entities("&gt;"), ">");
        assert_eq!(BookmarkContentAnalyzer::decode_html_entities("&quot;"), "\"");
        assert_eq!(BookmarkContentAnalyzer::decode_html_entities("&nbsp;"), " ");
    }

    #[test]
    fn test_status_code_to_accessibility() {
        assert!(matches!(
            BookmarkContentAnalyzer::status_code_to_accessibility(200),
            AccessibilityStatus::Accessible
        ));
        assert!(matches!(
            BookmarkContentAnalyzer::status_code_to_accessibility(301),
            AccessibilityStatus::Accessible
        ));
        assert!(matches!(
            BookmarkContentAnalyzer::status_code_to_accessibility(404),
            AccessibilityStatus::NotFound
        ));
        assert!(matches!(
            BookmarkContentAnalyzer::status_code_to_accessibility(403),
            AccessibilityStatus::Forbidden
        ));
    }

    #[test]
    fn test_parse_html_content() {
        let analyzer = BookmarkContentAnalyzer::new();
        let html = r#"
            <html lang="en">
            <head>
                <title>Test Page</title>
                <meta name="description" content="Test description">
                <meta name="keywords" content="test, page">
            </head>
            <body>
                <h1>Welcome</h1>
                <p>This is test content.</p>
                <img src="test.jpg">
                <a href="https://example.com">Link</a>
            </body>
            </html>
        "#;

        let content = analyzer.parse_html_content(html);
        
        assert_eq!(content.title, "Test Page");
        assert_eq!(content.description, Some("Test description".to_string()));
        assert!(content.keywords.contains(&"test".to_string()));
        assert!(content.keywords.contains(&"page".to_string()));
        assert!(content.text.contains("Welcome"));
        assert!(content.text.contains("This is test content."));
        assert!(content.images.contains(&"test.jpg".to_string()));
        assert!(content.links.contains(&"https://example.com".to_string()));
    }

    #[test]
    fn test_extract_metadata() {
        let analyzer = BookmarkContentAnalyzer::new();
        let content = PageContent {
            html: r#"
                <html lang="en">
                <head>
                    <title>Test Page</title>
                    <meta name="description" content="Test description">
                    <meta name="author" content="Test Author">
                    <meta property="og:image" content="https://example.com/image.jpg">
                    <meta property="og:site_name" content="Test Site">
                    <link rel="canonical" href="https://example.com/canonical">
                </head>
                </html>
            "#.to_string(),
            text: "Test content".to_string(),
            title: "Test Page".to_string(),
            description: Some("Test description".to_string()),
            keywords: vec![],
            images: vec![],
            links: vec![],
            extracted_at: Utc::now(),
        };

        let metadata = analyzer.extract_metadata(&content);
        
        assert_eq!(metadata.title, "Test Page");
        assert_eq!(metadata.description, Some("Test description".to_string()));
        assert_eq!(metadata.author, Some("Test Author".to_string()));
        assert_eq!(metadata.og_image, Some("https://example.com/image.jpg".to_string()));
        assert_eq!(metadata.site_name, Some("Test Site".to_string()));
        assert_eq!(metadata.canonical_url, Some("https://example.com/canonical".to_string()));
        assert_eq!(metadata.language, Some("en".to_string()));
    }

    #[test]
    fn test_config_default() {
        let config = BookmarkContentAnalyzerConfig::default();
        assert_eq!(config.request_timeout_secs, 15);
        assert_eq!(config.max_concurrent_requests, 10);
        assert_eq!(config.max_content_size, 5 * 1024 * 1024);
        assert!(config.follow_redirects);
        assert_eq!(config.max_redirects, 5);
    }

    #[tokio::test]
    async fn test_validate_accessibility_invalid_url() {
        let analyzer = BookmarkContentAnalyzer::new();
        let (status, redirect) = analyzer.validate_accessibility("ftp://invalid.url").await;
        
        assert!(matches!(status, AccessibilityStatus::NetworkError(_)));
        assert!(redirect.is_none());
    }

    #[tokio::test]
    async fn test_fetch_bookmark_content_invalid_url() {
        let analyzer = BookmarkContentAnalyzer::new();
        let bookmark = create_test_bookmark("ftp://invalid.url", "Invalid");
        
        let result = analyzer.fetch_bookmark_content(&bookmark).await;
        
        assert!(matches!(result.status, AccessibilityStatus::NetworkError(_)));
        assert!(result.content.is_none());
        assert!(result.metadata.is_none());
    }

    #[test]
    fn test_batch_analysis_result_creation() {
        let result = BatchAnalysisResult {
            total_bookmarks: 10,
            successful: 8,
            failed: 2,
            results: vec![],
            started_at: Utc::now(),
            completed_at: Utc::now(),
            total_duration_ms: 1000,
        };

        assert_eq!(result.total_bookmarks, 10);
        assert_eq!(result.successful, 8);
        assert_eq!(result.failed, 2);
    }

    #[test]
    fn test_bookmark_content_result_creation() {
        let bookmark = create_test_bookmark("https://example.com", "Test");
        let result = BookmarkContentResult {
            bookmark: bookmark.clone(),
            status: AccessibilityStatus::Accessible,
            content: None,
            metadata: None,
            response_time_ms: 100,
            final_url: None,
            fetched_at: Utc::now(),
        };

        assert_eq!(result.bookmark.url, "https://example.com");
        assert!(matches!(result.status, AccessibilityStatus::Accessible));
        assert_eq!(result.response_time_ms, 100);
    }

    // ============================================================================
    // Tests for Batch Bookmark Processing and Deduplication
    // ============================================================================

    #[test]
    fn test_batch_processor_creation() {
        let processor = BatchBookmarkProcessor::new();
        assert_eq!(processor.config().similarity_threshold, 0.8);
        assert!(processor.config().detect_exact_duplicates);
        assert!(processor.config().detect_similar_content);
        assert!(processor.config().detect_redirect_chains);
    }

    #[test]
    fn test_batch_processor_with_config() {
        let config = BatchAnalysisConfig {
            similarity_threshold: 0.9,
            detect_exact_duplicates: true,
            detect_similar_content: false,
            detect_redirect_chains: false,
            max_concurrent_fetches: 5,
        };
        let processor = BatchBookmarkProcessor::with_config(config);
        assert_eq!(processor.config().similarity_threshold, 0.9);
        assert!(!processor.config().detect_similar_content);
    }

    #[test]
    fn test_normalize_url() {
        // Test trailing slash removal
        assert_eq!(
            BatchBookmarkProcessor::normalize_url("https://example.com/"),
            "https://example.com"
        );

        // Test www removal
        assert_eq!(
            BatchBookmarkProcessor::normalize_url("https://www.example.com"),
            "https://example.com"
        );

        // Test UTM parameter removal
        assert_eq!(
            BatchBookmarkProcessor::normalize_url("https://example.com?utm_source=test"),
            "https://example.com"
        );

        // Test fragment removal
        assert_eq!(
            BatchBookmarkProcessor::normalize_url("https://example.com#section"),
            "https://example.com"
        );

        // Test combined normalization
        assert_eq!(
            BatchBookmarkProcessor::normalize_url("https://www.example.com/page/?utm_source=test#section"),
            "https://example.com/page"
        );
    }

    #[test]
    fn test_detect_exact_url_duplicates() {
        let processor = BatchBookmarkProcessor::new();
        
        let bookmarks = vec![
            create_test_bookmark("https://example.com", "Example 1"),
            create_test_bookmark("https://example.com", "Example 2"),
            create_test_bookmark("https://other.com", "Other"),
        ];

        let duplicates = processor.detect_exact_url_duplicates(&bookmarks);
        
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].bookmarks.len(), 2);
        assert!(matches!(duplicates[0].duplicate_type, DuplicateType::ExactUrl));
        assert_eq!(duplicates[0].similarity_score, 1.0);
    }

    #[test]
    fn test_detect_exact_url_duplicates_with_normalization() {
        let processor = BatchBookmarkProcessor::new();
        
        let bookmarks = vec![
            create_test_bookmark("https://example.com", "Example 1"),
            create_test_bookmark("https://www.example.com/", "Example 2"),
            create_test_bookmark("https://example.com?utm_source=test", "Example 3"),
        ];

        let duplicates = processor.detect_exact_url_duplicates(&bookmarks);
        
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].bookmarks.len(), 3);
    }

    #[test]
    fn test_string_similarity() {
        // Identical strings
        assert_eq!(BatchBookmarkProcessor::string_similarity("hello", "hello"), 1.0);
        
        // Case insensitive
        assert_eq!(BatchBookmarkProcessor::string_similarity("Hello", "hello"), 1.0);
        
        // Empty strings
        assert_eq!(BatchBookmarkProcessor::string_similarity("", ""), 1.0);
        
        // One empty
        assert_eq!(BatchBookmarkProcessor::string_similarity("hello", ""), 0.0);
        
        // Similar strings
        let sim = BatchBookmarkProcessor::string_similarity("hello", "hallo");
        assert!(sim > 0.5 && sim < 1.0);
    }

    #[test]
    fn test_jaccard_similarity() {
        // Identical texts
        let sim = BatchBookmarkProcessor::jaccard_similarity(
            "the quick brown fox",
            "the quick brown fox"
        );
        assert_eq!(sim, 1.0);
        
        // Completely different
        let sim = BatchBookmarkProcessor::jaccard_similarity(
            "hello world",
            "goodbye universe"
        );
        assert!(sim < 0.5);
        
        // Partial overlap
        let sim = BatchBookmarkProcessor::jaccard_similarity(
            "the quick brown fox jumps",
            "the lazy brown dog sleeps"
        );
        assert!(sim > 0.0 && sim < 1.0);
    }

    #[test]
    fn test_set_similarity() {
        // Identical sets
        let sim = BatchBookmarkProcessor::set_similarity(
            &["rust".to_string(), "programming".to_string()],
            &["rust".to_string(), "programming".to_string()]
        );
        assert_eq!(sim, 1.0);
        
        // Empty sets
        let sim = BatchBookmarkProcessor::set_similarity(&[], &[]);
        assert_eq!(sim, 1.0);
        
        // One empty
        let sim = BatchBookmarkProcessor::set_similarity(
            &["rust".to_string()],
            &[]
        );
        assert_eq!(sim, 0.0);
        
        // Partial overlap
        let sim = BatchBookmarkProcessor::set_similarity(
            &["rust".to_string(), "programming".to_string()],
            &["rust".to_string(), "web".to_string()]
        );
        assert!(sim > 0.0 && sim < 1.0);
    }

    #[test]
    fn test_select_best_bookmark() {
        let bookmarks = vec![
            create_test_bookmark("https://example.com", "Short"),
            create_test_bookmark("https://example.com", "A Much Longer and More Descriptive Title"),
        ];

        let best_id = BatchBookmarkProcessor::select_best_bookmark(&bookmarks);
        assert!(best_id.is_some());
        
        // The bookmark with the longer title should be selected
        let best = bookmarks.iter().find(|b| Some(&b.id) == best_id.as_ref()).unwrap();
        assert!(best.title.len() > 10);
    }

    #[test]
    fn test_select_best_bookmark_empty() {
        let bookmarks: Vec<BookmarkInfo> = vec![];
        let best_id = BatchBookmarkProcessor::select_best_bookmark(&bookmarks);
        assert!(best_id.is_none());
    }

    #[test]
    fn test_batch_analysis_config_default() {
        let config = BatchAnalysisConfig::default();
        assert_eq!(config.similarity_threshold, 0.8);
        assert!(config.detect_exact_duplicates);
        assert!(config.detect_similar_content);
        assert!(config.detect_redirect_chains);
        assert_eq!(config.max_concurrent_fetches, 10);
    }

    #[test]
    fn test_levenshtein_distance() {
        // Same strings
        assert_eq!(BatchBookmarkProcessor::levenshtein_distance("hello", "hello"), 0);
        
        // One character difference
        assert_eq!(BatchBookmarkProcessor::levenshtein_distance("hello", "hallo"), 1);
        
        // Empty strings
        assert_eq!(BatchBookmarkProcessor::levenshtein_distance("", ""), 0);
        assert_eq!(BatchBookmarkProcessor::levenshtein_distance("hello", ""), 5);
        assert_eq!(BatchBookmarkProcessor::levenshtein_distance("", "hello"), 5);
        
        // Completely different
        assert_eq!(BatchBookmarkProcessor::levenshtein_distance("abc", "xyz"), 3);
    }

    #[test]
    fn test_tokenize() {
        let tokens = BatchBookmarkProcessor::tokenize("Hello, World! This is a test.");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"this".to_string()));
        assert!(tokens.contains(&"test".to_string()));
        // Short words should be filtered
        assert!(!tokens.contains(&"is".to_string()));
        assert!(!tokens.contains(&"a".to_string()));
    }

    #[test]
    fn test_merge_suggestion_creation() {
        let keep_bookmark = create_test_bookmark("https://example.com", "Keep This");
        let remove_bookmark = create_test_bookmark("https://example.com", "Remove This");
        
        let suggestion = MergeSuggestion {
            group_id: web_page_manager_core::Uuid::new_v4(),
            keep_bookmark: keep_bookmark.clone(),
            remove_bookmarks: vec![remove_bookmark],
            reason: "Test reason".to_string(),
            confidence: 0.95,
            merged_metadata: MergedBookmarkMetadata {
                best_title: "Keep This".to_string(),
                combined_keywords: vec!["test".to_string()],
                suggested_folder_path: vec!["Test".to_string()],
                combined_description: None,
            },
        };

        assert_eq!(suggestion.keep_bookmark.title, "Keep This");
        assert_eq!(suggestion.remove_bookmarks.len(), 1);
        assert_eq!(suggestion.confidence, 0.95);
    }

    #[test]
    fn test_merged_bookmark_metadata() {
        let metadata = MergedBookmarkMetadata {
            best_title: "Best Title".to_string(),
            combined_keywords: vec!["rust".to_string(), "web".to_string()],
            suggested_folder_path: vec!["Programming".to_string(), "Rust".to_string()],
            combined_description: Some("A great description".to_string()),
        };

        assert_eq!(metadata.best_title, "Best Title");
        assert_eq!(metadata.combined_keywords.len(), 2);
        assert_eq!(metadata.suggested_folder_path.len(), 2);
        assert!(metadata.combined_description.is_some());
    }

    #[tokio::test]
    async fn test_analyze_batch_empty() {
        let processor = BatchBookmarkProcessor::new();
        let result = processor.analyze_batch(&[]).await;
        
        assert_eq!(result.total_bookmarks, 0);
        assert_eq!(result.unique_bookmarks, 0);
        assert_eq!(result.duplicate_groups_count, 0);
        assert!(result.duplicate_groups.is_empty());
        assert!(result.merge_suggestions.is_empty());
    }

    #[tokio::test]
    async fn test_analyze_batch_no_duplicates() {
        let processor = BatchBookmarkProcessor::new();
        let bookmarks = vec![
            create_test_bookmark("https://example1.com", "Example 1"),
            create_test_bookmark("https://example2.com", "Example 2"),
            create_test_bookmark("https://example3.com", "Example 3"),
        ];
        
        let result = processor.analyze_batch(&bookmarks).await;
        
        assert_eq!(result.total_bookmarks, 3);
        // All bookmarks should be unique when URLs are different
        assert_eq!(result.bookmark_results.len(), 3);
    }

    #[tokio::test]
    async fn test_analyze_batch_with_exact_duplicates() {
        let processor = BatchBookmarkProcessor::new();
        let bookmarks = vec![
            create_test_bookmark("https://example.com", "Example 1"),
            create_test_bookmark("https://example.com", "Example 2"),
            create_test_bookmark("https://other.com", "Other"),
        ];
        
        let result = processor.analyze_batch(&bookmarks).await;
        
        assert_eq!(result.total_bookmarks, 3);
        assert!(result.duplicate_groups_count >= 1);
        
        // Should have at least one duplicate group for the example.com URLs
        let exact_url_groups: Vec<_> = result.duplicate_groups
            .iter()
            .filter(|g| matches!(g.duplicate_type, DuplicateType::ExactUrl))
            .collect();
        assert!(!exact_url_groups.is_empty());
    }
}
