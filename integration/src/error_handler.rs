/// Unified error handler for centralized error management

use web_page_manager_core::errors::WebPageManagerError;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, warn, info};

/// Error severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Critical error requiring immediate attention
    Critical,
    /// Error that affects functionality
    Error,
    /// Warning about potential issues
    Warning,
    /// Informational message
    Info,
}

/// Error entry for tracking
#[derive(Debug, Clone)]
pub struct ErrorEntry {
    pub error: String,
    pub severity: ErrorSeverity,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub context: String,
}

/// Unified error handler
pub struct UnifiedErrorHandler {
    /// Recent errors for reporting
    recent_errors: Arc<RwLock<Vec<ErrorEntry>>>,
    /// Maximum number of errors to keep
    max_errors: usize,
}

impl UnifiedErrorHandler {
    /// Create a new error handler
    pub fn new() -> Self {
        Self {
            recent_errors: Arc::new(RwLock::new(Vec::new())),
            max_errors: 100,
        }
    }

    /// Handle an error with automatic logging
    pub async fn handle_error(
        &self,
        error: &WebPageManagerError,
        context: &str,
    ) {
        let severity = self.classify_error(error);

        // Log based on severity
        match severity {
            ErrorSeverity::Critical => {
                error!("CRITICAL ERROR in {}: {}", context, error);
            }
            ErrorSeverity::Error => {
                error!("ERROR in {}: {}", context, error);
            }
            ErrorSeverity::Warning => {
                warn!("WARNING in {}: {}", context, error);
            }
            ErrorSeverity::Info => {
                info!("INFO in {}: {}", context, error);
            }
        }

        // Record error
        let entry = ErrorEntry {
            error: error.to_string(),
            severity,
            timestamp: chrono::Utc::now(),
            context: context.to_string(),
        };

        self.add_error_entry(entry).await;
    }

    /// Classify error severity
    fn classify_error(&self, error: &WebPageManagerError) -> ErrorSeverity {
        use WebPageManagerError::*;

        match error {
            BrowserConnectionError(_) => ErrorSeverity::Warning,
            AIProcessingError(_) => ErrorSeverity::Error,
            DataConsistency(_) => ErrorSeverity::Critical,
            PerformanceError(_) => ErrorSeverity::Warning,
            UIError(_) => ErrorSeverity::Error,
            SystemError(_) => ErrorSeverity::Critical,
            BookmarkAnalysis(_) => ErrorSeverity::Warning,
            HistoryError(_) => ErrorSeverity::Error,
            CrossBrowserError(_) => ErrorSeverity::Warning,
            ArchiveError(_) => ErrorSeverity::Warning,
        }
    }

    /// Add an error entry to the history
    async fn add_error_entry(&self, entry: ErrorEntry) {
        let mut errors = self.recent_errors.write().await;
        errors.push(entry);

        // Keep only recent errors
        if errors.len() > self.max_errors {
            let excess = errors.len() - self.max_errors;
            errors.drain(0..excess);
        }
    }

    /// Get recent errors
    pub async fn get_recent_errors(&self) -> Vec<ErrorEntry> {
        self.recent_errors.read().await.clone()
    }

    /// Get error statistics
    pub async fn get_error_stats(&self) -> ErrorStatistics {
        let errors = self.recent_errors.read().await;

        let mut stats = ErrorStatistics {
            total: errors.len(),
            critical: 0,
            errors: 0,
            warnings: 0,
            info: 0,
        };

        for error in errors.iter() {
            match error.severity {
                ErrorSeverity::Critical => stats.critical += 1,
                ErrorSeverity::Error => stats.errors += 1,
                ErrorSeverity::Warning => stats.warnings += 1,
                ErrorSeverity::Info => stats.info += 1,
            }
        }

        stats
    }

    /// Clear error history
    pub async fn clear_errors(&self) {
        self.recent_errors.write().await.clear();
    }
}

impl Default for UnifiedErrorHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Error statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ErrorStatistics {
    pub total: usize,
    pub critical: usize,
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use web_page_manager_core::errors::BrowserConnectionError;

    #[tokio::test]
    async fn test_error_handler_creation() {
        let handler = UnifiedErrorHandler::new();
        let stats = handler.get_error_stats().await;
        assert_eq!(stats.total, 0);
    }

    #[tokio::test]
    async fn test_handle_error() {
        let handler = UnifiedErrorHandler::new();
        let error = WebPageManagerError::BrowserConnectionError(
            BrowserConnectionError::ConnectionFailed("test".to_string())
        );

        handler.handle_error(&error, "test_context").await;

        let errors = handler.get_recent_errors().await;
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].context, "test_context");
    }
}
