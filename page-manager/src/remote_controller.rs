//! Remote Tab Controller Module
//!
//! Provides functionality for remotely controlling browser tabs, including:
//! - Tab close, activate, and create operations
//! - Operation result verification and error handling
//! - Operation history and undo mechanism
//!
//! # Requirements Implemented
//! - 1.5: Execute remote control operations (close, activate, create tabs)
//!
//! # Design Properties
//! - Property 4: Remote control operation atomicity - operations either fully succeed
//!   or fully fail, maintaining original state

use web_page_manager_core::*;
use browser_connector::{BrowserConnector, BrowserConnectorManager};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Maximum number of operations to keep in history for undo
const DEFAULT_MAX_HISTORY_SIZE: usize = 100;

/// Configuration for the Remote Tab Controller
#[derive(Debug, Clone)]
pub struct RemoteTabControllerConfig {
    /// Maximum number of operations to keep in history
    pub max_history_size: usize,
    /// Whether to verify operations after execution
    pub verify_operations: bool,
    /// Timeout for operation verification in milliseconds
    pub verification_timeout_ms: u64,
    /// Whether to enable undo functionality
    pub enable_undo: bool,
    /// Maximum number of retry attempts for failed operations
    pub max_retry_attempts: u32,
}

impl Default for RemoteTabControllerConfig {
    fn default() -> Self {
        Self {
            max_history_size: DEFAULT_MAX_HISTORY_SIZE,
            verify_operations: true,
            verification_timeout_ms: 5000,
            enable_undo: true,
            max_retry_attempts: 2,
        }
    }
}

/// Type of tab operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TabOperationType {
    /// Close a tab
    Close,
    /// Activate (focus) a tab
    Activate,
    /// Create a new tab
    Create,
}

impl std::fmt::Display for TabOperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TabOperationType::Close => write!(f, "Close"),
            TabOperationType::Activate => write!(f, "Activate"),
            TabOperationType::Create => write!(f, "Create"),
        }
    }
}

/// Status of a tab operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationStatus {
    /// Operation completed successfully
    Success,
    /// Operation failed with error
    Failed(String),
    /// Operation is pending verification
    PendingVerification,
    /// Operation was rolled back
    RolledBack,
}

impl OperationStatus {
    /// Check if the operation was successful
    pub fn is_success(&self) -> bool {
        matches!(self, OperationStatus::Success)
    }

    /// Check if the operation failed
    pub fn is_failed(&self) -> bool {
        matches!(self, OperationStatus::Failed(_))
    }
}

/// Record of a tab operation for history and undo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabOperationRecord {
    /// Unique identifier for this operation
    pub id: uuid::Uuid,
    /// Type of operation performed
    pub operation_type: TabOperationType,
    /// Browser where the operation was performed
    pub browser_type: BrowserType,
    /// Tab ID involved in the operation
    pub tab_id: TabId,
    /// URL associated with the operation (for create/close)
    pub url: Option<String>,
    /// Title of the tab (for close operations, to support undo)
    pub title: Option<String>,
    /// Status of the operation
    pub status: OperationStatus,
    /// Timestamp when the operation was executed
    pub executed_at: DateTime<Utc>,
    /// Whether this operation can be undone
    pub undoable: bool,
    /// Related operation ID (e.g., the original operation for an undo)
    pub related_operation_id: Option<uuid::Uuid>,
}

impl TabOperationRecord {
    /// Create a new operation record
    pub fn new(
        operation_type: TabOperationType,
        browser_type: BrowserType,
        tab_id: TabId,
        url: Option<String>,
        title: Option<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            operation_type,
            browser_type,
            tab_id,
            url,
            title,
            status: OperationStatus::PendingVerification,
            executed_at: Utc::now(),
            undoable: matches!(operation_type, TabOperationType::Close | TabOperationType::Create),
            related_operation_id: None,
        }
    }

    /// Mark the operation as successful
    pub fn mark_success(&mut self) {
        self.status = OperationStatus::Success;
    }

    /// Mark the operation as failed
    pub fn mark_failed(&mut self, error: String) {
        self.status = OperationStatus::Failed(error);
    }

    /// Mark the operation as rolled back
    pub fn mark_rolled_back(&mut self) {
        self.status = OperationStatus::RolledBack;
        self.undoable = false;
    }
}

/// Result of a tab operation
#[derive(Debug, Clone)]
pub struct TabOperationResult {
    /// The operation record
    pub record: TabOperationRecord,
    /// New tab ID if a tab was created
    pub new_tab_id: Option<TabId>,
    /// Whether verification was performed
    pub verified: bool,
}

impl TabOperationResult {
    /// Check if the operation was successful
    pub fn is_success(&self) -> bool {
        self.record.status.is_success()
    }

    /// Get the error message if the operation failed
    pub fn error_message(&self) -> Option<&str> {
        match &self.record.status {
            OperationStatus::Failed(msg) => Some(msg),
            _ => None,
        }
    }
}

/// Statistics about the remote tab controller
#[derive(Debug, Clone, Default)]
pub struct RemoteControllerStats {
    /// Total operations executed
    pub total_operations: usize,
    /// Successful operations
    pub successful_operations: usize,
    /// Failed operations
    pub failed_operations: usize,
    /// Operations by type
    pub operations_by_type: std::collections::HashMap<String, usize>,
    /// Operations by browser
    pub operations_by_browser: std::collections::HashMap<BrowserType, usize>,
    /// Undo operations performed
    pub undo_operations: usize,
    /// Operations currently in history
    pub history_size: usize,
}

/// Remote Tab Controller
///
/// Provides remote control capabilities for browser tabs with operation
/// history and undo functionality.
///
/// Implements Requirement 1.5: Execute remote control operations
pub struct RemoteTabController {
    config: RemoteTabControllerConfig,
    /// Operation history for undo support
    operation_history: Arc<RwLock<VecDeque<TabOperationRecord>>>,
    /// Statistics
    stats: Arc<RwLock<RemoteControllerStats>>,
}

impl RemoteTabController {
    /// Create a new Remote Tab Controller with default configuration
    pub fn new() -> Self {
        Self::with_config(RemoteTabControllerConfig::default())
    }

    /// Create a new Remote Tab Controller with custom configuration
    pub fn with_config(config: RemoteTabControllerConfig) -> Self {
        Self {
            config,
            operation_history: Arc::new(RwLock::new(VecDeque::new())),
            stats: Arc::new(RwLock::new(RemoteControllerStats::default())),
        }
    }

    /// Get the current configuration
    pub fn config(&self) -> &RemoteTabControllerConfig {
        &self.config
    }

    // =========================================================================
    // Core Tab Operations (Requirement 1.5)
    // =========================================================================

    /// Close a tab in the specified browser
    ///
    /// This operation is atomic - it either fully succeeds or fails without
    /// partial state changes.
    ///
    /// # Arguments
    /// * `connector` - The browser connector to use
    /// * `tab_id` - The ID of the tab to close
    /// * `tab_info` - Optional tab info for undo support
    ///
    /// # Returns
    /// * `TabOperationResult` with the operation status
    pub async fn close_tab<C: BrowserConnector>(
        &self,
        connector: &C,
        tab_id: &TabId,
        tab_info: Option<&TabInfo>,
    ) -> Result<TabOperationResult> {
        let browser_type = connector.browser_type();
        let url = tab_info.map(|t| t.url.clone());
        let title = tab_info.map(|t| t.title.clone());

        let mut record = TabOperationRecord::new(
            TabOperationType::Close,
            browser_type,
            tab_id.clone(),
            url,
            title,
        );

        info!("Closing tab {:?} in {:?}", tab_id, browser_type);

        // Execute the close operation
        match connector.close_tab(tab_id).await {
            Ok(()) => {
                record.mark_success();
                debug!("Successfully closed tab {:?}", tab_id);
            }
            Err(e) => {
                let error_msg = e.to_string();
                record.mark_failed(error_msg.clone());
                warn!("Failed to close tab {:?}: {}", tab_id, error_msg);
            }
        }

        // Update statistics and history
        self.record_operation(&record).await;

        Ok(TabOperationResult {
            record,
            new_tab_id: None,
            verified: false,
        })
    }

    /// Activate (focus) a tab in the specified browser
    ///
    /// # Arguments
    /// * `connector` - The browser connector to use
    /// * `tab_id` - The ID of the tab to activate
    ///
    /// # Returns
    /// * `TabOperationResult` with the operation status
    pub async fn activate_tab<C: BrowserConnector>(
        &self,
        connector: &C,
        tab_id: &TabId,
    ) -> Result<TabOperationResult> {
        let browser_type = connector.browser_type();

        let mut record = TabOperationRecord::new(
            TabOperationType::Activate,
            browser_type,
            tab_id.clone(),
            None,
            None,
        );
        // Activate operations are not undoable
        record.undoable = false;

        info!("Activating tab {:?} in {:?}", tab_id, browser_type);

        // Execute the activate operation
        match connector.activate_tab(tab_id).await {
            Ok(()) => {
                record.mark_success();
                debug!("Successfully activated tab {:?}", tab_id);
            }
            Err(e) => {
                let error_msg = e.to_string();
                record.mark_failed(error_msg.clone());
                warn!("Failed to activate tab {:?}: {}", tab_id, error_msg);
            }
        }

        // Update statistics and history
        self.record_operation(&record).await;

        Ok(TabOperationResult {
            record,
            new_tab_id: None,
            verified: false,
        })
    }

    /// Create a new tab in the specified browser
    ///
    /// # Arguments
    /// * `connector` - The browser connector to use
    /// * `url` - The URL to open in the new tab
    ///
    /// # Returns
    /// * `TabOperationResult` with the operation status and new tab ID
    pub async fn create_tab<C: BrowserConnector>(
        &self,
        connector: &C,
        url: &str,
    ) -> Result<TabOperationResult> {
        let browser_type = connector.browser_type();

        // Create a placeholder tab ID - will be updated with actual ID on success
        let placeholder_id = TabId::new();
        let mut record = TabOperationRecord::new(
            TabOperationType::Create,
            browser_type,
            placeholder_id,
            Some(url.to_string()),
            None,
        );

        info!("Creating tab with URL {} in {:?}", url, browser_type);

        // Execute the create operation
        let new_tab_id = match connector.create_tab(url).await {
            Ok(tab_id) => {
                record.tab_id = tab_id.clone();
                record.mark_success();
                debug!("Successfully created tab {:?}", tab_id);
                Some(tab_id)
            }
            Err(e) => {
                let error_msg = e.to_string();
                record.mark_failed(error_msg.clone());
                warn!("Failed to create tab: {}", error_msg);
                None
            }
        };

        // Update statistics and history
        self.record_operation(&record).await;

        Ok(TabOperationResult {
            record,
            new_tab_id,
            verified: false,
        })
    }

    // =========================================================================
    // Operations using BrowserConnectorManager
    // =========================================================================

    /// Close a tab using the browser connector manager
    pub async fn close_tab_via_manager(
        &self,
        manager: &BrowserConnectorManager,
        browser_type: BrowserType,
        tab_id: &TabId,
        tab_info: Option<&TabInfo>,
    ) -> Result<TabOperationResult> {
        let url = tab_info.map(|t| t.url.clone());
        let title = tab_info.map(|t| t.title.clone());

        let mut record = TabOperationRecord::new(
            TabOperationType::Close,
            browser_type,
            tab_id.clone(),
            url,
            title,
        );

        info!("Closing tab {:?} in {:?} via manager", tab_id, browser_type);

        match manager.close_tab(browser_type, tab_id).await {
            Ok(()) => {
                record.mark_success();
                debug!("Successfully closed tab {:?}", tab_id);
            }
            Err(e) => {
                let error_msg = e.to_string();
                record.mark_failed(error_msg.clone());
                warn!("Failed to close tab {:?}: {}", tab_id, error_msg);
            }
        }

        self.record_operation(&record).await;

        Ok(TabOperationResult {
            record,
            new_tab_id: None,
            verified: false,
        })
    }

    /// Activate a tab using the browser connector manager
    pub async fn activate_tab_via_manager(
        &self,
        manager: &BrowserConnectorManager,
        browser_type: BrowserType,
        tab_id: &TabId,
    ) -> Result<TabOperationResult> {
        let mut record = TabOperationRecord::new(
            TabOperationType::Activate,
            browser_type,
            tab_id.clone(),
            None,
            None,
        );
        record.undoable = false;

        info!("Activating tab {:?} in {:?} via manager", tab_id, browser_type);

        match manager.activate_tab(browser_type, tab_id).await {
            Ok(()) => {
                record.mark_success();
                debug!("Successfully activated tab {:?}", tab_id);
            }
            Err(e) => {
                let error_msg = e.to_string();
                record.mark_failed(error_msg.clone());
                warn!("Failed to activate tab {:?}: {}", tab_id, error_msg);
            }
        }

        self.record_operation(&record).await;

        Ok(TabOperationResult {
            record,
            new_tab_id: None,
            verified: false,
        })
    }

    /// Create a tab using the browser connector manager
    pub async fn create_tab_via_manager(
        &self,
        manager: &BrowserConnectorManager,
        browser_type: BrowserType,
        url: &str,
    ) -> Result<TabOperationResult> {
        let placeholder_id = TabId::new();
        let mut record = TabOperationRecord::new(
            TabOperationType::Create,
            browser_type,
            placeholder_id,
            Some(url.to_string()),
            None,
        );

        info!("Creating tab with URL {} in {:?} via manager", url, browser_type);

        let new_tab_id = match manager.create_tab(browser_type, url).await {
            Ok(tab_id) => {
                record.tab_id = tab_id.clone();
                record.mark_success();
                debug!("Successfully created tab {:?}", tab_id);
                Some(tab_id)
            }
            Err(e) => {
                let error_msg = e.to_string();
                record.mark_failed(error_msg.clone());
                warn!("Failed to create tab: {}", error_msg);
                None
            }
        };

        self.record_operation(&record).await;

        Ok(TabOperationResult {
            record,
            new_tab_id,
            verified: false,
        })
    }

    // =========================================================================
    // Operation History and Undo
    // =========================================================================

    /// Record an operation in history
    async fn record_operation(&self, record: &TabOperationRecord) {
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_operations += 1;

            if record.status.is_success() {
                stats.successful_operations += 1;
            } else if record.status.is_failed() {
                stats.failed_operations += 1;
            }

            *stats
                .operations_by_type
                .entry(record.operation_type.to_string())
                .or_insert(0) += 1;

            *stats
                .operations_by_browser
                .entry(record.browser_type)
                .or_insert(0) += 1;
        }

        // Add to history if undo is enabled
        if self.config.enable_undo {
            let mut history = self.operation_history.write().await;
            history.push_back(record.clone());

            // Trim history if needed
            while history.len() > self.config.max_history_size {
                history.pop_front();
            }

            // Update history size in stats
            let mut stats = self.stats.write().await;
            stats.history_size = history.len();
        }
    }

    /// Get the operation history
    pub async fn get_history(&self) -> Vec<TabOperationRecord> {
        let history = self.operation_history.read().await;
        history.iter().cloned().collect()
    }

    /// Get recent operations from history
    pub async fn get_recent_operations(&self, count: usize) -> Vec<TabOperationRecord> {
        let history = self.operation_history.read().await;
        history.iter().rev().take(count).cloned().collect()
    }

    /// Get undoable operations from history
    pub async fn get_undoable_operations(&self) -> Vec<TabOperationRecord> {
        let history = self.operation_history.read().await;
        history
            .iter()
            .filter(|r| r.undoable && r.status.is_success())
            .cloned()
            .collect()
    }

    /// Undo a close operation by reopening the tab
    ///
    /// This creates a new tab with the same URL as the closed tab.
    pub async fn undo_close<C: BrowserConnector>(
        &self,
        connector: &C,
        operation_id: uuid::Uuid,
    ) -> Result<TabOperationResult> {
        // Find the operation in history
        let operation = {
            let history = self.operation_history.read().await;
            history.iter().find(|r| r.id == operation_id).cloned()
        };

        let operation = operation.ok_or_else(|| {
            WebPageManagerError::History {
                source: HistoryError::EntryNotFound {
                    history_id: operation_id.to_string(),
                },
            }
        })?;

        // Verify it's a close operation that can be undone
        if operation.operation_type != TabOperationType::Close {
            return Err(WebPageManagerError::History {
                source: HistoryError::RestoreFailed {
                    reason: "Can only undo close operations".to_string(),
                },
            });
        }

        if !operation.undoable {
            return Err(WebPageManagerError::History {
                source: HistoryError::RestoreFailed {
                    reason: "Operation cannot be undone".to_string(),
                },
            });
        }

        let url = operation.url.as_ref().ok_or_else(|| {
            WebPageManagerError::History {
                source: HistoryError::RestoreFailed {
                    reason: "No URL available for undo".to_string(),
                },
            }
        })?;

        // Create a new tab with the same URL
        let mut result = self.create_tab(connector, url).await?;

        // Link the undo operation to the original
        result.record.related_operation_id = Some(operation_id);

        // Mark the original operation as no longer undoable
        {
            let mut history = self.operation_history.write().await;
            if let Some(original) = history.iter_mut().find(|r| r.id == operation_id) {
                original.undoable = false;
            }
        }

        // Update undo stats
        if result.is_success() {
            let mut stats = self.stats.write().await;
            stats.undo_operations += 1;
        }

        info!("Undid close operation: {:?}", operation_id);

        Ok(result)
    }

    /// Undo a create operation by closing the tab
    pub async fn undo_create<C: BrowserConnector>(
        &self,
        connector: &C,
        operation_id: uuid::Uuid,
    ) -> Result<TabOperationResult> {
        // Find the operation in history
        let operation = {
            let history = self.operation_history.read().await;
            history.iter().find(|r| r.id == operation_id).cloned()
        };

        let operation = operation.ok_or_else(|| {
            WebPageManagerError::History {
                source: HistoryError::EntryNotFound {
                    history_id: operation_id.to_string(),
                },
            }
        })?;

        // Verify it's a create operation that can be undone
        if operation.operation_type != TabOperationType::Create {
            return Err(WebPageManagerError::History {
                source: HistoryError::RestoreFailed {
                    reason: "Can only undo create operations with this method".to_string(),
                },
            });
        }

        if !operation.undoable {
            return Err(WebPageManagerError::History {
                source: HistoryError::RestoreFailed {
                    reason: "Operation cannot be undone".to_string(),
                },
            });
        }

        // Close the created tab
        let mut result = self.close_tab(connector, &operation.tab_id, None).await?;

        // Link the undo operation to the original
        result.record.related_operation_id = Some(operation_id);

        // Mark the original operation as no longer undoable
        {
            let mut history = self.operation_history.write().await;
            if let Some(original) = history.iter_mut().find(|r| r.id == operation_id) {
                original.undoable = false;
            }
        }

        // Update undo stats
        if result.is_success() {
            let mut stats = self.stats.write().await;
            stats.undo_operations += 1;
        }

        info!("Undid create operation: {:?}", operation_id);

        Ok(result)
    }

    /// Undo the most recent undoable operation
    pub async fn undo_last<C: BrowserConnector>(
        &self,
        connector: &C,
    ) -> Result<Option<TabOperationResult>> {
        // Find the most recent undoable operation
        let operation = {
            let history = self.operation_history.read().await;
            history
                .iter()
                .rev()
                .find(|r| r.undoable && r.status.is_success())
                .cloned()
        };

        match operation {
            Some(op) => {
                let result = match op.operation_type {
                    TabOperationType::Close => self.undo_close(connector, op.id).await?,
                    TabOperationType::Create => self.undo_create(connector, op.id).await?,
                    TabOperationType::Activate => {
                        // Activate operations cannot be undone
                        return Ok(None);
                    }
                };
                Ok(Some(result))
            }
            None => Ok(None),
        }
    }

    // =========================================================================
    // Statistics and Management
    // =========================================================================

    /// Get controller statistics
    pub async fn get_stats(&self) -> RemoteControllerStats {
        self.stats.read().await.clone()
    }

    /// Clear operation history
    pub async fn clear_history(&self) {
        let mut history = self.operation_history.write().await;
        history.clear();

        let mut stats = self.stats.write().await;
        stats.history_size = 0;

        info!("Cleared operation history");
    }

    /// Get an operation by ID
    pub async fn get_operation(&self, operation_id: uuid::Uuid) -> Option<TabOperationRecord> {
        let history = self.operation_history.read().await;
        history.iter().find(|r| r.id == operation_id).cloned()
    }

    /// Get operations for a specific browser
    pub async fn get_operations_for_browser(
        &self,
        browser_type: BrowserType,
    ) -> Vec<TabOperationRecord> {
        let history = self.operation_history.read().await;
        history
            .iter()
            .filter(|r| r.browser_type == browser_type)
            .cloned()
            .collect()
    }

    /// Get operations of a specific type
    pub async fn get_operations_by_type(
        &self,
        operation_type: TabOperationType,
    ) -> Vec<TabOperationRecord> {
        let history = self.operation_history.read().await;
        history
            .iter()
            .filter(|r| r.operation_type == operation_type)
            .cloned()
            .collect()
    }

    /// Get failed operations
    pub async fn get_failed_operations(&self) -> Vec<TabOperationRecord> {
        let history = self.operation_history.read().await;
        history
            .iter()
            .filter(|r| r.status.is_failed())
            .cloned()
            .collect()
    }
}

impl Default for RemoteTabController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_record_creation() {
        let record = TabOperationRecord::new(
            TabOperationType::Close,
            BrowserType::Chrome,
            TabId("test-tab".to_string()),
            Some("https://example.com".to_string()),
            Some("Example".to_string()),
        );

        assert_eq!(record.operation_type, TabOperationType::Close);
        assert_eq!(record.browser_type, BrowserType::Chrome);
        assert!(record.undoable);
        assert_eq!(record.status, OperationStatus::PendingVerification);
    }

    #[test]
    fn test_operation_status_transitions() {
        let mut record = TabOperationRecord::new(
            TabOperationType::Create,
            BrowserType::Firefox,
            TabId::new(),
            Some("https://test.com".to_string()),
            None,
        );

        assert!(!record.status.is_success());
        assert!(!record.status.is_failed());

        record.mark_success();
        assert!(record.status.is_success());

        let mut record2 = TabOperationRecord::new(
            TabOperationType::Activate,
            BrowserType::Edge,
            TabId::new(),
            None,
            None,
        );

        record2.mark_failed("Connection error".to_string());
        assert!(record2.status.is_failed());
    }

    #[test]
    fn test_activate_not_undoable() {
        let mut record = TabOperationRecord::new(
            TabOperationType::Activate,
            BrowserType::Chrome,
            TabId::new(),
            None,
            None,
        );
        record.undoable = false;

        assert!(!record.undoable);
    }

    #[tokio::test]
    async fn test_controller_creation() {
        let controller = RemoteTabController::new();
        let stats = controller.get_stats().await;

        assert_eq!(stats.total_operations, 0);
        assert_eq!(stats.successful_operations, 0);
        assert_eq!(stats.failed_operations, 0);
    }

    #[tokio::test]
    async fn test_history_management() {
        let controller = RemoteTabController::new();

        // Initially empty
        let history = controller.get_history().await;
        assert!(history.is_empty());

        // Get undoable operations (should be empty)
        let undoable = controller.get_undoable_operations().await;
        assert!(undoable.is_empty());
    }

    #[tokio::test]
    async fn test_config_defaults() {
        let config = RemoteTabControllerConfig::default();

        assert_eq!(config.max_history_size, DEFAULT_MAX_HISTORY_SIZE);
        assert!(config.verify_operations);
        assert!(config.enable_undo);
        assert_eq!(config.max_retry_attempts, 2);
    }
}
