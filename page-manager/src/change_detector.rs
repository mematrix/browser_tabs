//! Page Change Detection for Web Page Manager
//!
//! This module provides functionality to detect changes in archived web pages,
//! manage content versions, and notify users about updates.
//!
//! # Features
//! - Page content change monitoring system
//! - Incremental update and version management
//! - Update notification and user choice mechanism
//!
//! # Requirements Implemented
//! - 3.5: Detect page changes and provide update archive options

use web_page_manager_core::*;
use data_access::{ContentArchive, ArchiveRepository};
use std::sync::Arc;
use chrono::{DateTime, Utc, Duration};
use tracing::{info, debug};
use serde::{Deserialize, Serialize};

/// Configuration for the change detector
#[derive(Debug, Clone)]
pub struct ChangeDetectorConfig {
    /// Minimum time between checks for the same URL (in hours)
    pub min_check_interval_hours: u32,
    /// Similarity threshold below which content is considered changed (0.0 - 1.0)
    pub change_threshold: f32,
    /// Maximum number of versions to keep per page
    pub max_versions_per_page: usize,
    /// Whether to automatically check for changes on archived pages
    pub auto_check_enabled: bool,
    /// Batch size for checking multiple pages
    pub batch_size: usize,
}

impl Default for ChangeDetectorConfig {
    fn default() -> Self {
        Self {
            min_check_interval_hours: 24,
            change_threshold: 0.85,
            max_versions_per_page: 5,
            auto_check_enabled: true,
            batch_size: 10,
        }
    }
}

/// Type of change detected in page content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChangeType {
    /// No significant change detected
    NoChange,
    /// Minor changes (typos, formatting)
    Minor,
    /// Moderate changes (some content updated)
    Moderate,
    /// Major changes (significant content rewrite)
    Major,
    /// Page structure changed significantly
    StructuralChange,
    /// Page no longer accessible
    PageUnavailable,
    /// Page redirects to different URL
    Redirected { new_url: String },
}

/// Result of a change detection check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeDetectionResult {
    /// The archive that was checked
    pub archive_id: ArchiveId,
    /// URL of the page
    pub url: String,
    /// Type of change detected
    pub change_type: ChangeType,
    /// Similarity score between old and new content (0.0 - 1.0)
    pub similarity_score: f32,
    /// Summary of changes detected
    pub change_summary: String,
    /// New content if changes were detected
    pub new_content: Option<PageChangeContent>,
    /// When the check was performed
    pub checked_at: DateTime<Utc>,
    /// Time taken to perform the check (in milliseconds)
    pub check_duration_ms: u64,
}

/// Content from a changed page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageChangeContent {
    /// New HTML content
    pub html: String,
    /// New text content
    pub text: String,
    /// New title
    pub title: String,
    /// Content checksum
    pub checksum: String,
    /// When the content was fetched
    pub fetched_at: DateTime<Utc>,
}

/// Version information for an archived page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveVersion {
    /// Version number (1 = original, 2+ = updates)
    pub version: u32,
    /// Archive ID for this version
    pub archive_id: ArchiveId,
    /// When this version was created
    pub created_at: DateTime<Utc>,
    /// Content checksum for this version
    pub checksum: String,
    /// Size of this version in bytes
    pub size_bytes: u64,
    /// Change type from previous version
    pub change_from_previous: ChangeType,
}

/// Update option presented to the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateOption {
    /// Update the archive with new content
    UpdateArchive,
    /// Keep the current archive, ignore changes
    KeepCurrent,
    /// Create a new version while keeping the old one
    CreateNewVersion,
    /// Delete the archive (page no longer relevant)
    DeleteArchive,
}

/// Notification about detected changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeNotification {
    /// Unique notification ID
    pub id: Uuid,
    /// Archive that changed
    pub archive_id: ArchiveId,
    /// URL of the page
    pub url: String,
    /// Title of the page
    pub title: String,
    /// Type of change detected
    pub change_type: ChangeType,
    /// Human-readable summary of changes
    pub summary: String,
    /// When the notification was created
    pub created_at: DateTime<Utc>,
    /// Whether the user has seen this notification
    pub is_read: bool,
    /// User's chosen action (if any)
    pub user_action: Option<UpdateOption>,
}


/// Page change detector for monitoring archived content
pub struct ChangeDetector {
    config: ChangeDetectorConfig,
    archive_repository: Option<Arc<dyn ArchiveRepository + Send + Sync>>,
    /// Track last check time for each URL
    last_check_times: Arc<tokio::sync::RwLock<std::collections::HashMap<String, DateTime<Utc>>>>,
    /// Pending notifications
    notifications: Arc<tokio::sync::RwLock<Vec<ChangeNotification>>>,
    /// Version history for archives
    version_history: Arc<tokio::sync::RwLock<std::collections::HashMap<Uuid, Vec<ArchiveVersion>>>>,
}

impl ChangeDetector {
    /// Create a new change detector with default configuration
    pub fn new() -> Self {
        Self {
            config: ChangeDetectorConfig::default(),
            archive_repository: None,
            last_check_times: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            notifications: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            version_history: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Create a new change detector with custom configuration
    pub fn with_config(config: ChangeDetectorConfig) -> Self {
        Self {
            config,
            archive_repository: None,
            last_check_times: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            notifications: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            version_history: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Set the archive repository for persistence
    pub fn with_repository(mut self, repository: Arc<dyn ArchiveRepository + Send + Sync>) -> Self {
        self.archive_repository = Some(repository);
        self
    }

    /// Get the current configuration
    pub fn config(&self) -> &ChangeDetectorConfig {
        &self.config
    }

    /// Check if a URL should be checked for changes based on the minimum interval
    pub async fn should_check(&self, url: &str) -> bool {
        let last_checks = self.last_check_times.read().await;
        if let Some(last_check) = last_checks.get(url) {
            let min_interval = Duration::hours(self.config.min_check_interval_hours as i64);
            Utc::now() - *last_check >= min_interval
        } else {
            true
        }
    }

    /// Record that a URL was checked
    async fn record_check(&self, url: &str) {
        let mut last_checks = self.last_check_times.write().await;
        last_checks.insert(url.to_string(), Utc::now());
    }

    /// Check a single archive for changes
    ///
    /// This method compares the archived content with the current live content
    /// and returns a detection result indicating what changed.
    pub async fn check_for_changes(
        &self,
        archive: &ContentArchive,
        new_content: &PageChangeContent,
    ) -> ChangeDetectionResult {
        let start_time = std::time::Instant::now();
        
        debug!("Checking for changes: {}", archive.url);
        
        // Calculate similarity between old and new content
        let similarity_score = self.calculate_similarity(&archive.content_text, &new_content.text);
        
        // Determine change type based on similarity
        let change_type = self.classify_change(similarity_score, archive, new_content);
        
        // Generate change summary
        let change_summary = self.generate_change_summary(&change_type, similarity_score, archive, new_content);
        
        // Record the check
        self.record_check(&archive.url).await;
        
        let duration_ms = start_time.elapsed().as_millis() as u64;
        
        let result = ChangeDetectionResult {
            archive_id: archive.id.clone(),
            url: archive.url.clone(),
            change_type: change_type.clone(),
            similarity_score,
            change_summary,
            new_content: if change_type != ChangeType::NoChange {
                Some(new_content.clone())
            } else {
                None
            },
            checked_at: Utc::now(),
            check_duration_ms: duration_ms,
        };
        
        // Create notification if changes detected
        if change_type != ChangeType::NoChange {
            self.create_notification(&result, &archive.title).await;
        }
        
        info!(
            "Change detection complete for {}: {:?} (similarity: {:.2})",
            archive.url, change_type, similarity_score
        );
        
        result
    }

    /// Calculate text similarity using a simple algorithm
    /// Returns a value between 0.0 (completely different) and 1.0 (identical)
    fn calculate_similarity(&self, old_text: &str, new_text: &str) -> f32 {
        if old_text == new_text {
            return 1.0;
        }
        
        if old_text.is_empty() || new_text.is_empty() {
            return 0.0;
        }
        
        // Use word-based Jaccard similarity
        let old_words: std::collections::HashSet<&str> = old_text
            .split_whitespace()
            .collect();
        let new_words: std::collections::HashSet<&str> = new_text
            .split_whitespace()
            .collect();
        
        if old_words.is_empty() && new_words.is_empty() {
            return 1.0;
        }
        
        let intersection = old_words.intersection(&new_words).count();
        let union = old_words.union(&new_words).count();
        
        if union == 0 {
            return 1.0;
        }
        
        intersection as f32 / union as f32
    }

    /// Classify the type of change based on similarity and content analysis
    fn classify_change(
        &self,
        similarity: f32,
        archive: &ContentArchive,
        new_content: &PageChangeContent,
    ) -> ChangeType {
        // Check if checksums match (no change)
        if let Some(ref old_checksum) = archive.checksum {
            if *old_checksum == new_content.checksum {
                return ChangeType::NoChange;
            }
        }
        
        // Classify based on similarity threshold
        if similarity >= self.config.change_threshold {
            return ChangeType::NoChange;
        }
        
        if similarity >= 0.7 {
            return ChangeType::Minor;
        }
        
        if similarity >= 0.4 {
            return ChangeType::Moderate;
        }
        
        // Check for structural changes (title changed significantly)
        if archive.title != new_content.title {
            let title_similarity = self.calculate_similarity(&archive.title, &new_content.title);
            if title_similarity < 0.5 {
                return ChangeType::StructuralChange;
            }
        }
        
        ChangeType::Major
    }

    /// Generate a human-readable summary of the changes
    fn generate_change_summary(
        &self,
        change_type: &ChangeType,
        similarity: f32,
        archive: &ContentArchive,
        new_content: &PageChangeContent,
    ) -> String {
        match change_type {
            ChangeType::NoChange => "No significant changes detected.".to_string(),
            ChangeType::Minor => format!(
                "Minor changes detected ({:.0}% similar). Small updates to content.",
                similarity * 100.0
            ),
            ChangeType::Moderate => format!(
                "Moderate changes detected ({:.0}% similar). Some sections have been updated.",
                similarity * 100.0
            ),
            ChangeType::Major => format!(
                "Major changes detected ({:.0}% similar). Significant content rewrite.",
                similarity * 100.0
            ),
            ChangeType::StructuralChange => {
                let title_changed = if archive.title != new_content.title {
                    format!(" Title changed from '{}' to '{}'.", archive.title, new_content.title)
                } else {
                    String::new()
                };
                format!(
                    "Page structure has changed significantly ({:.0}% similar).{}",
                    similarity * 100.0,
                    title_changed
                )
            }
            ChangeType::PageUnavailable => "Page is no longer accessible.".to_string(),
            ChangeType::Redirected { new_url } => {
                format!("Page redirects to: {}", new_url)
            }
        }
    }

    /// Create a notification for detected changes
    async fn create_notification(&self, result: &ChangeDetectionResult, title: &str) {
        let notification = ChangeNotification {
            id: Uuid::new_v4(),
            archive_id: result.archive_id.clone(),
            url: result.url.clone(),
            title: title.to_string(),
            change_type: result.change_type.clone(),
            summary: result.change_summary.clone(),
            created_at: Utc::now(),
            is_read: false,
            user_action: None,
        };
        
        let mut notifications = self.notifications.write().await;
        notifications.push(notification);
        
        debug!("Created change notification for: {}", result.url);
    }

    /// Get all pending notifications
    pub async fn get_notifications(&self) -> Vec<ChangeNotification> {
        let notifications = self.notifications.read().await;
        notifications.clone()
    }

    /// Get unread notifications
    pub async fn get_unread_notifications(&self) -> Vec<ChangeNotification> {
        let notifications = self.notifications.read().await;
        notifications.iter().filter(|n| !n.is_read).cloned().collect()
    }

    /// Mark a notification as read
    pub async fn mark_notification_read(&self, notification_id: &Uuid) {
        let mut notifications = self.notifications.write().await;
        if let Some(notification) = notifications.iter_mut().find(|n| n.id == *notification_id) {
            notification.is_read = true;
        }
    }

    /// Apply user's chosen action for a notification
    pub async fn apply_user_action(
        &self,
        notification_id: &Uuid,
        action: UpdateOption,
    ) -> Result<()> {
        let mut notifications = self.notifications.write().await;
        if let Some(notification) = notifications.iter_mut().find(|n| n.id == *notification_id) {
            notification.user_action = Some(action.clone());
            notification.is_read = true;
            
            info!(
                "User action applied for {}: {:?}",
                notification.url, action
            );
        }
        Ok(())
    }

    /// Clear all notifications
    pub async fn clear_notifications(&self) {
        let mut notifications = self.notifications.write().await;
        notifications.clear();
    }

    /// Clear read notifications only
    pub async fn clear_read_notifications(&self) {
        let mut notifications = self.notifications.write().await;
        notifications.retain(|n| !n.is_read);
    }


    /// Add a version to the history for an archive
    pub async fn add_version(
        &self,
        page_id: &Uuid,
        archive_id: &ArchiveId,
        checksum: &str,
        size_bytes: u64,
        change_type: ChangeType,
    ) {
        let mut history = self.version_history.write().await;
        let versions = history.entry(*page_id).or_insert_with(Vec::new);
        
        let version_number = versions.len() as u32 + 1;
        
        let version = ArchiveVersion {
            version: version_number,
            archive_id: archive_id.clone(),
            created_at: Utc::now(),
            checksum: checksum.to_string(),
            size_bytes,
            change_from_previous: change_type,
        };
        
        versions.push(version);
        
        // Enforce max versions limit
        if versions.len() > self.config.max_versions_per_page {
            versions.remove(0);
        }
        
        debug!("Added version {} for page {}", version_number, page_id);
    }

    /// Get version history for a page
    pub async fn get_version_history(&self, page_id: &Uuid) -> Vec<ArchiveVersion> {
        let history = self.version_history.read().await;
        history.get(page_id).cloned().unwrap_or_default()
    }

    /// Get the latest version for a page
    pub async fn get_latest_version(&self, page_id: &Uuid) -> Option<ArchiveVersion> {
        let history = self.version_history.read().await;
        history.get(page_id).and_then(|v| v.last().cloned())
    }

    /// Compare two versions and return the differences
    pub fn compare_versions(
        &self,
        old_content: &str,
        new_content: &str,
    ) -> VersionComparison {
        let old_lines: Vec<&str> = old_content.lines().collect();
        let new_lines: Vec<&str> = new_content.lines().collect();
        
        let mut added_lines = Vec::new();
        let mut removed_lines = Vec::new();
        let mut unchanged_lines = 0;
        
        // Simple line-by-line comparison
        let old_set: std::collections::HashSet<&str> = old_lines.iter().copied().collect();
        let new_set: std::collections::HashSet<&str> = new_lines.iter().copied().collect();
        
        for line in &new_lines {
            if !old_set.contains(line) {
                added_lines.push(line.to_string());
            } else {
                unchanged_lines += 1;
            }
        }
        
        for line in &old_lines {
            if !new_set.contains(line) {
                removed_lines.push(line.to_string());
            }
        }
        
        let total_lines = old_lines.len().max(new_lines.len());
        let change_percentage = if total_lines > 0 {
            ((added_lines.len() + removed_lines.len()) as f32 / total_lines as f32) * 100.0
        } else {
            0.0
        };
        
        VersionComparison {
            added_lines,
            removed_lines,
            unchanged_lines,
            change_percentage,
        }
    }

    /// Calculate content checksum using FNV-1a hash
    pub fn calculate_checksum(&self, content: &str) -> String {
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in content.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{:016x}", hash)
    }

    /// Check multiple archives for changes in batch
    pub async fn check_batch(
        &self,
        archives: &[ContentArchive],
        content_fetcher: impl Fn(&str) -> Option<PageChangeContent>,
    ) -> Vec<ChangeDetectionResult> {
        let mut results = Vec::new();
        
        for archive in archives.iter().take(self.config.batch_size) {
            // Skip if recently checked
            if !self.should_check(&archive.url).await {
                debug!("Skipping {} - recently checked", archive.url);
                continue;
            }
            
            // Fetch new content
            if let Some(new_content) = content_fetcher(&archive.url) {
                let result = self.check_for_changes(archive, &new_content).await;
                results.push(result);
            } else {
                // Page unavailable
                let result = ChangeDetectionResult {
                    archive_id: archive.id.clone(),
                    url: archive.url.clone(),
                    change_type: ChangeType::PageUnavailable,
                    similarity_score: 0.0,
                    change_summary: "Page is no longer accessible.".to_string(),
                    new_content: None,
                    checked_at: Utc::now(),
                    check_duration_ms: 0,
                };
                
                self.create_notification(&result, &archive.title).await;
                results.push(result);
            }
        }
        
        info!("Batch check complete: {} archives checked", results.len());
        results
    }

    /// Get statistics about change detection
    pub async fn get_stats(&self) -> ChangeDetectorStats {
        let notifications = self.notifications.read().await;
        let last_checks = self.last_check_times.read().await;
        let version_history = self.version_history.read().await;
        
        let unread_count = notifications.iter().filter(|n| !n.is_read).count();
        let total_versions: usize = version_history.values().map(|v| v.len()).sum();
        
        ChangeDetectorStats {
            total_notifications: notifications.len(),
            unread_notifications: unread_count,
            urls_tracked: last_checks.len(),
            total_versions,
            pages_with_versions: version_history.len(),
        }
    }
}

impl Default for ChangeDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Comparison result between two versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionComparison {
    /// Lines added in the new version
    pub added_lines: Vec<String>,
    /// Lines removed from the old version
    pub removed_lines: Vec<String>,
    /// Number of unchanged lines
    pub unchanged_lines: usize,
    /// Percentage of content that changed
    pub change_percentage: f32,
}

/// Statistics about change detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeDetectorStats {
    /// Total number of notifications
    pub total_notifications: usize,
    /// Number of unread notifications
    pub unread_notifications: usize,
    /// Number of URLs being tracked
    pub urls_tracked: usize,
    /// Total number of versions across all pages
    pub total_versions: usize,
    /// Number of pages with version history
    pub pages_with_versions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_similarity_identical() {
        let detector = ChangeDetector::new();
        let similarity = detector.calculate_similarity("hello world", "hello world");
        assert_eq!(similarity, 1.0);
    }

    #[test]
    fn test_calculate_similarity_completely_different() {
        let detector = ChangeDetector::new();
        let similarity = detector.calculate_similarity("hello world", "foo bar baz");
        assert!(similarity < 0.5);
    }

    #[test]
    fn test_calculate_similarity_partial_overlap() {
        let detector = ChangeDetector::new();
        let similarity = detector.calculate_similarity(
            "the quick brown fox",
            "the quick red fox",
        );
        // 3 out of 5 unique words match
        assert!(similarity > 0.5);
        assert!(similarity < 1.0);
    }

    #[test]
    fn test_calculate_similarity_empty_strings() {
        let detector = ChangeDetector::new();
        // Two empty strings are considered identical (100% similar)
        assert_eq!(detector.calculate_similarity("", ""), 1.0);
        // One empty and one non-empty are completely different
        assert_eq!(detector.calculate_similarity("hello", ""), 0.0);
        assert_eq!(detector.calculate_similarity("", "world"), 0.0);
    }

    #[test]
    fn test_classify_change_no_change() {
        let detector = ChangeDetector::new();
        let archive = create_test_archive("test content", "abc123");
        let new_content = create_test_content("test content", "abc123");
        
        let change_type = detector.classify_change(1.0, &archive, &new_content);
        assert_eq!(change_type, ChangeType::NoChange);
    }

    #[test]
    fn test_classify_change_minor() {
        let detector = ChangeDetector::new();
        let archive = create_test_archive("test content here", "abc123");
        let new_content = create_test_content("test content there", "def456");
        
        let change_type = detector.classify_change(0.75, &archive, &new_content);
        assert_eq!(change_type, ChangeType::Minor);
    }

    #[test]
    fn test_classify_change_moderate() {
        let detector = ChangeDetector::new();
        let archive = create_test_archive("original content", "abc123");
        let new_content = create_test_content("new different content", "def456");
        
        let change_type = detector.classify_change(0.5, &archive, &new_content);
        assert_eq!(change_type, ChangeType::Moderate);
    }

    #[test]
    fn test_classify_change_major() {
        let detector = ChangeDetector::new();
        let archive = create_test_archive("original content", "abc123");
        let new_content = create_test_content("completely different text", "def456");
        
        let change_type = detector.classify_change(0.2, &archive, &new_content);
        assert_eq!(change_type, ChangeType::Major);
    }

    #[test]
    fn test_compare_versions() {
        let detector = ChangeDetector::new();
        let old_content = "line 1\nline 2\nline 3";
        let new_content = "line 1\nline 2 modified\nline 4";
        
        let comparison = detector.compare_versions(old_content, new_content);
        
        assert!(!comparison.added_lines.is_empty());
        assert!(!comparison.removed_lines.is_empty());
        assert!(comparison.unchanged_lines > 0);
        assert!(comparison.change_percentage > 0.0);
    }

    #[test]
    fn test_calculate_checksum() {
        let detector = ChangeDetector::new();
        
        let checksum1 = detector.calculate_checksum("test content");
        let checksum2 = detector.calculate_checksum("test content");
        let checksum3 = detector.calculate_checksum("different content");
        
        assert_eq!(checksum1, checksum2);
        assert_ne!(checksum1, checksum3);
    }

    #[test]
    fn test_generate_change_summary() {
        let detector = ChangeDetector::new();
        let archive = create_test_archive("content", "abc");
        let new_content = create_test_content("content", "abc");
        
        let summary = detector.generate_change_summary(
            &ChangeType::NoChange,
            1.0,
            &archive,
            &new_content,
        );
        assert!(summary.contains("No significant changes"));
        
        let summary = detector.generate_change_summary(
            &ChangeType::Minor,
            0.8,
            &archive,
            &new_content,
        );
        assert!(summary.contains("Minor changes"));
        
        let summary = detector.generate_change_summary(
            &ChangeType::Major,
            0.3,
            &archive,
            &new_content,
        );
        assert!(summary.contains("Major changes"));
    }

    #[tokio::test]
    async fn test_should_check() {
        let config = ChangeDetectorConfig {
            min_check_interval_hours: 1,
            ..Default::default()
        };
        let detector = ChangeDetector::with_config(config);
        
        // First check should always be allowed
        assert!(detector.should_check("https://example.com").await);
        
        // Record a check
        detector.record_check("https://example.com").await;
        
        // Immediate second check should be blocked
        assert!(!detector.should_check("https://example.com").await);
        
        // Different URL should be allowed
        assert!(detector.should_check("https://other.com").await);
    }

    #[tokio::test]
    async fn test_notifications() {
        let detector = ChangeDetector::new();
        let archive = create_test_archive("old content", "abc123");
        let new_content = create_test_content("new content", "def456");
        
        // Check for changes (should create notification)
        let _result = detector.check_for_changes(&archive, &new_content).await;
        
        // Get notifications
        let notifications = detector.get_notifications().await;
        assert!(!notifications.is_empty());
        
        // Get unread notifications
        let unread = detector.get_unread_notifications().await;
        assert_eq!(unread.len(), notifications.len());
        
        // Mark as read
        if let Some(notification) = notifications.first() {
            detector.mark_notification_read(&notification.id).await;
        }
        
        let unread_after = detector.get_unread_notifications().await;
        assert!(unread_after.len() < unread.len());
    }

    #[tokio::test]
    async fn test_version_history() {
        let detector = ChangeDetector::new();
        let page_id = Uuid::new_v4();
        let archive_id = ArchiveId::new();
        
        // Add first version
        detector.add_version(
            &page_id,
            &archive_id,
            "checksum1",
            1000,
            ChangeType::NoChange,
        ).await;
        
        // Get version history
        let history = detector.get_version_history(&page_id).await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].version, 1);
        
        // Add second version
        let archive_id2 = ArchiveId::new();
        detector.add_version(
            &page_id,
            &archive_id2,
            "checksum2",
            1200,
            ChangeType::Minor,
        ).await;
        
        let history = detector.get_version_history(&page_id).await;
        assert_eq!(history.len(), 2);
        assert_eq!(history[1].version, 2);
        
        // Get latest version
        let latest = detector.get_latest_version(&page_id).await;
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().version, 2);
    }

    #[tokio::test]
    async fn test_stats() {
        let detector = ChangeDetector::new();
        
        // Initial stats
        let stats = detector.get_stats().await;
        assert_eq!(stats.total_notifications, 0);
        assert_eq!(stats.urls_tracked, 0);
        
        // Add some data
        let archive = create_test_archive("content", "abc");
        let new_content = create_test_content("different", "def");
        let _result = detector.check_for_changes(&archive, &new_content).await;
        
        let stats = detector.get_stats().await;
        assert!(stats.total_notifications > 0);
        assert!(stats.urls_tracked > 0);
    }

    // Helper functions for tests
    fn create_test_archive(content: &str, checksum: &str) -> ContentArchive {
        ContentArchive {
            id: ArchiveId::new(),
            page_id: Uuid::new_v4(),
            url: "https://example.com/page".to_string(),
            title: "Test Page".to_string(),
            content_html: format!("<p>{}</p>", content),
            content_text: content.to_string(),
            media_files: vec![],
            archived_at: Utc::now(),
            file_size: content.len() as u64,
            checksum: Some(checksum.to_string()),
        }
    }

    fn create_test_content(text: &str, checksum: &str) -> PageChangeContent {
        PageChangeContent {
            html: format!("<p>{}</p>", text),
            text: text.to_string(),
            title: "Test Page".to_string(),
            checksum: checksum.to_string(),
            fetched_at: Utc::now(),
        }
    }
}
