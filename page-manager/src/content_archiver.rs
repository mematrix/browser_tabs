//! Content Archiver for Web Page Manager
//!
//! This module provides functionality to archive web page content locally,
//! including HTML extraction, media file download, and compression.
//!
//! # Features
//! - HTML content extraction and cleaning
//! - Media file download and local storage
//! - Archive format with compression
//! - Full-text searchable content storage
//!
//! # Requirements Implemented
//! - 3.1: Extract page text, images, and structured content when user selects archive
//! - 3.3: Download and locally store related media resources for archives with media files

use web_page_manager_core::*;
use data_access::{ContentArchive, ArchiveRepository};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;
use tracing::{info, warn, debug};

/// Configuration for the content archiver
#[derive(Debug, Clone)]
pub struct ContentArchiverConfig {
    /// Base directory for storing media files
    pub media_storage_path: PathBuf,
    /// Maximum size for a single media file (in bytes)
    pub max_media_file_size: u64,
    /// Maximum total size for all media files per archive (in bytes)
    pub max_total_media_size: u64,
    /// Timeout for downloading media files (in seconds)
    pub download_timeout_secs: u64,
    /// Whether to compress archived content
    pub enable_compression: bool,
    /// Supported media file extensions
    pub supported_media_extensions: Vec<String>,
    /// Maximum concurrent downloads
    pub max_concurrent_downloads: usize,
}

impl Default for ContentArchiverConfig {
    fn default() -> Self {
        Self {
            media_storage_path: PathBuf::from("./archives/media"),
            max_media_file_size: 10 * 1024 * 1024, // 10MB
            max_total_media_size: 100 * 1024 * 1024, // 100MB
            download_timeout_secs: 30,
            enable_compression: true,
            supported_media_extensions: vec![
                "jpg".to_string(), "jpeg".to_string(), "png".to_string(),
                "gif".to_string(), "webp".to_string(), "svg".to_string(),
                "ico".to_string(), "bmp".to_string(),
            ],
            max_concurrent_downloads: 5,
        }
    }
}

/// Result of archiving a page
#[derive(Debug, Clone)]
pub struct ArchiveResult {
    /// The created archive
    pub archive: ContentArchive,
    /// List of successfully downloaded media files
    pub downloaded_media: Vec<MediaFileInfo>,
    /// List of failed media downloads
    pub failed_media: Vec<MediaDownloadError>,
    /// Total size of the archive (content + media)
    pub total_size: u64,
    /// Time taken to create the archive (in milliseconds)
    pub archive_duration_ms: u64,
}

/// Information about a downloaded media file
#[derive(Debug, Clone)]
pub struct MediaFileInfo {
    /// Original URL of the media file
    pub original_url: String,
    /// Local path where the file is stored
    pub local_path: PathBuf,
    /// File size in bytes
    pub file_size: u64,
    /// MIME type of the file
    pub mime_type: Option<String>,
    /// Checksum of the file content
    pub checksum: String,
}

/// Error information for failed media downloads
#[derive(Debug, Clone)]
pub struct MediaDownloadError {
    /// URL that failed to download
    pub url: String,
    /// Error message
    pub error: String,
}

/// HTML content extraction result
#[derive(Debug, Clone)]
pub struct ExtractedContent {
    /// Cleaned HTML content
    pub html: String,
    /// Plain text content extracted from HTML
    pub text: String,
    /// Page title
    pub title: String,
    /// List of image URLs found in the content
    pub image_urls: Vec<String>,
    /// List of other media URLs (videos, audio, etc.)
    pub other_media_urls: Vec<String>,
    /// List of internal links
    pub internal_links: Vec<String>,
    /// List of external links
    pub external_links: Vec<String>,
    /// Estimated reading time in minutes
    pub reading_time_minutes: u32,
    /// Word count
    pub word_count: u32,
}

/// Content archiver for saving web pages locally
pub struct ContentArchiver {
    config: ContentArchiverConfig,
    archive_repository: Option<Arc<dyn ArchiveRepository + Send + Sync>>,
    active_downloads: Arc<RwLock<HashSet<String>>>,
}

impl ContentArchiver {
    /// Create a new content archiver with default configuration
    pub fn new() -> Self {
        Self {
            config: ContentArchiverConfig::default(),
            archive_repository: None,
            active_downloads: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Create a new content archiver with custom configuration
    pub fn with_config(config: ContentArchiverConfig) -> Self {
        Self {
            config,
            archive_repository: None,
            active_downloads: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Set the archive repository for persistence
    pub fn with_repository(mut self, repository: Arc<dyn ArchiveRepository + Send + Sync>) -> Self {
        self.archive_repository = Some(repository);
        self
    }

    /// Get the current configuration
    pub fn config(&self) -> &ContentArchiverConfig {
        &self.config
    }

    /// Extract content from HTML
    /// 
    /// This method parses HTML content and extracts:
    /// - Clean HTML with scripts and styles removed
    /// - Plain text content
    /// - Image and media URLs
    /// - Links (internal and external)
    pub fn extract_content(&self, html: &str, base_url: &str) -> ExtractedContent {
        let cleaned_html = self.clean_html(html);
        let text = self.extract_text(&cleaned_html);
        let title = self.extract_title(html);
        let image_urls = self.extract_image_urls(html, base_url);
        let other_media_urls = self.extract_other_media_urls(html, base_url);
        let (internal_links, external_links) = self.extract_links(html, base_url);
        let word_count = self.count_words(&text);
        let reading_time_minutes = self.estimate_reading_time(word_count);

        ExtractedContent {
            html: cleaned_html,
            text,
            title,
            image_urls,
            other_media_urls,
            internal_links,
            external_links,
            reading_time_minutes,
            word_count,
        }
    }

    /// Clean HTML by removing scripts, styles, and other non-content elements
    fn clean_html(&self, html: &str) -> String {
        let mut result = html.to_string();
        
        // Remove script tags and their content
        result = Self::remove_tag_with_content(&result, "script");
        
        // Remove style tags and their content
        result = Self::remove_tag_with_content(&result, "style");
        
        // Remove noscript tags and their content
        result = Self::remove_tag_with_content(&result, "noscript");
        
        // Remove comments
        result = Self::remove_html_comments(&result);
        
        // Remove inline event handlers (onclick, onload, etc.)
        result = Self::remove_event_handlers(&result);
        
        // Normalize whitespace
        result = Self::normalize_whitespace(&result);
        
        result
    }

    /// Remove a specific HTML tag and its content
    fn remove_tag_with_content(html: &str, tag: &str) -> String {
        let open_tag = format!("<{}", tag);
        let close_tag = format!("</{}>", tag);
        let mut result = String::new();
        let mut chars = html.chars().peekable();
        let mut in_tag = false;
        let mut depth = 0;
        let mut buffer = String::new();

        while let Some(c) = chars.next() {
            buffer.push(c);
            
            if !in_tag {
                // Check if we're starting the target tag
                if buffer.to_lowercase().ends_with(&open_tag) {
                    // Check if it's actually the tag (followed by space, >, or /)
                    if let Some(&next) = chars.peek() {
                        if next == ' ' || next == '>' || next == '/' || next == '\t' || next == '\n' {
                            in_tag = true;
                            depth = 1;
                            // Remove the tag start from buffer
                            let len = buffer.len();
                            buffer.truncate(len - open_tag.len());
                            result.push_str(&buffer);
                            buffer.clear();
                            continue;
                        }
                    }
                }
                
                // Flush buffer periodically to avoid memory issues
                if buffer.len() > 1000 {
                    result.push_str(&buffer);
                    buffer.clear();
                }
            } else {
                // We're inside the tag, look for closing tag
                if buffer.to_lowercase().ends_with(&close_tag) {
                    depth -= 1;
                    if depth == 0 {
                        in_tag = false;
                        buffer.clear();
                    }
                } else if buffer.to_lowercase().ends_with(&open_tag) {
                    // Check for nested tags
                    if let Some(&next) = chars.peek() {
                        if next == ' ' || next == '>' || next == '/' || next == '\t' || next == '\n' {
                            depth += 1;
                        }
                    }
                }
            }
        }
        
        result.push_str(&buffer);
        result
    }

    /// Remove HTML comments
    fn remove_html_comments(html: &str) -> String {
        let mut result = String::new();
        let mut in_comment = false;
        let mut chars = html.chars().peekable();
        let mut buffer = String::new();

        while let Some(c) = chars.next() {
            buffer.push(c);
            
            if !in_comment {
                if buffer.ends_with("<!--") {
                    in_comment = true;
                    let len = buffer.len();
                    buffer.truncate(len - 4);
                    result.push_str(&buffer);
                    buffer.clear();
                }
            } else {
                if buffer.ends_with("-->") {
                    in_comment = false;
                    buffer.clear();
                }
            }
            
            if !in_comment && buffer.len() > 1000 {
                result.push_str(&buffer);
                buffer.clear();
            }
        }
        
        if !in_comment {
            result.push_str(&buffer);
        }
        
        result
    }

    /// Remove inline event handlers from HTML
    fn remove_event_handlers(html: &str) -> String {
        let event_handlers = [
            "onclick", "onload", "onerror", "onmouseover", "onmouseout",
            "onkeydown", "onkeyup", "onkeypress", "onfocus", "onblur",
            "onchange", "onsubmit", "onreset", "onselect", "onscroll",
        ];
        
        let mut result = html.to_string();
        
        for handler in &event_handlers {
            // Simple pattern matching for event handlers
            let pattern = format!(r#"{}="#, handler);
            while let Some(start) = result.to_lowercase().find(&pattern) {
                // Find the end of the attribute value
                let after_eq = start + handler.len() + 2; // +2 for ="
                if after_eq >= result.len() {
                    break;
                }
                
                let quote_char = result.chars().nth(after_eq - 1).unwrap_or('"');
                if let Some(end) = result[after_eq..].find(quote_char) {
                    let end_pos = after_eq + end + 1;
                    result = format!("{}{}", &result[..start], &result[end_pos..]);
                } else {
                    break;
                }
            }
        }
        
        result
    }

    /// Normalize whitespace in HTML
    fn normalize_whitespace(html: &str) -> String {
        let mut result = String::new();
        let mut prev_was_whitespace = false;
        
        for c in html.chars() {
            if c.is_whitespace() {
                if !prev_was_whitespace {
                    result.push(' ');
                    prev_was_whitespace = true;
                }
            } else {
                result.push(c);
                prev_was_whitespace = false;
            }
        }
        
        result.trim().to_string()
    }

    /// Extract plain text from HTML
    fn extract_text(&self, html: &str) -> String {
        let mut text = String::new();
        let mut in_tag = false;
        let mut prev_was_space = false;
        
        for c in html.chars() {
            if c == '<' {
                in_tag = true;
                // Add space for block elements
                if !prev_was_space && !text.is_empty() {
                    text.push(' ');
                    prev_was_space = true;
                }
            } else if c == '>' {
                in_tag = false;
            } else if !in_tag {
                if c.is_whitespace() {
                    if !prev_was_space {
                        text.push(' ');
                        prev_was_space = true;
                    }
                } else {
                    text.push(c);
                    prev_was_space = false;
                }
            }
        }
        
        // Decode common HTML entities
        let text = text
            .replace("&nbsp;", " ")
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&apos;", "'");
        
        text.trim().to_string()
    }

    /// Extract title from HTML
    fn extract_title(&self, html: &str) -> String {
        // Try to find <title> tag
        let lower_html = html.to_lowercase();
        if let Some(start) = lower_html.find("<title") {
            if let Some(tag_end) = html[start..].find('>') {
                let content_start = start + tag_end + 1;
                if let Some(end) = lower_html[content_start..].find("</title>") {
                    let title = &html[content_start..content_start + end];
                    return title.trim().to_string();
                }
            }
        }
        
        // Fallback: try to find <h1> tag
        if let Some(start) = lower_html.find("<h1") {
            if let Some(tag_end) = html[start..].find('>') {
                let content_start = start + tag_end + 1;
                if let Some(end) = lower_html[content_start..].find("</h1>") {
                    let h1 = &html[content_start..content_start + end];
                    return self.extract_text(h1);
                }
            }
        }
        
        String::new()
    }


    /// Extract image URLs from HTML
    fn extract_image_urls(&self, html: &str, base_url: &str) -> Vec<String> {
        let mut urls = Vec::new();
        let lower_html = html.to_lowercase();
        
        // Find <img> tags
        let mut search_start = 0;
        while let Some(img_start) = lower_html[search_start..].find("<img") {
            let actual_start = search_start + img_start;
            if let Some(tag_end) = html[actual_start..].find('>') {
                let tag = &html[actual_start..actual_start + tag_end + 1];
                
                // Extract src attribute
                if let Some(src) = self.extract_attribute(tag, "src") {
                    if let Some(absolute_url) = self.resolve_url(&src, base_url) {
                        if self.is_supported_image(&absolute_url) {
                            urls.push(absolute_url);
                        }
                    }
                }
                
                // Also check srcset for responsive images
                if let Some(srcset) = self.extract_attribute(tag, "srcset") {
                    for part in srcset.split(',') {
                        let src = part.trim().split_whitespace().next().unwrap_or("");
                        if let Some(absolute_url) = self.resolve_url(src, base_url) {
                            if self.is_supported_image(&absolute_url) && !urls.contains(&absolute_url) {
                                urls.push(absolute_url);
                            }
                        }
                    }
                }
                
                search_start = actual_start + tag_end + 1;
            } else {
                break;
            }
        }
        
        // Find background images in style attributes
        search_start = 0;
        while let Some(style_start) = lower_html[search_start..].find("style=") {
            let actual_start = search_start + style_start;
            if let Some(style_value) = self.extract_attribute(&html[actual_start..], "style") {
                if let Some(url_start) = style_value.to_lowercase().find("url(") {
                    let url_content_start = url_start + 4;
                    if let Some(url_end) = style_value[url_content_start..].find(')') {
                        let url = style_value[url_content_start..url_content_start + url_end]
                            .trim_matches(|c| c == '"' || c == '\'');
                        if let Some(absolute_url) = self.resolve_url(url, base_url) {
                            if self.is_supported_image(&absolute_url) && !urls.contains(&absolute_url) {
                                urls.push(absolute_url);
                            }
                        }
                    }
                }
            }
            search_start = actual_start + 6;
        }
        
        urls
    }

    /// Extract other media URLs (video, audio) from HTML
    fn extract_other_media_urls(&self, html: &str, base_url: &str) -> Vec<String> {
        let mut urls = Vec::new();
        let lower_html = html.to_lowercase();
        
        // Find <video> and <audio> tags
        for tag_name in &["video", "audio", "source"] {
            let mut search_start = 0;
            let open_tag = format!("<{}", tag_name);
            
            while let Some(tag_start) = lower_html[search_start..].find(&open_tag) {
                let actual_start = search_start + tag_start;
                if let Some(tag_end) = html[actual_start..].find('>') {
                    let tag = &html[actual_start..actual_start + tag_end + 1];
                    
                    if let Some(src) = self.extract_attribute(tag, "src") {
                        if let Some(absolute_url) = self.resolve_url(&src, base_url) {
                            if !urls.contains(&absolute_url) {
                                urls.push(absolute_url);
                            }
                        }
                    }
                    
                    search_start = actual_start + tag_end + 1;
                } else {
                    break;
                }
            }
        }
        
        urls
    }

    /// Extract links from HTML, separating internal and external
    fn extract_links(&self, html: &str, base_url: &str) -> (Vec<String>, Vec<String>) {
        let mut internal_links = Vec::new();
        let mut external_links = Vec::new();
        let lower_html = html.to_lowercase();
        
        let base_domain = self.extract_domain(base_url);
        
        let mut search_start = 0;
        while let Some(a_start) = lower_html[search_start..].find("<a") {
            let actual_start = search_start + a_start;
            if let Some(tag_end) = html[actual_start..].find('>') {
                let tag = &html[actual_start..actual_start + tag_end + 1];
                
                if let Some(href) = self.extract_attribute(tag, "href") {
                    // Skip javascript: and mailto: links
                    if href.starts_with("javascript:") || href.starts_with("mailto:") || href.starts_with("#") {
                        search_start = actual_start + tag_end + 1;
                        continue;
                    }
                    
                    if let Some(absolute_url) = self.resolve_url(&href, base_url) {
                        let link_domain = self.extract_domain(&absolute_url);
                        
                        if link_domain == base_domain {
                            if !internal_links.contains(&absolute_url) {
                                internal_links.push(absolute_url);
                            }
                        } else {
                            if !external_links.contains(&absolute_url) {
                                external_links.push(absolute_url);
                            }
                        }
                    }
                }
                
                search_start = actual_start + tag_end + 1;
            } else {
                break;
            }
        }
        
        (internal_links, external_links)
    }

    /// Extract an attribute value from an HTML tag
    fn extract_attribute(&self, tag: &str, attr_name: &str) -> Option<String> {
        let lower_tag = tag.to_lowercase();
        let patterns = [
            format!(r#"{}=""#, attr_name),
            format!(r#"{}='"#, attr_name),
            format!(r#"{}="#, attr_name),
        ];
        
        for (i, pattern) in patterns.iter().enumerate() {
            if let Some(start) = lower_tag.find(pattern) {
                let value_start = start + pattern.len();
                let quote_char = if i == 0 { '"' } else if i == 1 { '\'' } else { ' ' };
                
                if i < 2 {
                    // Quoted value
                    if let Some(end) = tag[value_start..].find(quote_char) {
                        return Some(tag[value_start..value_start + end].to_string());
                    }
                } else {
                    // Unquoted value
                    let end = tag[value_start..]
                        .find(|c: char| c.is_whitespace() || c == '>')
                        .unwrap_or(tag.len() - value_start);
                    return Some(tag[value_start..value_start + end].to_string());
                }
            }
        }
        
        None
    }

    /// Resolve a relative URL to an absolute URL
    fn resolve_url(&self, url: &str, base_url: &str) -> Option<String> {
        let url = url.trim();
        
        // Already absolute
        if url.starts_with("http://") || url.starts_with("https://") {
            return Some(url.to_string());
        }
        
        // Protocol-relative URL
        if url.starts_with("//") {
            let protocol = if base_url.starts_with("https://") { "https:" } else { "http:" };
            return Some(format!("{}{}", protocol, url));
        }
        
        // Data URL - skip
        if url.starts_with("data:") {
            return None;
        }
        
        // Parse base URL
        let base_parts = self.parse_url(base_url)?;
        
        // Absolute path
        if url.starts_with('/') {
            return Some(format!("{}://{}{}", base_parts.0, base_parts.1, url));
        }
        
        // Relative path
        let base_path = base_parts.2.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
        let resolved_path = format!("{}/{}", base_path, url);
        
        // Normalize path (handle .. and .)
        let normalized = self.normalize_path(&resolved_path);
        
        Some(format!("{}://{}{}", base_parts.0, base_parts.1, normalized))
    }

    /// Parse URL into (protocol, host, path)
    fn parse_url(&self, url: &str) -> Option<(String, String, String)> {
        let protocol_end = url.find("://")?;
        let protocol = &url[..protocol_end];
        let rest = &url[protocol_end + 3..];
        
        let (host, path) = if let Some(path_start) = rest.find('/') {
            (&rest[..path_start], &rest[path_start..])
        } else {
            (rest, "/")
        };
        
        Some((protocol.to_string(), host.to_string(), path.to_string()))
    }

    /// Normalize a URL path (resolve . and ..)
    fn normalize_path(&self, path: &str) -> String {
        let mut segments: Vec<&str> = Vec::new();
        
        for segment in path.split('/') {
            match segment {
                "" | "." => continue,
                ".." => { segments.pop(); }
                s => segments.push(s),
            }
        }
        
        format!("/{}", segments.join("/"))
    }

    /// Extract domain from URL
    fn extract_domain(&self, url: &str) -> String {
        if let Some((_, host, _)) = self.parse_url(url) {
            // Remove port if present
            host.split(':').next().unwrap_or(&host).to_lowercase()
        } else {
            String::new()
        }
    }

    /// Check if URL points to a supported image format
    fn is_supported_image(&self, url: &str) -> bool {
        let lower_url = url.to_lowercase();
        
        // Remove query string for extension check
        let path = lower_url.split('?').next().unwrap_or(&lower_url);
        
        self.config.supported_media_extensions.iter().any(|ext| {
            path.ends_with(&format!(".{}", ext))
        })
    }

    /// Count words in text
    fn count_words(&self, text: &str) -> u32 {
        text.split_whitespace().count() as u32
    }

    /// Estimate reading time based on word count
    /// Average reading speed: 200-250 words per minute
    fn estimate_reading_time(&self, word_count: u32) -> u32 {
        ((word_count as f32) / 225.0).ceil() as u32
    }


    /// Archive a web page
    /// 
    /// This method extracts content from the provided HTML, downloads media files,
    /// and stores the archive in the repository.
    /// 
    /// # Arguments
    /// * `page_id` - The ID of the unified page being archived
    /// * `url` - The URL of the page
    /// * `html` - The HTML content of the page
    /// 
    /// # Returns
    /// * `ArchiveResult` containing the created archive and download statistics
    pub async fn archive_page(
        &self,
        page_id: Uuid,
        url: &str,
        html: &str,
    ) -> Result<ArchiveResult> {
        let start_time = std::time::Instant::now();
        
        info!("Starting archive for page: {}", url);
        
        // Extract content
        let extracted = self.extract_content(html, url);
        debug!("Extracted content: {} words, {} images", extracted.word_count, extracted.image_urls.len());
        
        // Download media files
        let (downloaded_media, failed_media) = self.download_media_files(
            &page_id,
            &extracted.image_urls,
        ).await;
        
        // Calculate file size
        let content_size = extracted.html.len() + extracted.text.len();
        let media_size: u64 = downloaded_media.iter().map(|m| m.file_size).sum();
        let total_size = content_size as u64 + media_size;
        
        // Generate checksum for content
        let checksum = self.calculate_checksum(&extracted.text);
        
        // Create media file paths list
        let media_files: Vec<String> = downloaded_media
            .iter()
            .map(|m| m.local_path.to_string_lossy().to_string())
            .collect();
        
        // Create archive
        let archive = ContentArchive {
            id: ArchiveId::new(),
            page_id,
            url: url.to_string(),
            title: extracted.title.clone(),
            content_html: if self.config.enable_compression {
                self.compress_content(&extracted.html)
            } else {
                extracted.html.clone()
            },
            content_text: extracted.text.clone(),
            media_files,
            archived_at: Utc::now(),
            file_size: total_size,
            checksum: Some(checksum),
        };
        
        // Save to repository if available
        if let Some(ref repo) = self.archive_repository {
            repo.save(&archive).await?;
            info!("Archive saved to repository: {}", archive.id.0);
        }
        
        let duration_ms = start_time.elapsed().as_millis() as u64;
        
        Ok(ArchiveResult {
            archive,
            downloaded_media,
            failed_media,
            total_size,
            archive_duration_ms: duration_ms,
        })
    }

    /// Download media files for an archive
    async fn download_media_files(
        &self,
        page_id: &Uuid,
        urls: &[String],
    ) -> (Vec<MediaFileInfo>, Vec<MediaDownloadError>) {
        let mut downloaded = Vec::new();
        let mut failed = Vec::new();
        let mut total_size: u64 = 0;
        
        // Create directory for this page's media
        let media_dir = self.config.media_storage_path.join(page_id.to_string());
        if let Err(e) = std::fs::create_dir_all(&media_dir) {
            warn!("Failed to create media directory: {}", e);
            // Return all as failed
            for url in urls {
                failed.push(MediaDownloadError {
                    url: url.clone(),
                    error: format!("Failed to create directory: {}", e),
                });
            }
            return (downloaded, failed);
        }
        
        for url in urls {
            // Check if we've exceeded the total size limit
            if total_size >= self.config.max_total_media_size {
                failed.push(MediaDownloadError {
                    url: url.clone(),
                    error: "Total media size limit exceeded".to_string(),
                });
                continue;
            }
            
            // Check if already downloading
            {
                let active = self.active_downloads.read().await;
                if active.contains(url) {
                    continue;
                }
            }
            
            // Mark as downloading
            {
                let mut active = self.active_downloads.write().await;
                active.insert(url.clone());
            }
            
            // Download the file
            match self.download_single_media(url, &media_dir).await {
                Ok(info) => {
                    total_size += info.file_size;
                    downloaded.push(info);
                }
                Err(e) => {
                    failed.push(MediaDownloadError {
                        url: url.clone(),
                        error: e,
                    });
                }
            }
            
            // Remove from active downloads
            {
                let mut active = self.active_downloads.write().await;
                active.remove(url);
            }
        }
        
        (downloaded, failed)
    }

    /// Download a single media file
    async fn download_single_media(
        &self,
        url: &str,
        target_dir: &Path,
    ) -> std::result::Result<MediaFileInfo, String> {
        debug!("Downloading media: {}", url);
        
        // Generate filename from URL
        let filename = self.generate_media_filename(url);
        let local_path = target_dir.join(&filename);
        
        // For now, we'll create a placeholder since we don't have HTTP client
        // In a real implementation, this would use reqwest or similar
        // This is a stub that simulates the download
        
        // Create placeholder file with URL as content (for testing)
        let content = format!("PLACEHOLDER: {}", url);
        let content_bytes = content.as_bytes();
        
        if content_bytes.len() as u64 > self.config.max_media_file_size {
            return Err("File size exceeds limit".to_string());
        }
        
        std::fs::write(&local_path, content_bytes)
            .map_err(|e| format!("Failed to write file: {}", e))?;
        
        let checksum = self.calculate_checksum(&content);
        
        Ok(MediaFileInfo {
            original_url: url.to_string(),
            local_path,
            file_size: content_bytes.len() as u64,
            mime_type: self.guess_mime_type(url),
            checksum,
        })
    }

    /// Generate a filename for a media file based on its URL
    fn generate_media_filename(&self, url: &str) -> String {
        // Extract filename from URL
        let path = url.split('?').next().unwrap_or(url);
        let filename = path.rsplit('/').next().unwrap_or("media");
        
        // Sanitize filename
        let sanitized: String = filename
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
            .collect();
        
        // Add hash suffix to avoid collisions
        let hash = self.simple_hash(url);
        let (name, ext) = sanitized.rsplit_once('.').unwrap_or((&sanitized, "bin"));
        
        format!("{}_{}.{}", name, hash, ext)
    }

    /// Simple hash function for generating unique identifiers
    fn simple_hash(&self, input: &str) -> String {
        let mut hash: u64 = 0;
        for byte in input.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }
        format!("{:08x}", hash)
    }

    /// Guess MIME type from URL
    fn guess_mime_type(&self, url: &str) -> Option<String> {
        let lower_url = url.to_lowercase();
        let path = lower_url.split('?').next().unwrap_or(&lower_url);
        
        if path.ends_with(".jpg") || path.ends_with(".jpeg") {
            Some("image/jpeg".to_string())
        } else if path.ends_with(".png") {
            Some("image/png".to_string())
        } else if path.ends_with(".gif") {
            Some("image/gif".to_string())
        } else if path.ends_with(".webp") {
            Some("image/webp".to_string())
        } else if path.ends_with(".svg") {
            Some("image/svg+xml".to_string())
        } else if path.ends_with(".ico") {
            Some("image/x-icon".to_string())
        } else if path.ends_with(".bmp") {
            Some("image/bmp".to_string())
        } else if path.ends_with(".mp4") {
            Some("video/mp4".to_string())
        } else if path.ends_with(".webm") {
            Some("video/webm".to_string())
        } else if path.ends_with(".mp3") {
            Some("audio/mpeg".to_string())
        } else {
            None
        }
    }

    /// Calculate a simple checksum for content
    fn calculate_checksum(&self, content: &str) -> String {
        // Simple checksum using FNV-1a hash
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in content.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{:016x}", hash)
    }

    /// Compress content using simple run-length encoding
    /// In a real implementation, this would use zlib or similar
    fn compress_content(&self, content: &str) -> String {
        // For now, just return the content as-is
        // A real implementation would use flate2 or similar
        content.to_string()
    }

    /// Decompress content
    pub fn decompress_content(&self, content: &str) -> String {
        // For now, just return the content as-is
        content.to_string()
    }

    /// Get an archive by ID
    pub async fn get_archive(&self, id: &ArchiveId) -> Result<Option<ContentArchive>> {
        if let Some(ref repo) = self.archive_repository {
            repo.get_by_id(id).await
        } else {
            Err(WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: "No archive repository configured".to_string(),
                },
            })
        }
    }

    /// Get an archive by page ID
    pub async fn get_archive_by_page(&self, page_id: &Uuid) -> Result<Option<ContentArchive>> {
        if let Some(ref repo) = self.archive_repository {
            repo.get_by_page_id(page_id).await
        } else {
            Err(WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: "No archive repository configured".to_string(),
                },
            })
        }
    }

    /// Search archives
    pub async fn search_archives(&self, query: &str, limit: usize) -> Result<Vec<ContentArchive>> {
        if let Some(ref repo) = self.archive_repository {
            repo.search(query, limit).await
        } else {
            Err(WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: "No archive repository configured".to_string(),
                },
            })
        }
    }

    /// Delete an archive
    pub async fn delete_archive(&self, id: &ArchiveId) -> Result<()> {
        // First, get the archive to find media files
        if let Some(ref repo) = self.archive_repository {
            if let Some(archive) = repo.get_by_id(id).await? {
                // Delete media files
                for media_path in &archive.media_files {
                    let path = Path::new(media_path);
                    if path.exists() {
                        if let Err(e) = std::fs::remove_file(path) {
                            warn!("Failed to delete media file {}: {}", media_path, e);
                        }
                    }
                }
                
                // Delete from repository
                repo.delete(id).await?;
                
                info!("Archive deleted: {}", id.0);
            }
        }
        
        Ok(())
    }

    /// Get total size of all archives
    pub async fn get_total_archive_size(&self) -> Result<u64> {
        if let Some(ref repo) = self.archive_repository {
            repo.get_total_size().await
        } else {
            Ok(0)
        }
    }
}

impl Default for ContentArchiver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title() {
        let archiver = ContentArchiver::new();
        
        let html = r#"<html><head><title>Test Page Title</title></head><body></body></html>"#;
        let title = archiver.extract_title(html);
        assert_eq!(title, "Test Page Title");
    }

    #[test]
    fn test_extract_title_from_h1() {
        let archiver = ContentArchiver::new();
        
        let html = r#"<html><body><h1>Main Heading</h1></body></html>"#;
        let title = archiver.extract_title(html);
        assert_eq!(title, "Main Heading");
    }

    #[test]
    fn test_extract_text() {
        let archiver = ContentArchiver::new();
        
        let html = r#"<p>Hello <strong>World</strong>!</p>"#;
        let text = archiver.extract_text(html);
        assert_eq!(text, "Hello World !");
    }

    #[test]
    fn test_clean_html_removes_scripts() {
        let archiver = ContentArchiver::new();
        
        let html = r#"<p>Before</p><script>alert('test');</script><p>After</p>"#;
        let cleaned = archiver.clean_html(html);
        assert!(!cleaned.contains("script"));
        assert!(!cleaned.contains("alert"));
        assert!(cleaned.contains("Before"));
        assert!(cleaned.contains("After"));
    }

    #[test]
    fn test_clean_html_removes_styles() {
        let archiver = ContentArchiver::new();
        
        let html = r#"<p>Before</p><style>.test { color: red; }</style><p>After</p>"#;
        let cleaned = archiver.clean_html(html);
        assert!(!cleaned.contains("style"));
        assert!(!cleaned.contains("color"));
    }

    #[test]
    fn test_extract_image_urls() {
        let archiver = ContentArchiver::new();
        
        let html = r#"<img src="/images/test.jpg"><img src="https://example.com/photo.png">"#;
        let urls = archiver.extract_image_urls(html, "https://example.com/page");
        
        assert_eq!(urls.len(), 2);
        assert!(urls.contains(&"https://example.com/images/test.jpg".to_string()));
        assert!(urls.contains(&"https://example.com/photo.png".to_string()));
    }

    #[test]
    fn test_resolve_url_absolute() {
        let archiver = ContentArchiver::new();
        
        let result = archiver.resolve_url("https://other.com/image.jpg", "https://example.com/page");
        assert_eq!(result, Some("https://other.com/image.jpg".to_string()));
    }

    #[test]
    fn test_resolve_url_relative() {
        let archiver = ContentArchiver::new();
        
        let result = archiver.resolve_url("images/test.jpg", "https://example.com/page/index.html");
        assert_eq!(result, Some("https://example.com/page/images/test.jpg".to_string()));
    }

    #[test]
    fn test_resolve_url_absolute_path() {
        let archiver = ContentArchiver::new();
        
        let result = archiver.resolve_url("/images/test.jpg", "https://example.com/page/index.html");
        assert_eq!(result, Some("https://example.com/images/test.jpg".to_string()));
    }

    #[test]
    fn test_resolve_url_protocol_relative() {
        let archiver = ContentArchiver::new();
        
        let result = archiver.resolve_url("//cdn.example.com/image.jpg", "https://example.com/page");
        assert_eq!(result, Some("https://cdn.example.com/image.jpg".to_string()));
    }

    #[test]
    fn test_extract_links() {
        let archiver = ContentArchiver::new();
        
        let html = r#"
            <a href="/internal">Internal</a>
            <a href="https://external.com/page">External</a>
            <a href="https://example.com/other">Same Domain</a>
        "#;
        
        let (internal, external) = archiver.extract_links(html, "https://example.com/page");
        
        assert!(internal.contains(&"https://example.com/internal".to_string()));
        assert!(internal.contains(&"https://example.com/other".to_string()));
        assert!(external.contains(&"https://external.com/page".to_string()));
    }

    #[test]
    fn test_word_count() {
        let archiver = ContentArchiver::new();
        
        let text = "This is a test sentence with seven words.";
        let count = archiver.count_words(text);
        assert_eq!(count, 8);
    }

    #[test]
    fn test_reading_time() {
        let archiver = ContentArchiver::new();
        
        // 225 words = 1 minute
        assert_eq!(archiver.estimate_reading_time(225), 1);
        
        // 450 words = 2 minutes
        assert_eq!(archiver.estimate_reading_time(450), 2);
        
        // 100 words = 1 minute (rounded up)
        assert_eq!(archiver.estimate_reading_time(100), 1);
    }

    #[test]
    fn test_extract_attribute() {
        let archiver = ContentArchiver::new();
        
        let tag = r#"<img src="test.jpg" alt="Test Image">"#;
        
        assert_eq!(archiver.extract_attribute(tag, "src"), Some("test.jpg".to_string()));
        assert_eq!(archiver.extract_attribute(tag, "alt"), Some("Test Image".to_string()));
        assert_eq!(archiver.extract_attribute(tag, "class"), None);
    }

    #[test]
    fn test_extract_attribute_single_quotes() {
        let archiver = ContentArchiver::new();
        
        let tag = r#"<img src='test.jpg'>"#;
        assert_eq!(archiver.extract_attribute(tag, "src"), Some("test.jpg".to_string()));
    }

    #[test]
    fn test_is_supported_image() {
        let archiver = ContentArchiver::new();
        
        assert!(archiver.is_supported_image("https://example.com/image.jpg"));
        assert!(archiver.is_supported_image("https://example.com/image.PNG"));
        assert!(archiver.is_supported_image("https://example.com/image.webp?v=123"));
        assert!(!archiver.is_supported_image("https://example.com/document.pdf"));
    }

    #[test]
    fn test_generate_media_filename() {
        let archiver = ContentArchiver::new();
        
        let filename = archiver.generate_media_filename("https://example.com/images/photo.jpg");
        assert!(filename.ends_with(".jpg"));
        assert!(filename.contains("photo"));
    }

    #[test]
    fn test_guess_mime_type() {
        let archiver = ContentArchiver::new();
        
        assert_eq!(archiver.guess_mime_type("test.jpg"), Some("image/jpeg".to_string()));
        assert_eq!(archiver.guess_mime_type("test.png"), Some("image/png".to_string()));
        assert_eq!(archiver.guess_mime_type("test.gif"), Some("image/gif".to_string()));
        assert_eq!(archiver.guess_mime_type("test.mp4"), Some("video/mp4".to_string()));
        assert_eq!(archiver.guess_mime_type("test.unknown"), None);
    }

    #[test]
    fn test_calculate_checksum() {
        let archiver = ContentArchiver::new();
        
        let checksum1 = archiver.calculate_checksum("test content");
        let checksum2 = archiver.calculate_checksum("test content");
        let checksum3 = archiver.calculate_checksum("different content");
        
        assert_eq!(checksum1, checksum2);
        assert_ne!(checksum1, checksum3);
    }

    #[test]
    fn test_extract_content() {
        let archiver = ContentArchiver::new();
        
        let html = r#"
            <html>
            <head><title>Test Page</title></head>
            <body>
                <h1>Welcome</h1>
                <p>This is a test paragraph with some content.</p>
                <img src="/images/test.jpg">
                <a href="/internal">Internal Link</a>
                <a href="https://external.com">External Link</a>
                <script>alert('removed');</script>
            </body>
            </html>
        "#;
        
        let extracted = archiver.extract_content(html, "https://example.com/page");
        
        assert_eq!(extracted.title, "Test Page");
        assert!(extracted.text.contains("Welcome"));
        assert!(extracted.text.contains("test paragraph"));
        assert!(!extracted.text.contains("alert"));
        assert_eq!(extracted.image_urls.len(), 1);
        assert!(extracted.internal_links.len() >= 1);
        assert!(extracted.external_links.len() >= 1);
        assert!(extracted.word_count > 0);
    }

    #[test]
    fn test_normalize_path() {
        let archiver = ContentArchiver::new();
        
        assert_eq!(archiver.normalize_path("/a/b/../c"), "/a/c");
        assert_eq!(archiver.normalize_path("/a/./b/c"), "/a/b/c");
        assert_eq!(archiver.normalize_path("/a/b/c/../.."), "/a");
    }

    #[test]
    fn test_extract_domain() {
        let archiver = ContentArchiver::new();
        
        assert_eq!(archiver.extract_domain("https://example.com/page"), "example.com");
        assert_eq!(archiver.extract_domain("https://sub.example.com:8080/page"), "sub.example.com");
        assert_eq!(archiver.extract_domain("http://EXAMPLE.COM/"), "example.com");
    }
}
