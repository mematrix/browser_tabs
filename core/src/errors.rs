use thiserror::Error;
use crate::types::BrowserType;
use uuid::Uuid;

/// Browser connection related errors
#[derive(Debug, Error)]
pub enum BrowserConnectionError {
    #[error("Browser not running: {browser:?}")]
    BrowserNotRunning { browser: BrowserType },
    
    #[error("Connection timeout: {browser:?}")]
    ConnectionTimeout { browser: BrowserType },
    
    #[error("Incompatible API version: {browser:?}, required version {required}")]
    IncompatibleVersion { browser: BrowserType, required: String },
    
    #[error("Permission denied: {browser:?}")]
    PermissionDenied { browser: BrowserType },
    
    #[error("Invalid response from browser: {browser:?}")]
    InvalidResponse { browser: BrowserType },
}

/// AI processing related errors
#[derive(Debug, Error)]
pub enum AIProcessingError {
    #[error("Content fetch failed: {url}")]
    ContentFetchFailed { url: String },
    
    #[error("Analysis timeout")]
    AnalysisTimeout,
    
    #[error("AI model load failed: {model}")]
    ModelLoadFailed { model: String },
    
    #[error("Unsupported content type: {content_type}")]
    UnsupportedContentType { content_type: String },
    
    #[error("Processing failed: {reason}")]
    ProcessingFailed { reason: String },
}

/// Data consistency related errors
#[derive(Debug, Error)]
pub enum DataConsistencyError {
    #[error("Page data conflict: {page_id}")]
    PageDataConflict { page_id: Uuid },
    
    #[error("Group relation inconsistent: {group_id}")]
    GroupRelationInconsistent { group_id: Uuid },
    
    #[error("History record corrupted: {history_id}")]
    HistoryCorrupted { history_id: Uuid },
    
    #[error("Database integrity violation: {details}")]
    DatabaseIntegrityViolation { details: String },
}

/// Performance and resource related errors
#[derive(Debug, Error)]
pub enum PerformanceError {
    #[error("Memory limit exceeded: {current_mb}MB > {limit_mb}MB")]
    MemoryLimitExceeded { current_mb: u64, limit_mb: u64 },
    
    #[error("Processing timeout: {operation} > {timeout_ms}ms")]
    ProcessingTimeout { operation: String, timeout_ms: u64 },
    
    #[error("Insufficient disk space: {available_mb}MB < {required_mb}MB")]
    InsufficientDiskSpace { available_mb: u64, required_mb: u64 },
    
    #[error("Resource unavailable: {resource}")]
    ResourceUnavailable { resource: String },
}

/// UI framework related errors
#[derive(Debug, Error)]
pub enum UIError {
    #[error("UI framework not initialized")]
    NotInitialized,
    
    #[error("UI operation failed: {operation}")]
    OperationFailed { operation: String },
    
    #[error("Unsupported UI framework: {framework}")]
    UnsupportedFramework { framework: String },
    
    #[error("Platform not supported: {platform}")]
    PlatformNotSupported { platform: String },
}

/// General system errors
#[derive(Debug, Error)]
pub enum SystemError {
    #[error("Configuration error: {details}")]
    Configuration { details: String },
    
    #[error("IO error: {source}")]
    IO {
        #[from]
        source: std::io::Error,
    },
    
    #[error("Serialization error: {source}")]
    Serialization {
        #[from]
        source: serde_json::Error,
    },
    
    #[error("Network error: {details}")]
    Network { details: String },
    
    #[error("Unknown error: {details}")]
    Unknown { details: String },
}

/// Main error type for the application
#[derive(Debug, Error)]
pub enum WebPageManagerError {
    #[error("Browser connection error: {source}")]
    BrowserConnection {
        #[from]
        source: BrowserConnectionError,
    },
    
    #[error("AI processing error: {source}")]
    AIProcessing {
        #[from]
        source: AIProcessingError,
    },
    
    #[error("Data consistency error: {source}")]
    DataConsistency {
        #[from]
        source: DataConsistencyError,
    },
    
    #[error("Performance error: {source}")]
    Performance {
        #[from]
        source: PerformanceError,
    },
    
    #[error("UI error: {source}")]
    UI {
        #[from]
        source: UIError,
    },
    
    #[error("System error: {source}")]
    System {
        #[from]
        source: SystemError,
    },
    
    #[error("Bookmark analysis error: {source}")]
    BookmarkAnalysis {
        #[from]
        source: BookmarkAnalysisError,
    },
    
    #[error("History error: {source}")]
    History {
        #[from]
        source: HistoryError,
    },
    
    #[error("Cross-browser error: {source}")]
    CrossBrowser {
        #[from]
        source: CrossBrowserError,
    },
    
    #[error("Archive error: {source}")]
    Archive {
        #[from]
        source: ArchiveError,
    },
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, WebPageManagerError>;

/// Bookmark analysis related errors
#[derive(Debug, Error)]
pub enum BookmarkAnalysisError {
    #[error("Bookmark not found: {bookmark_id}")]
    BookmarkNotFound { bookmark_id: String },
    
    #[error("Content extraction failed for URL: {url}")]
    ContentExtractionFailed { url: String },
    
    #[error("Batch analysis failed: {processed}/{total} bookmarks processed")]
    BatchAnalysisFailed { processed: usize, total: usize },
    
    #[error("Duplicate detection failed: {reason}")]
    DuplicateDetectionFailed { reason: String },
}

/// History management related errors
#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("History entry not found: {history_id}")]
    EntryNotFound { history_id: String },
    
    #[error("Failed to save history entry: {reason}")]
    SaveFailed { reason: String },
    
    #[error("Failed to restore tab: {reason}")]
    RestoreFailed { reason: String },
    
    #[error("Cleanup operation failed: {reason}")]
    CleanupFailed { reason: String },
}

/// Cross-browser operation related errors
#[derive(Debug, Error)]
pub enum CrossBrowserError {
    #[error("Migration failed from {source_browser:?} to {target_browser:?}: {reason}")]
    MigrationFailed {
        source_browser: BrowserType,
        target_browser: BrowserType,
        reason: String,
    },
    
    #[error("Session state could not be preserved: {reason}")]
    SessionStateError { reason: String },
    
    #[error("Rollback failed: {reason}")]
    RollbackFailed { reason: String },
    
    #[error("Operation not supported between {source_browser:?} and {target_browser:?}")]
    OperationNotSupported {
        source_browser: BrowserType,
        target_browser: BrowserType,
    },
}

/// Archive related errors
#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("Archive not found: {archive_id}")]
    ArchiveNotFound { archive_id: String },
    
    #[error("Content extraction failed: {reason}")]
    ExtractionFailed { reason: String },
    
    #[error("Media download failed: {url}")]
    MediaDownloadFailed { url: String },
    
    #[error("Storage limit exceeded: {current_mb}MB > {limit_mb}MB")]
    StorageLimitExceeded { current_mb: u64, limit_mb: u64 },
    
    #[error("Archive corrupted: {archive_id}")]
    ArchiveCorrupted { archive_id: String },
}