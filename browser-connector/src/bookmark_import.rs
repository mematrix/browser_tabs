//! Bookmark Import Module for Web Page Manager
//!
//! This module provides functionality to import bookmarks from various browsers,
//! parse bookmark data, validate bookmark accessibility, and standardize bookmark
//! information across different browser formats.
//!
//! # Features
//! - Auto-detection of installed browsers and their bookmark files
//! - Parsing of Chrome, Edge, and Firefox bookmark formats
//! - Bookmark validation and accessibility checking
//! - Standardized bookmark data structure
//!
//! # Requirements
//! - Requirement 2.1: Auto-detect and import bookmarks from all installed browsers
//! - Requirement 2.2: Validate bookmark accessibility and generate status reports

use web_page_manager_core::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Bookmark source information for import wizard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkSource {
    pub browser_type: BrowserType,
    pub profile_name: String,
    pub bookmark_path: PathBuf,
    pub bookmark_count: Option<usize>,
    pub last_modified: Option<DateTime<Utc>>,
    pub is_accessible: bool,
}

/// Import progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProgress {
    pub total_bookmarks: usize,
    pub processed: usize,
    pub successful: usize,
    pub failed: usize,
    pub current_browser: Option<BrowserType>,
    pub status: ImportStatus,
}

/// Import status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportStatus {
    NotStarted,
    Detecting,
    Importing,
    Validating,
    Completed,
    Failed(String),
}

/// Validation result for a single bookmark
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkValidationResult {
    pub bookmark: BookmarkInfo,
    pub status: AccessibilityStatus,
    pub response_time_ms: Option<u64>,
    pub redirect_url: Option<String>,
    pub validated_at: DateTime<Utc>,
}

/// Validation report for a batch of bookmarks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub total_bookmarks: usize,
    pub accessible: usize,
    pub not_found: usize,
    pub forbidden: usize,
    pub timeout: usize,
    pub network_errors: usize,
    pub results: Vec<BookmarkValidationResult>,
    pub generated_at: DateTime<Utc>,
    pub duration_ms: u64,
}

/// Chrome/Edge bookmark JSON structure
#[derive(Debug, Clone, Deserialize)]
pub struct ChromeBookmarks {
    pub roots: ChromeBookmarkRoots,
    pub version: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChromeBookmarkRoots {
    pub bookmark_bar: ChromeBookmarkNode,
    pub other: ChromeBookmarkNode,
    pub synced: Option<ChromeBookmarkNode>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChromeBookmarkNode {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub url: Option<String>,
    pub date_added: Option<String>,
    pub date_modified: Option<String>,
    pub children: Option<Vec<ChromeBookmarkNode>>,
}

/// Bookmark importer for detecting and importing bookmarks from browsers
pub struct BookmarkImporter {
    detected_sources: Vec<BookmarkSource>,
    import_progress: ImportProgress,
}

impl BookmarkImporter {
    /// Create a new bookmark importer
    pub fn new() -> Self {
        Self {
            detected_sources: Vec::new(),
            import_progress: ImportProgress {
                total_bookmarks: 0,
                processed: 0,
                successful: 0,
                failed: 0,
                current_browser: None,
                status: ImportStatus::NotStarted,
            },
        }
    }

    /// Detect all available bookmark sources from installed browsers
    /// 
    /// This implements Requirement 2.1: Auto-detect bookmarks from all installed browsers
    pub async fn detect_bookmark_sources(&mut self) -> Result<Vec<BookmarkSource>> {
        self.import_progress.status = ImportStatus::Detecting;
        let mut sources = Vec::new();

        // Detect Chrome bookmarks
        if let Some(source) = self.detect_chrome_bookmarks().await {
            sources.push(source);
        }

        // Detect Edge bookmarks
        if let Some(source) = self.detect_edge_bookmarks().await {
            sources.push(source);
        }

        // Detect Firefox bookmarks
        if let Some(source) = self.detect_firefox_bookmarks().await {
            sources.push(source);
        }

        self.detected_sources = sources.clone();
        self.import_progress.status = ImportStatus::NotStarted;
        
        tracing::info!("Detected {} bookmark sources", sources.len());
        Ok(sources)
    }

    /// Get the list of detected bookmark sources
    pub fn get_detected_sources(&self) -> &[BookmarkSource] {
        &self.detected_sources
    }

    /// Get current import progress
    pub fn get_progress(&self) -> &ImportProgress {
        &self.import_progress
    }

    /// Detect Chrome bookmark file
    async fn detect_chrome_bookmarks(&self) -> Option<BookmarkSource> {
        let bookmark_path = Self::get_chrome_bookmark_path()?;
        
        if !bookmark_path.exists() {
            return None;
        }

        let metadata = std::fs::metadata(&bookmark_path).ok()?;
        let last_modified = metadata.modified().ok()
            .map(|t| DateTime::<Utc>::from(t));

        // Try to count bookmarks
        let bookmark_count = self.count_chrome_bookmarks(&bookmark_path);

        Some(BookmarkSource {
            browser_type: BrowserType::Chrome,
            profile_name: "Default".to_string(),
            bookmark_path,
            bookmark_count,
            last_modified,
            is_accessible: true,
        })
    }

    /// Detect Edge bookmark file
    async fn detect_edge_bookmarks(&self) -> Option<BookmarkSource> {
        let bookmark_path = Self::get_edge_bookmark_path()?;
        
        if !bookmark_path.exists() {
            return None;
        }

        let metadata = std::fs::metadata(&bookmark_path).ok()?;
        let last_modified = metadata.modified().ok()
            .map(|t| DateTime::<Utc>::from(t));

        let bookmark_count = self.count_chrome_bookmarks(&bookmark_path);

        Some(BookmarkSource {
            browser_type: BrowserType::Edge,
            profile_name: "Default".to_string(),
            bookmark_path,
            bookmark_count,
            last_modified,
            is_accessible: true,
        })
    }

    /// Detect Firefox bookmark file (places.sqlite)
    async fn detect_firefox_bookmarks(&self) -> Option<BookmarkSource> {
        let profile_path = Self::get_firefox_profile_path()?;
        let bookmark_path = profile_path.join("places.sqlite");
        
        if !bookmark_path.exists() {
            return None;
        }

        let metadata = std::fs::metadata(&bookmark_path).ok()?;
        let last_modified = metadata.modified().ok()
            .map(|t| DateTime::<Utc>::from(t));

        Some(BookmarkSource {
            browser_type: BrowserType::Firefox,
            profile_name: profile_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Default")
                .to_string(),
            bookmark_path,
            bookmark_count: None, // Firefox requires SQLite access
            last_modified,
            is_accessible: true,
        })
    }

    /// Get Chrome bookmark file path based on platform
    fn get_chrome_bookmark_path() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            dirs::data_local_dir().map(|p| {
                p.join("Google").join("Chrome").join("User Data").join("Default").join("Bookmarks")
            })
        }
        
        #[cfg(target_os = "linux")]
        {
            dirs::config_dir().map(|p| {
                p.join("google-chrome").join("Default").join("Bookmarks")
            })
        }
        
        #[cfg(target_os = "macos")]
        {
            dirs::home_dir().map(|p| {
                p.join("Library")
                    .join("Application Support")
                    .join("Google")
                    .join("Chrome")
                    .join("Default")
                    .join("Bookmarks")
            })
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            None
        }
    }

    /// Get Edge bookmark file path based on platform
    fn get_edge_bookmark_path() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            dirs::data_local_dir().map(|p| {
                p.join("Microsoft").join("Edge").join("User Data").join("Default").join("Bookmarks")
            })
        }
        
        #[cfg(target_os = "linux")]
        {
            dirs::config_dir().map(|p| {
                p.join("microsoft-edge").join("Default").join("Bookmarks")
            })
        }

        #[cfg(target_os = "macos")]
        {
            dirs::home_dir().map(|p| {
                p.join("Library")
                    .join("Application Support")
                    .join("Microsoft Edge")
                    .join("Default")
                    .join("Bookmarks")
            })
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            None
        }
    }

    /// Get Firefox profile path
    fn get_firefox_profile_path() -> Option<PathBuf> {
        let profile_base = Self::get_firefox_profile_base()?;
        
        if !profile_base.exists() {
            return None;
        }

        // Look for default profile
        if let Ok(entries) = std::fs::read_dir(&profile_base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    if name.ends_with(".default") || name.ends_with(".default-release") {
                        return Some(path);
                    }
                }
            }
        }
        
        None
    }

    /// Get Firefox profile base directory
    fn get_firefox_profile_base() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            dirs::data_local_dir().map(|p| p.join("Mozilla").join("Firefox").join("Profiles"))
        }
        
        #[cfg(target_os = "linux")]
        {
            dirs::home_dir().map(|p| p.join(".mozilla").join("firefox"))
        }

        #[cfg(target_os = "macos")]
        {
            dirs::home_dir().map(|p| {
                p.join("Library")
                    .join("Application Support")
                    .join("Firefox")
                    .join("Profiles")
            })
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            None
        }
    }

    /// Count bookmarks in a Chrome/Edge bookmark file
    fn count_chrome_bookmarks(&self, path: &PathBuf) -> Option<usize> {
        let content = std::fs::read_to_string(path).ok()?;
        let bookmarks: ChromeBookmarks = serde_json::from_str(&content).ok()?;
        
        let mut count = 0;
        count += Self::count_chrome_node_bookmarks(&bookmarks.roots.bookmark_bar);
        count += Self::count_chrome_node_bookmarks(&bookmarks.roots.other);
        if let Some(ref synced) = bookmarks.roots.synced {
            count += Self::count_chrome_node_bookmarks(synced);
        }
        
        Some(count)
    }

    /// Recursively count bookmarks in a Chrome bookmark node
    fn count_chrome_node_bookmarks(node: &ChromeBookmarkNode) -> usize {
        let mut count = 0;
        
        if node.node_type == "url" {
            count += 1;
        }
        
        if let Some(ref children) = node.children {
            for child in children {
                count += Self::count_chrome_node_bookmarks(child);
            }
        }
        
        count
    }

    /// Import bookmarks from a specific browser
    pub async fn import_from_browser(&mut self, browser_type: BrowserType) -> Result<Vec<BookmarkInfo>> {
        self.import_progress.current_browser = Some(browser_type);
        self.import_progress.status = ImportStatus::Importing;

        let bookmarks = match browser_type {
            BrowserType::Chrome => self.import_chrome_bookmarks().await?,
            BrowserType::Edge => self.import_edge_bookmarks().await?,
            BrowserType::Firefox => self.import_firefox_bookmarks().await?,
            BrowserType::Safari => {
                return Err(WebPageManagerError::BrowserConnection {
                    source: BrowserConnectionError::BrowserNotRunning {
                        browser: BrowserType::Safari,
                    },
                });
            }
        };

        self.import_progress.successful += bookmarks.len();
        self.import_progress.processed += bookmarks.len();
        self.import_progress.status = ImportStatus::Completed;
        
        tracing::info!("Imported {} bookmarks from {:?}", bookmarks.len(), browser_type);
        Ok(bookmarks)
    }

    /// Import bookmarks from all detected sources
    pub async fn import_all(&mut self) -> Result<HashMap<BrowserType, Vec<BookmarkInfo>>> {
        let mut all_bookmarks = HashMap::new();
        
        // Clone sources to avoid borrow issues
        let sources: Vec<BrowserType> = self.detected_sources
            .iter()
            .map(|s| s.browser_type)
            .collect();

        self.import_progress.total_bookmarks = self.detected_sources
            .iter()
            .filter_map(|s| s.bookmark_count)
            .sum();

        for browser_type in sources {
            match self.import_from_browser(browser_type).await {
                Ok(bookmarks) => {
                    all_bookmarks.insert(browser_type, bookmarks);
                }
                Err(e) => {
                    tracing::warn!("Failed to import from {:?}: {}", browser_type, e);
                    self.import_progress.failed += 1;
                }
            }
        }

        Ok(all_bookmarks)
    }

    /// Import Chrome bookmarks
    async fn import_chrome_bookmarks(&self) -> Result<Vec<BookmarkInfo>> {
        let path = Self::get_chrome_bookmark_path().ok_or_else(|| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Chrome,
                },
            }
        })?;

        self.parse_chrome_bookmarks(&path, BrowserType::Chrome)
    }

    /// Import Edge bookmarks
    async fn import_edge_bookmarks(&self) -> Result<Vec<BookmarkInfo>> {
        let path = Self::get_edge_bookmark_path().ok_or_else(|| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Edge,
                },
            }
        })?;

        self.parse_chrome_bookmarks(&path, BrowserType::Edge)
    }

    /// Parse Chrome/Edge bookmark JSON file
    fn parse_chrome_bookmarks(&self, path: &PathBuf, browser_type: BrowserType) -> Result<Vec<BookmarkInfo>> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            WebPageManagerError::System {
                source: SystemError::IO { source: e },
            }
        })?;

        let chrome_bookmarks: ChromeBookmarks = serde_json::from_str(&content).map_err(|e| {
            WebPageManagerError::System {
                source: SystemError::Serialization { source: e },
            }
        })?;

        let mut bookmarks = Vec::new();
        
        // Parse bookmark bar
        self.parse_chrome_node(
            &chrome_bookmarks.roots.bookmark_bar,
            &mut bookmarks,
            vec!["Bookmarks Bar".to_string()],
            browser_type,
        );

        // Parse other bookmarks
        self.parse_chrome_node(
            &chrome_bookmarks.roots.other,
            &mut bookmarks,
            vec!["Other Bookmarks".to_string()],
            browser_type,
        );

        // Parse synced bookmarks if present
        if let Some(ref synced) = chrome_bookmarks.roots.synced {
            self.parse_chrome_node(
                synced,
                &mut bookmarks,
                vec!["Synced Bookmarks".to_string()],
                browser_type,
            );
        }

        Ok(bookmarks)
    }

    /// Recursively parse Chrome bookmark nodes
    fn parse_chrome_node(
        &self,
        node: &ChromeBookmarkNode,
        bookmarks: &mut Vec<BookmarkInfo>,
        folder_path: Vec<String>,
        browser_type: BrowserType,
    ) {
        if node.node_type == "url" {
            if let Some(ref url) = node.url {
                let created_at = node.date_added
                    .as_ref()
                    .and_then(|d| Self::parse_chrome_timestamp(d))
                    .unwrap_or_else(Utc::now);

                bookmarks.push(BookmarkInfo {
                    id: BookmarkId(node.id.clone()),
                    url: url.clone(),
                    title: node.name.clone(),
                    favicon_url: None,
                    browser_type,
                    folder_path: folder_path.clone(),
                    created_at,
                    last_accessed: None,
                });
            }
        } else if node.node_type == "folder" {
            if let Some(ref children) = node.children {
                let mut child_path = folder_path.clone();
                if !node.name.is_empty() {
                    child_path.push(node.name.clone());
                }
                
                for child in children {
                    self.parse_chrome_node(child, bookmarks, child_path.clone(), browser_type);
                }
            }
        }
    }

    /// Parse Chrome timestamp (microseconds since Windows epoch)
    fn parse_chrome_timestamp(timestamp_str: &str) -> Option<DateTime<Utc>> {
        let timestamp: i64 = timestamp_str.parse().ok()?;
        // Chrome uses microseconds since January 1, 1601
        // Convert to Unix timestamp (seconds since January 1, 1970)
        let windows_epoch_offset: i64 = 11644473600; // seconds between 1601 and 1970
        let unix_timestamp = (timestamp / 1_000_000) - windows_epoch_offset;
        DateTime::from_timestamp(unix_timestamp, 0)
    }

    /// Import Firefox bookmarks from places.sqlite
    async fn import_firefox_bookmarks(&self) -> Result<Vec<BookmarkInfo>> {
        let profile_path = Self::get_firefox_profile_path().ok_or_else(|| {
            WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Firefox,
                },
            }
        })?;

        let places_db = profile_path.join("places.sqlite");
        
        if !places_db.exists() {
            return Err(WebPageManagerError::BrowserConnection {
                source: BrowserConnectionError::BrowserNotRunning {
                    browser: BrowserType::Firefox,
                },
            });
        }

        // Firefox locks the database while running, so we need to copy it first
        let temp_db = std::env::temp_dir().join("wpm_firefox_places.sqlite");
        std::fs::copy(&places_db, &temp_db).map_err(|e| {
            WebPageManagerError::System {
                source: SystemError::IO { source: e },
            }
        })?;

        let bookmarks = self.parse_firefox_bookmarks(&temp_db)?;
        
        // Clean up temp file
        let _ = std::fs::remove_file(&temp_db);
        
        Ok(bookmarks)
    }

    /// Parse Firefox places.sqlite database
    fn parse_firefox_bookmarks(&self, db_path: &PathBuf) -> Result<Vec<BookmarkInfo>> {
        use rusqlite::Connection;
        
        let conn = Connection::open(db_path).map_err(|e| {
            WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to open Firefox database: {}", e),
                },
            }
        })?;

        // Query bookmarks with their folder paths
        let mut stmt = conn.prepare(
            r#"
            WITH RECURSIVE folder_path(id, title, parent, path) AS (
                SELECT id, title, parent, title as path
                FROM moz_bookmarks
                WHERE parent = 0 OR parent IS NULL
                UNION ALL
                SELECT b.id, b.title, b.parent, fp.path || '/' || b.title
                FROM moz_bookmarks b
                JOIN folder_path fp ON b.parent = fp.id
                WHERE b.type = 2
            )
            SELECT 
                b.id,
                p.url,
                COALESCE(b.title, p.title, '') as title,
                b.dateAdded,
                COALESCE(fp.path, '') as folder_path
            FROM moz_bookmarks b
            JOIN moz_places p ON b.fk = p.id
            LEFT JOIN folder_path fp ON b.parent = fp.id
            WHERE b.type = 1 AND p.url IS NOT NULL AND p.url NOT LIKE 'place:%'
            "#
        ).map_err(|e| {
            WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to prepare Firefox query: {}", e),
                },
            }
        })?;

        let bookmarks: Vec<BookmarkInfo> = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let url: String = row.get(1)?;
            let title: String = row.get(2)?;
            let date_added: Option<i64> = row.get(3)?;
            let folder_path_str: String = row.get(4)?;

            let created_at = date_added
                .and_then(|ts| DateTime::from_timestamp_micros(ts))
                .unwrap_or_else(Utc::now);

            let folder_path: Vec<String> = folder_path_str
                .split('/')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();

            Ok(BookmarkInfo {
                id: BookmarkId(id.to_string()),
                url,
                title,
                favicon_url: None,
                browser_type: BrowserType::Firefox,
                folder_path,
                created_at,
                last_accessed: None,
            })
        }).map_err(|e| {
            WebPageManagerError::System {
                source: SystemError::Configuration {
                    details: format!("Failed to query Firefox bookmarks: {}", e),
                },
            }
        })?
        .filter_map(|r| r.ok())
        .collect();

        Ok(bookmarks)
    }
}

impl Default for BookmarkImporter {
    fn default() -> Self {
        Self::new()
    }
}


/// Bookmark validator for checking accessibility status
/// 
/// Implements Requirement 2.2: Validate bookmark accessibility and generate status reports
pub struct BookmarkValidator {
    client: reqwest::Client,
    timeout_secs: u64,
    max_concurrent: usize,
}

impl BookmarkValidator {
    /// Create a new bookmark validator with default settings
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .redirect(reqwest::redirect::Policy::limited(5))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            timeout_secs: 10,
            max_concurrent: 10,
        }
    }

    /// Create a validator with custom timeout
    pub fn with_timeout(timeout_secs: u64) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(timeout_secs))
                .redirect(reqwest::redirect::Policy::limited(5))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            timeout_secs,
            max_concurrent: 10,
        }
    }

    /// Set maximum concurrent validations
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }

    /// Validate a single bookmark's accessibility
    pub async fn validate_bookmark(&self, bookmark: &BookmarkInfo) -> BookmarkValidationResult {
        let start = std::time::Instant::now();
        let validated_at = Utc::now();

        let (status, redirect_url) = self.check_url(&bookmark.url).await;
        
        let response_time_ms = Some(start.elapsed().as_millis() as u64);

        BookmarkValidationResult {
            bookmark: bookmark.clone(),
            status,
            response_time_ms,
            redirect_url,
            validated_at,
        }
    }

    /// Check URL accessibility and return status
    async fn check_url(&self, url: &str) -> (AccessibilityStatus, Option<String>) {
        // Skip non-HTTP URLs
        if !url.starts_with("http://") && !url.starts_with("https://") {
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

                let status = match response.status().as_u16() {
                    200..=299 => AccessibilityStatus::Accessible,
                    301 | 302 | 307 | 308 => AccessibilityStatus::Accessible,
                    403 => AccessibilityStatus::Forbidden,
                    404 => AccessibilityStatus::NotFound,
                    _ => AccessibilityStatus::NetworkError(
                        format!("HTTP {}", response.status().as_u16())
                    ),
                };

                (status, redirect_url)
            }
            Err(e) => {
                if e.is_timeout() {
                    (AccessibilityStatus::Timeout, None)
                } else if e.is_connect() {
                    (AccessibilityStatus::NetworkError("Connection failed".to_string()), None)
                } else {
                    (AccessibilityStatus::NetworkError(e.to_string()), None)
                }
            }
        }
    }

    /// Validate a batch of bookmarks and generate a report
    pub async fn validate_batch(&self, bookmarks: &[BookmarkInfo]) -> ValidationReport {
        use futures_util::stream::{self, StreamExt};
        
        let start = std::time::Instant::now();
        let generated_at = Utc::now();

        let results: Vec<BookmarkValidationResult> = stream::iter(bookmarks)
            .map(|bookmark| self.validate_bookmark(bookmark))
            .buffer_unordered(self.max_concurrent)
            .collect()
            .await;

        let mut accessible = 0;
        let mut not_found = 0;
        let mut forbidden = 0;
        let mut timeout = 0;
        let mut network_errors = 0;

        for result in &results {
            match result.status {
                AccessibilityStatus::Accessible => accessible += 1,
                AccessibilityStatus::NotFound => not_found += 1,
                AccessibilityStatus::Forbidden => forbidden += 1,
                AccessibilityStatus::Timeout => timeout += 1,
                AccessibilityStatus::NetworkError(_) => network_errors += 1,
            }
        }

        ValidationReport {
            total_bookmarks: bookmarks.len(),
            accessible,
            not_found,
            forbidden,
            timeout,
            network_errors,
            results,
            generated_at,
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// Get timeout setting
    pub fn timeout_secs(&self) -> u64 {
        self.timeout_secs
    }

    /// Get max concurrent setting
    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }
}

impl Default for BookmarkValidator {
    fn default() -> Self {
        Self::new()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bookmark_importer_creation() {
        let importer = BookmarkImporter::new();
        assert!(importer.get_detected_sources().is_empty());
        assert_eq!(importer.get_progress().status, ImportStatus::NotStarted);
    }

    #[test]
    fn test_chrome_timestamp_parsing() {
        // Chrome timestamp for 2024-01-01 00:00:00 UTC
        // Windows epoch (1601) + 423 years to 2024 in microseconds
        let timestamp = "13351334400000000"; // Example Chrome timestamp
        let result = BookmarkImporter::parse_chrome_timestamp(timestamp);
        assert!(result.is_some());
    }

    #[test]
    fn test_chrome_bookmark_node_counting() {
        let node = ChromeBookmarkNode {
            id: "1".to_string(),
            name: "Test Folder".to_string(),
            node_type: "folder".to_string(),
            url: None,
            date_added: None,
            date_modified: None,
            children: Some(vec![
                ChromeBookmarkNode {
                    id: "2".to_string(),
                    name: "Bookmark 1".to_string(),
                    node_type: "url".to_string(),
                    url: Some("https://example.com".to_string()),
                    date_added: None,
                    date_modified: None,
                    children: None,
                },
                ChromeBookmarkNode {
                    id: "3".to_string(),
                    name: "Bookmark 2".to_string(),
                    node_type: "url".to_string(),
                    url: Some("https://example.org".to_string()),
                    date_added: None,
                    date_modified: None,
                    children: None,
                },
            ]),
        };

        let count = BookmarkImporter::count_chrome_node_bookmarks(&node);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_bookmark_validator_creation() {
        let validator = BookmarkValidator::new();
        assert_eq!(validator.timeout_secs(), 10);
        assert_eq!(validator.max_concurrent(), 10);
    }

    #[test]
    fn test_bookmark_validator_with_timeout() {
        let validator = BookmarkValidator::with_timeout(30);
        assert_eq!(validator.timeout_secs(), 30);
    }

    #[test]
    fn test_bookmark_validator_with_max_concurrent() {
        let validator = BookmarkValidator::new().with_max_concurrent(5);
        assert_eq!(validator.max_concurrent(), 5);
    }

    #[test]
    fn test_import_progress_default() {
        let progress = ImportProgress {
            total_bookmarks: 0,
            processed: 0,
            successful: 0,
            failed: 0,
            current_browser: None,
            status: ImportStatus::NotStarted,
        };
        
        assert_eq!(progress.total_bookmarks, 0);
        assert_eq!(progress.status, ImportStatus::NotStarted);
    }

    #[test]
    fn test_validation_report_creation() {
        let report = ValidationReport {
            total_bookmarks: 10,
            accessible: 7,
            not_found: 2,
            forbidden: 0,
            timeout: 1,
            network_errors: 0,
            results: vec![],
            generated_at: Utc::now(),
            duration_ms: 1000,
        };

        assert_eq!(report.total_bookmarks, 10);
        assert_eq!(report.accessible, 7);
        assert_eq!(report.not_found, 2);
    }

    #[test]
    fn test_bookmark_source_creation() {
        let source = BookmarkSource {
            browser_type: BrowserType::Chrome,
            profile_name: "Default".to_string(),
            bookmark_path: PathBuf::from("/path/to/bookmarks"),
            bookmark_count: Some(100),
            last_modified: Some(Utc::now()),
            is_accessible: true,
        };

        assert_eq!(source.browser_type, BrowserType::Chrome);
        assert_eq!(source.bookmark_count, Some(100));
        assert!(source.is_accessible);
    }
}
