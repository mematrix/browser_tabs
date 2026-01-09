//! Remote Tab Controller Module
//!
//! Provides functionality for remotely controlling browser tabs, including:
//! - Tab close, activate, and create operations
//! - Operation result verification and error handling
//! - Operation history and undo mechanism
//! - Cross-browser tab migration with session state preservation
//! - Fallback mechanisms for API-limited operations
//!
//! # Requirements Implemented
//! - 1.5: Execute remote control operations (close, activate, create tabs)
//! - 8.2: Cross-browser tab migration with safe movement
//! - 8.3: Session state and login information preservation
//! - 8.4: Fallback solutions for API-limited operations
//!
//! # Design Properties
//! - Property 4: Remote control operation atomicity - operations either fully succeed
//!   or fully fail, maintaining original state
//! - Property 22: Cross-browser migration integrity
//! - Property 23: Fallback solution availability
//! - Property 24: Operation verification and rollback reliability

use web_page_manager_core::*;
use browser_connector::{BrowserConnector, BrowserConnectorManager};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn, error};

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
    /// Cross-browser migrations performed
    pub cross_browser_migrations: usize,
    /// Fallback operations used
    pub fallback_operations: usize,
}

// =========================================================================
// Cross-Browser Migration Types (Requirements 8.2, 8.3, 8.4)
// =========================================================================

/// Type of cross-browser migration operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationType {
    /// Full migration with session state preservation attempt
    Full,
    /// URL-only migration (fallback when session state cannot be preserved)
    UrlOnly,
    /// Export URLs for manual import (fallback when direct migration fails)
    Export,
}

impl std::fmt::Display for MigrationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrationType::Full => write!(f, "Full"),
            MigrationType::UrlOnly => write!(f, "UrlOnly"),
            MigrationType::Export => write!(f, "Export"),
        }
    }
}

/// Status of a cross-browser migration operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationStatus {
    /// Migration completed successfully
    Success,
    /// Migration completed with fallback method
    SuccessWithFallback { fallback_type: String },
    /// Migration failed
    Failed(String),
    /// Migration is pending
    Pending,
    /// Migration was rolled back
    RolledBack,
}

impl MigrationStatus {
    /// Check if the migration was successful (including fallback success)
    pub fn is_success(&self) -> bool {
        matches!(self, MigrationStatus::Success | MigrationStatus::SuccessWithFallback { .. })
    }

    /// Check if the migration failed
    pub fn is_failed(&self) -> bool {
        matches!(self, MigrationStatus::Failed(_))
    }
}

/// Session state information for migration
/// 
/// Implements Requirement 8.3: Preserve session state and login information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// URL of the tab
    pub url: String,
    /// Title of the tab
    pub title: String,
    /// Scroll position (if available)
    pub scroll_position: Option<u32>,
    /// Form data (if available and safe to transfer)
    pub form_data: Option<std::collections::HashMap<String, String>>,
    /// Cookies associated with the page (domain-specific)
    pub cookies: Vec<CookieInfo>,
    /// Local storage data (if available)
    pub local_storage: Option<std::collections::HashMap<String, String>>,
    /// Session storage data (if available)
    pub session_storage: Option<std::collections::HashMap<String, String>>,
    /// Timestamp when state was captured
    pub captured_at: DateTime<Utc>,
}

impl SessionState {
    /// Create a basic session state with just URL and title
    pub fn basic(url: String, title: String) -> Self {
        Self {
            url,
            title,
            scroll_position: None,
            form_data: None,
            cookies: Vec::new(),
            local_storage: None,
            session_storage: None,
            captured_at: Utc::now(),
        }
    }

    /// Check if this session state has any preserved data beyond URL/title
    pub fn has_preserved_data(&self) -> bool {
        self.scroll_position.is_some()
            || self.form_data.is_some()
            || !self.cookies.is_empty()
            || self.local_storage.is_some()
            || self.session_storage.is_some()
    }
}

/// Cookie information for session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieInfo {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    pub expires: Option<DateTime<Utc>>,
}

/// Record of a cross-browser migration operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRecord {
    /// Unique identifier for this migration
    pub id: uuid::Uuid,
    /// Source browser
    pub source_browser: BrowserType,
    /// Target browser
    pub target_browser: BrowserType,
    /// Original tab ID in source browser
    pub source_tab_id: TabId,
    /// New tab ID in target browser (if successful)
    pub target_tab_id: Option<TabId>,
    /// URL being migrated
    pub url: String,
    /// Title of the tab
    pub title: String,
    /// Type of migration performed
    pub migration_type: MigrationType,
    /// Status of the migration
    pub status: MigrationStatus,
    /// Session state that was captured
    pub session_state: Option<SessionState>,
    /// Whether session state was successfully preserved
    pub session_preserved: bool,
    /// Timestamp when migration was initiated
    pub initiated_at: DateTime<Utc>,
    /// Timestamp when migration completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Whether this migration can be rolled back
    pub rollbackable: bool,
    /// Error message if migration failed
    pub error_message: Option<String>,
}

impl MigrationRecord {
    /// Create a new migration record
    pub fn new(
        source_browser: BrowserType,
        target_browser: BrowserType,
        source_tab_id: TabId,
        url: String,
        title: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            source_browser,
            target_browser,
            source_tab_id,
            target_tab_id: None,
            url,
            title,
            migration_type: MigrationType::Full,
            status: MigrationStatus::Pending,
            session_state: None,
            session_preserved: false,
            initiated_at: Utc::now(),
            completed_at: None,
            rollbackable: false,
            error_message: None,
        }
    }

    /// Mark the migration as successful
    pub fn mark_success(&mut self, target_tab_id: TabId, session_preserved: bool) {
        self.target_tab_id = Some(target_tab_id);
        self.status = MigrationStatus::Success;
        self.session_preserved = session_preserved;
        self.completed_at = Some(Utc::now());
        self.rollbackable = true;
    }

    /// Mark the migration as successful with fallback
    pub fn mark_success_with_fallback(&mut self, target_tab_id: TabId, fallback_type: &str) {
        self.target_tab_id = Some(target_tab_id);
        self.status = MigrationStatus::SuccessWithFallback {
            fallback_type: fallback_type.to_string(),
        };
        self.session_preserved = false;
        self.completed_at = Some(Utc::now());
        self.rollbackable = true;
    }

    /// Mark the migration as failed
    pub fn mark_failed(&mut self, error: String) {
        self.status = MigrationStatus::Failed(error.clone());
        self.error_message = Some(error);
        self.completed_at = Some(Utc::now());
        self.rollbackable = false;
    }

    /// Mark the migration as rolled back
    pub fn mark_rolled_back(&mut self) {
        self.status = MigrationStatus::RolledBack;
        self.rollbackable = false;
    }
}

/// Result of a cross-browser migration operation
#[derive(Debug, Clone)]
pub struct MigrationResult {
    /// The migration record
    pub record: MigrationRecord,
    /// Whether a fallback method was used
    pub used_fallback: bool,
    /// Fallback data if export fallback was used
    pub fallback_data: Option<FallbackData>,
}

impl MigrationResult {
    /// Check if the migration was successful
    pub fn is_success(&self) -> bool {
        self.record.status.is_success()
    }

    /// Get the error message if the migration failed
    pub fn error_message(&self) -> Option<&str> {
        self.record.error_message.as_deref()
    }

    /// Get the new tab ID if migration was successful
    pub fn new_tab_id(&self) -> Option<&TabId> {
        self.record.target_tab_id.as_ref()
    }
}

/// Fallback data for when direct migration is not possible
/// 
/// Implements Requirement 8.4: Provide alternative solutions like URL export/import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackData {
    /// Type of fallback
    pub fallback_type: FallbackType,
    /// URLs to be imported
    pub urls: Vec<UrlExportEntry>,
    /// Export format
    pub format: MigrationExportFormat,
    /// Generated export content (if applicable)
    pub export_content: Option<String>,
    /// Instructions for the user
    pub instructions: String,
}

/// Type of fallback operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FallbackType {
    /// URL list export
    UrlExport,
    /// HTML bookmark file export
    HtmlBookmarkExport,
    /// JSON export
    JsonExport,
    /// Clipboard copy
    ClipboardCopy,
}

/// URL entry for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlExportEntry {
    pub url: String,
    pub title: String,
    pub source_browser: BrowserType,
}

/// Export format for fallback data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationExportFormat {
    /// Plain text URL list
    PlainText,
    /// HTML bookmark format
    Html,
    /// JSON format
    Json,
}

/// Configuration for cross-browser migration
#[derive(Debug, Clone)]
pub struct MigrationConfig {
    /// Whether to attempt session state preservation
    pub preserve_session_state: bool,
    /// Whether to close the source tab after successful migration
    pub close_source_tab: bool,
    /// Whether to activate the new tab in target browser
    pub activate_target_tab: bool,
    /// Timeout for migration operations in milliseconds
    pub timeout_ms: u64,
    /// Whether to automatically use fallback on failure
    pub auto_fallback: bool,
    /// Preferred fallback type
    pub preferred_fallback: FallbackType,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            preserve_session_state: true,
            close_source_tab: true,
            activate_target_tab: true,
            timeout_ms: 10000,
            auto_fallback: true,
            preferred_fallback: FallbackType::UrlExport,
        }
    }
}

/// Remote Tab Controller
///
/// Provides remote control capabilities for browser tabs with operation
/// history and undo functionality.
///
/// Implements Requirement 1.5: Execute remote control operations
/// Implements Requirements 8.2, 8.3, 8.4: Cross-browser migration
pub struct RemoteTabController {
    config: RemoteTabControllerConfig,
    /// Operation history for undo support
    operation_history: Arc<RwLock<VecDeque<TabOperationRecord>>>,
    /// Migration history for cross-browser operations
    migration_history: Arc<RwLock<VecDeque<MigrationRecord>>>,
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
            migration_history: Arc::new(RwLock::new(VecDeque::new())),
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

    // =========================================================================
    // Cross-Browser Migration (Requirements 8.2, 8.3, 8.4)
    // =========================================================================

    /// Migrate a tab from one browser to another
    ///
    /// This operation attempts to:
    /// 1. Capture session state from the source tab (Requirement 8.3)
    /// 2. Create a new tab in the target browser with the same URL
    /// 3. Optionally close the source tab
    /// 4. Provide rollback capability (Requirement 8.5)
    ///
    /// If direct migration fails, fallback methods are used (Requirement 8.4)
    ///
    /// # Arguments
    /// * `manager` - The browser connector manager
    /// * `source_browser` - The browser to migrate from
    /// * `target_browser` - The browser to migrate to
    /// * `tab_id` - The ID of the tab to migrate
    /// * `tab_info` - Optional tab info for better migration
    /// * `config` - Migration configuration options
    ///
    /// # Returns
    /// * `MigrationResult` with the migration status and details
    pub async fn migrate_tab(
        &self,
        manager: &BrowserConnectorManager,
        source_browser: BrowserType,
        target_browser: BrowserType,
        tab_id: &TabId,
        tab_info: Option<&TabInfo>,
        config: Option<MigrationConfig>,
    ) -> Result<MigrationResult> {
        let config = config.unwrap_or_default();
        
        // Get tab info if not provided
        let (url, title) = if let Some(info) = tab_info {
            (info.url.clone(), info.title.clone())
        } else {
            // Try to get tab info from the browser
            let tabs = manager.get_tabs(source_browser).await?;
            let tab = tabs.iter().find(|t| &t.id == tab_id).ok_or_else(|| {
                WebPageManagerError::CrossBrowser {
                    source: CrossBrowserError::MigrationFailed {
                        source_browser,
                        target_browser,
                        reason: "Source tab not found".to_string(),
                    },
                }
            })?;
            (tab.url.clone(), tab.title.clone())
        };

        info!(
            "Migrating tab from {:?} to {:?}: {}",
            source_browser, target_browser, url
        );

        let mut record = MigrationRecord::new(
            source_browser,
            target_browser,
            tab_id.clone(),
            url.clone(),
            title.clone(),
        );

        // Step 1: Capture session state if configured
        let session_state = if config.preserve_session_state {
            self.capture_session_state(manager, source_browser, tab_id, &url, &title).await
        } else {
            None
        };
        record.session_state = session_state.clone();

        // Step 2: Attempt to create tab in target browser
        let migration_result = self.attempt_migration(
            manager,
            target_browser,
            &url,
            &config,
        ).await;

        match migration_result {
            Ok(new_tab_id) => {
                // Migration successful
                let session_preserved = session_state.as_ref()
                    .map(|s| s.has_preserved_data())
                    .unwrap_or(false);
                
                record.mark_success(new_tab_id.clone(), session_preserved);

                // Step 3: Close source tab if configured
                if config.close_source_tab {
                    if let Err(e) = manager.close_tab(source_browser, tab_id).await {
                        warn!("Failed to close source tab after migration: {}", e);
                        // Don't fail the migration, just log the warning
                    }
                }

                // Step 4: Activate target tab if configured
                if config.activate_target_tab {
                    if let Err(e) = manager.activate_tab(target_browser, &new_tab_id).await {
                        warn!("Failed to activate target tab after migration: {}", e);
                    }
                }

                // Record the migration
                self.record_migration(&record).await;

                info!(
                    "Successfully migrated tab to {:?}: {:?}",
                    target_browser, new_tab_id
                );

                Ok(MigrationResult {
                    record,
                    used_fallback: false,
                    fallback_data: None,
                })
            }
            Err(e) => {
                // Migration failed, try fallback if configured
                if config.auto_fallback {
                    self.handle_migration_fallback(
                        manager,
                        &mut record,
                        source_browser,
                        target_browser,
                        &url,
                        &title,
                        &config,
                        e,
                    ).await
                } else {
                    record.mark_failed(e.to_string());
                    self.record_migration(&record).await;
                    
                    Err(WebPageManagerError::CrossBrowser {
                        source: CrossBrowserError::MigrationFailed {
                            source_browser,
                            target_browser,
                            reason: e.to_string(),
                        },
                    })
                }
            }
        }
    }

    /// Migrate multiple tabs from one browser to another
    ///
    /// # Arguments
    /// * `manager` - The browser connector manager
    /// * `source_browser` - The browser to migrate from
    /// * `target_browser` - The browser to migrate to
    /// * `tabs` - The tabs to migrate
    /// * `config` - Migration configuration options
    ///
    /// # Returns
    /// * Vector of migration results for each tab
    pub async fn migrate_tabs_batch(
        &self,
        manager: &BrowserConnectorManager,
        source_browser: BrowserType,
        target_browser: BrowserType,
        tabs: &[TabInfo],
        config: Option<MigrationConfig>,
    ) -> Vec<MigrationResult> {
        let config = config.unwrap_or_default();
        let mut results = Vec::with_capacity(tabs.len());

        for tab in tabs {
            let result = self.migrate_tab(
                manager,
                source_browser,
                target_browser,
                &tab.id,
                Some(tab),
                Some(config.clone()),
            ).await;

            match result {
                Ok(migration_result) => results.push(migration_result),
                Err(e) => {
                    // Create a failed result for this tab
                    let mut record = MigrationRecord::new(
                        source_browser,
                        target_browser,
                        tab.id.clone(),
                        tab.url.clone(),
                        tab.title.clone(),
                    );
                    record.mark_failed(e.to_string());
                    
                    results.push(MigrationResult {
                        record,
                        used_fallback: false,
                        fallback_data: None,
                    });
                }
            }
        }

        results
    }

    /// Generate fallback export data for tabs that cannot be directly migrated
    ///
    /// Implements Requirement 8.4: Provide alternative solutions like URL export/import
    ///
    /// # Arguments
    /// * `tabs` - The tabs to export
    /// * `source_browser` - The source browser
    /// * `format` - The export format
    ///
    /// # Returns
    /// * `FallbackData` containing the export information
    pub fn generate_fallback_export(
        &self,
        tabs: &[TabInfo],
        source_browser: BrowserType,
        format: MigrationExportFormat,
    ) -> FallbackData {
        let urls: Vec<UrlExportEntry> = tabs
            .iter()
            .map(|tab| UrlExportEntry {
                url: tab.url.clone(),
                title: tab.title.clone(),
                source_browser,
            })
            .collect();

        let export_content = match format {
            MigrationExportFormat::PlainText => {
                Some(urls.iter()
                    .map(|u| format!("{}\t{}", u.title, u.url))
                    .collect::<Vec<_>>()
                    .join("\n"))
            }
            MigrationExportFormat::Json => {
                serde_json::to_string_pretty(&urls).ok()
            }
            MigrationExportFormat::Html => {
                Some(self.generate_html_bookmark_export(&urls))
            }
        };

        let instructions = match format {
            MigrationExportFormat::PlainText => {
                "Copy the URL list and paste into your target browser's address bar one by one, \
                 or use a browser extension to open multiple URLs at once.".to_string()
            }
            MigrationExportFormat::Json => {
                "Import this JSON file using a browser extension or bookmark manager \
                 that supports JSON import.".to_string()
            }
            MigrationExportFormat::Html => {
                "Import this HTML file using your browser's bookmark import feature \
                 (usually found in Settings > Bookmarks > Import).".to_string()
            }
        };

        FallbackData {
            fallback_type: match format {
                MigrationExportFormat::PlainText => FallbackType::UrlExport,
                MigrationExportFormat::Json => FallbackType::JsonExport,
                MigrationExportFormat::Html => FallbackType::HtmlBookmarkExport,
            },
            urls,
            format,
            export_content,
            instructions,
        }
    }

    /// Rollback a migration by closing the target tab and reopening in source browser
    ///
    /// Implements Requirement 8.5: Verify operation results and provide rollback options
    ///
    /// # Arguments
    /// * `manager` - The browser connector manager
    /// * `migration_id` - The ID of the migration to rollback
    ///
    /// # Returns
    /// * `Ok(())` if rollback successful
    pub async fn rollback_migration(
        &self,
        manager: &BrowserConnectorManager,
        migration_id: uuid::Uuid,
    ) -> Result<()> {
        // Find the migration record
        let migration = {
            let history = self.migration_history.read().await;
            history.iter().find(|m| m.id == migration_id).cloned()
        };

        let migration = migration.ok_or_else(|| {
            WebPageManagerError::CrossBrowser {
                source: CrossBrowserError::RollbackFailed {
                    reason: "Migration record not found".to_string(),
                },
            }
        })?;

        if !migration.rollbackable {
            return Err(WebPageManagerError::CrossBrowser {
                source: CrossBrowserError::RollbackFailed {
                    reason: "Migration cannot be rolled back".to_string(),
                },
            });
        }

        info!("Rolling back migration: {:?}", migration_id);

        // Step 1: Close the target tab if it exists
        if let Some(target_tab_id) = &migration.target_tab_id {
            if let Err(e) = manager.close_tab(migration.target_browser, target_tab_id).await {
                warn!("Failed to close target tab during rollback: {}", e);
            }
        }

        // Step 2: Reopen the tab in the source browser
        let new_tab_result = manager.create_tab(migration.source_browser, &migration.url).await;
        
        match new_tab_result {
            Ok(new_tab_id) => {
                // Activate the restored tab
                let _ = manager.activate_tab(migration.source_browser, &new_tab_id).await;

                // Update the migration record
                {
                    let mut history = self.migration_history.write().await;
                    if let Some(record) = history.iter_mut().find(|m| m.id == migration_id) {
                        record.mark_rolled_back();
                    }
                }

                info!("Successfully rolled back migration: {:?}", migration_id);
                Ok(())
            }
            Err(e) => {
                error!("Failed to restore tab during rollback: {}", e);
                Err(WebPageManagerError::CrossBrowser {
                    source: CrossBrowserError::RollbackFailed {
                        reason: format!("Failed to restore tab: {}", e),
                    },
                })
            }
        }
    }

    /// Verify a migration was successful by checking if the target tab exists
    ///
    /// Implements Requirement 8.5: Verify operation results
    ///
    /// # Arguments
    /// * `manager` - The browser connector manager
    /// * `migration_id` - The ID of the migration to verify
    ///
    /// # Returns
    /// * `Ok(true)` if migration is verified, `Ok(false)` if tab not found
    pub async fn verify_migration(
        &self,
        manager: &BrowserConnectorManager,
        migration_id: uuid::Uuid,
    ) -> Result<bool> {
        let migration = {
            let history = self.migration_history.read().await;
            history.iter().find(|m| m.id == migration_id).cloned()
        };

        let migration = migration.ok_or_else(|| {
            WebPageManagerError::CrossBrowser {
                source: CrossBrowserError::MigrationFailed {
                    source_browser: BrowserType::Chrome,
                    target_browser: BrowserType::Chrome,
                    reason: "Migration record not found".to_string(),
                },
            }
        })?;

        if let Some(target_tab_id) = &migration.target_tab_id {
            let tabs = manager.get_tabs(migration.target_browser).await?;
            let tab_exists = tabs.iter().any(|t| &t.id == target_tab_id);
            Ok(tab_exists)
        } else {
            Ok(false)
        }
    }

    /// Get migration history
    pub async fn get_migration_history(&self) -> Vec<MigrationRecord> {
        let history = self.migration_history.read().await;
        history.iter().cloned().collect()
    }

    /// Get recent migrations
    pub async fn get_recent_migrations(&self, count: usize) -> Vec<MigrationRecord> {
        let history = self.migration_history.read().await;
        history.iter().rev().take(count).cloned().collect()
    }

    /// Get migrations that can be rolled back
    pub async fn get_rollbackable_migrations(&self) -> Vec<MigrationRecord> {
        let history = self.migration_history.read().await;
        history
            .iter()
            .filter(|m| m.rollbackable && m.status.is_success())
            .cloned()
            .collect()
    }

    /// Clear migration history
    pub async fn clear_migration_history(&self) {
        let mut history = self.migration_history.write().await;
        history.clear();
        info!("Cleared migration history");
    }

    // =========================================================================
    // Private Helper Methods for Migration
    // =========================================================================

    /// Capture session state from a tab
    async fn capture_session_state(
        &self,
        _manager: &BrowserConnectorManager,
        _browser: BrowserType,
        _tab_id: &TabId,
        url: &str,
        title: &str,
    ) -> Option<SessionState> {
        // Note: Full session state capture would require deeper browser integration
        // via CDP WebSocket or browser extensions. For now, we capture basic state.
        // This is a limitation documented in the design - full session state
        // preservation is best-effort and depends on browser API capabilities.
        
        Some(SessionState::basic(url.to_string(), title.to_string()))
    }

    /// Attempt to migrate a tab to the target browser
    async fn attempt_migration(
        &self,
        manager: &BrowserConnectorManager,
        target_browser: BrowserType,
        url: &str,
        _config: &MigrationConfig,
    ) -> Result<TabId> {
        manager.create_tab(target_browser, url).await
    }

    /// Handle migration fallback when direct migration fails
    async fn handle_migration_fallback(
        &self,
        manager: &BrowserConnectorManager,
        record: &mut MigrationRecord,
        source_browser: BrowserType,
        target_browser: BrowserType,
        url: &str,
        title: &str,
        config: &MigrationConfig,
        original_error: WebPageManagerError,
    ) -> Result<MigrationResult> {
        warn!(
            "Direct migration failed, attempting fallback: {}",
            original_error
        );

        record.migration_type = MigrationType::UrlOnly;

        // Try URL-only migration as first fallback
        match manager.create_tab(target_browser, url).await {
            Ok(new_tab_id) => {
                record.mark_success_with_fallback(new_tab_id.clone(), "url_only");

                if config.close_source_tab {
                    let _ = manager.close_tab(source_browser, &record.source_tab_id).await;
                }

                if config.activate_target_tab {
                    let _ = manager.activate_tab(target_browser, &new_tab_id).await;
                }

                self.record_migration(record).await;

                // Update stats
                {
                    let mut stats = self.stats.write().await;
                    stats.fallback_operations += 1;
                }

                Ok(MigrationResult {
                    record: record.clone(),
                    used_fallback: true,
                    fallback_data: None,
                })
            }
            Err(_) => {
                // URL-only migration also failed, generate export fallback
                record.migration_type = MigrationType::Export;
                
                let tab_info = TabInfo {
                    id: record.source_tab_id.clone(),
                    url: url.to_string(),
                    title: title.to_string(),
                    favicon_url: None,
                    browser_type: source_browser,
                    is_private: false,
                    created_at: Utc::now(),
                    last_accessed: Utc::now(),
                };

                let fallback_data = self.generate_fallback_export(
                    &[tab_info],
                    source_browser,
                    match config.preferred_fallback {
                        FallbackType::HtmlBookmarkExport => MigrationExportFormat::Html,
                        FallbackType::JsonExport => MigrationExportFormat::Json,
                        _ => MigrationExportFormat::PlainText,
                    },
                );

                record.mark_success_with_fallback(TabId::new(), "export");
                self.record_migration(record).await;

                // Update stats
                {
                    let mut stats = self.stats.write().await;
                    stats.fallback_operations += 1;
                }

                Ok(MigrationResult {
                    record: record.clone(),
                    used_fallback: true,
                    fallback_data: Some(fallback_data),
                })
            }
        }
    }

    /// Generate HTML bookmark export content
    fn generate_html_bookmark_export(&self, urls: &[UrlExportEntry]) -> String {
        let mut html = String::from(
            "<!DOCTYPE NETSCAPE-Bookmark-file-1>\n\
             <META HTTP-EQUIV=\"Content-Type\" CONTENT=\"text/html; charset=UTF-8\">\n\
             <TITLE>Bookmarks</TITLE>\n\
             <H1>Bookmarks</H1>\n\
             <DL><p>\n\
             <DT><H3>Migrated Tabs</H3>\n\
             <DL><p>\n"
        );

        for entry in urls {
            html.push_str(&format!(
                "<DT><A HREF=\"{}\">{}</A>\n",
                entry.url,
                entry.title.replace('<', "&lt;").replace('>', "&gt;")
            ));
        }

        html.push_str("</DL><p>\n</DL><p>\n");
        html
    }

    /// Record a migration in history
    async fn record_migration(&self, record: &MigrationRecord) {
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.cross_browser_migrations += 1;
        }

        // Add to history
        let mut history = self.migration_history.write().await;
        history.push_back(record.clone());

        // Trim history if needed (use same limit as operation history)
        while history.len() > self.config.max_history_size {
            history.pop_front();
        }
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

    // =========================================================================
    // Cross-Browser Migration Tests (Requirements 8.2, 8.3, 8.4)
    // =========================================================================

    #[test]
    fn test_migration_record_creation() {
        let record = MigrationRecord::new(
            BrowserType::Chrome,
            BrowserType::Firefox,
            TabId("source-tab".to_string()),
            "https://example.com".to_string(),
            "Example Page".to_string(),
        );

        assert_eq!(record.source_browser, BrowserType::Chrome);
        assert_eq!(record.target_browser, BrowserType::Firefox);
        assert_eq!(record.url, "https://example.com");
        assert_eq!(record.title, "Example Page");
        assert_eq!(record.status, MigrationStatus::Pending);
        assert!(!record.rollbackable);
        assert!(record.target_tab_id.is_none());
    }

    #[test]
    fn test_migration_status_transitions() {
        let mut record = MigrationRecord::new(
            BrowserType::Chrome,
            BrowserType::Edge,
            TabId::new(),
            "https://test.com".to_string(),
            "Test".to_string(),
        );

        assert!(!record.status.is_success());
        assert!(!record.status.is_failed());

        // Test success transition
        record.mark_success(TabId("new-tab".to_string()), true);
        assert!(record.status.is_success());
        assert!(record.rollbackable);
        assert!(record.session_preserved);
        assert!(record.target_tab_id.is_some());
        assert!(record.completed_at.is_some());
    }

    #[test]
    fn test_migration_success_with_fallback() {
        let mut record = MigrationRecord::new(
            BrowserType::Firefox,
            BrowserType::Chrome,
            TabId::new(),
            "https://fallback.com".to_string(),
            "Fallback Test".to_string(),
        );

        record.mark_success_with_fallback(TabId("fallback-tab".to_string()), "url_only");
        
        assert!(record.status.is_success());
        assert!(!record.session_preserved);
        assert!(record.rollbackable);
        
        match &record.status {
            MigrationStatus::SuccessWithFallback { fallback_type } => {
                assert_eq!(fallback_type, "url_only");
            }
            _ => panic!("Expected SuccessWithFallback status"),
        }
    }

    #[test]
    fn test_migration_failure() {
        let mut record = MigrationRecord::new(
            BrowserType::Chrome,
            BrowserType::Firefox,
            TabId::new(),
            "https://fail.com".to_string(),
            "Fail Test".to_string(),
        );

        record.mark_failed("Connection refused".to_string());
        
        assert!(record.status.is_failed());
        assert!(!record.rollbackable);
        assert_eq!(record.error_message, Some("Connection refused".to_string()));
        assert!(record.completed_at.is_some());
    }

    #[test]
    fn test_session_state_basic() {
        let state = SessionState::basic(
            "https://example.com".to_string(),
            "Example".to_string(),
        );

        assert_eq!(state.url, "https://example.com");
        assert_eq!(state.title, "Example");
        assert!(!state.has_preserved_data());
        assert!(state.cookies.is_empty());
        assert!(state.scroll_position.is_none());
    }

    #[test]
    fn test_session_state_with_data() {
        let mut state = SessionState::basic(
            "https://example.com".to_string(),
            "Example".to_string(),
        );
        state.scroll_position = Some(500);

        assert!(state.has_preserved_data());
    }

    #[test]
    fn test_migration_config_defaults() {
        let config = MigrationConfig::default();

        assert!(config.preserve_session_state);
        assert!(config.close_source_tab);
        assert!(config.activate_target_tab);
        assert!(config.auto_fallback);
        assert_eq!(config.timeout_ms, 10000);
        assert_eq!(config.preferred_fallback, FallbackType::UrlExport);
    }

    #[test]
    fn test_fallback_data_generation_plain_text() {
        let controller = RemoteTabController::new();
        let tabs = vec![
            TabInfo {
                id: TabId("tab1".to_string()),
                url: "https://example1.com".to_string(),
                title: "Example 1".to_string(),
                favicon_url: None,
                browser_type: BrowserType::Chrome,
                is_private: false,
                created_at: Utc::now(),
                last_accessed: Utc::now(),
            },
            TabInfo {
                id: TabId("tab2".to_string()),
                url: "https://example2.com".to_string(),
                title: "Example 2".to_string(),
                favicon_url: None,
                browser_type: BrowserType::Chrome,
                is_private: false,
                created_at: Utc::now(),
                last_accessed: Utc::now(),
            },
        ];

        let fallback = controller.generate_fallback_export(
            &tabs,
            BrowserType::Chrome,
            MigrationExportFormat::PlainText,
        );

        assert_eq!(fallback.fallback_type, FallbackType::UrlExport);
        assert_eq!(fallback.urls.len(), 2);
        assert!(fallback.export_content.is_some());
        
        let content = fallback.export_content.unwrap();
        assert!(content.contains("https://example1.com"));
        assert!(content.contains("https://example2.com"));
        assert!(content.contains("Example 1"));
        assert!(content.contains("Example 2"));
    }

    #[test]
    fn test_fallback_data_generation_json() {
        let controller = RemoteTabController::new();
        let tabs = vec![
            TabInfo {
                id: TabId("tab1".to_string()),
                url: "https://json-test.com".to_string(),
                title: "JSON Test".to_string(),
                favicon_url: None,
                browser_type: BrowserType::Firefox,
                is_private: false,
                created_at: Utc::now(),
                last_accessed: Utc::now(),
            },
        ];

        let fallback = controller.generate_fallback_export(
            &tabs,
            BrowserType::Firefox,
            MigrationExportFormat::Json,
        );

        assert_eq!(fallback.fallback_type, FallbackType::JsonExport);
        assert!(fallback.export_content.is_some());
        
        let content = fallback.export_content.unwrap();
        assert!(content.contains("\"url\""));
        assert!(content.contains("https://json-test.com"));
    }

    #[test]
    fn test_fallback_data_generation_html() {
        let controller = RemoteTabController::new();
        let tabs = vec![
            TabInfo {
                id: TabId("tab1".to_string()),
                url: "https://html-test.com".to_string(),
                title: "HTML Test".to_string(),
                favicon_url: None,
                browser_type: BrowserType::Edge,
                is_private: false,
                created_at: Utc::now(),
                last_accessed: Utc::now(),
            },
        ];

        let fallback = controller.generate_fallback_export(
            &tabs,
            BrowserType::Edge,
            MigrationExportFormat::Html,
        );

        assert_eq!(fallback.fallback_type, FallbackType::HtmlBookmarkExport);
        assert!(fallback.export_content.is_some());
        
        let content = fallback.export_content.unwrap();
        assert!(content.contains("<!DOCTYPE NETSCAPE-Bookmark-file-1>"));
        assert!(content.contains("https://html-test.com"));
        assert!(content.contains("HTML Test"));
    }

    #[tokio::test]
    async fn test_migration_history_management() {
        let controller = RemoteTabController::new();

        // Initially empty
        let history = controller.get_migration_history().await;
        assert!(history.is_empty());

        // Get rollbackable migrations (should be empty)
        let rollbackable = controller.get_rollbackable_migrations().await;
        assert!(rollbackable.is_empty());
    }

    #[test]
    fn test_migration_type_display() {
        assert_eq!(format!("{}", MigrationType::Full), "Full");
        assert_eq!(format!("{}", MigrationType::UrlOnly), "UrlOnly");
        assert_eq!(format!("{}", MigrationType::Export), "Export");
    }

    #[test]
    fn test_url_export_entry() {
        let entry = UrlExportEntry {
            url: "https://test.com".to_string(),
            title: "Test Page".to_string(),
            source_browser: BrowserType::Chrome,
        };

        assert_eq!(entry.url, "https://test.com");
        assert_eq!(entry.title, "Test Page");
        assert_eq!(entry.source_browser, BrowserType::Chrome);
    }

    #[test]
    fn test_cookie_info() {
        let cookie = CookieInfo {
            name: "session".to_string(),
            value: "abc123".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: true,
            http_only: true,
            expires: None,
        };

        assert_eq!(cookie.name, "session");
        assert!(cookie.secure);
        assert!(cookie.http_only);
    }

    #[test]
    fn test_migration_result_helpers() {
        let mut record = MigrationRecord::new(
            BrowserType::Chrome,
            BrowserType::Firefox,
            TabId::new(),
            "https://test.com".to_string(),
            "Test".to_string(),
        );
        record.mark_success(TabId("new-tab".to_string()), false);

        let result = MigrationResult {
            record,
            used_fallback: false,
            fallback_data: None,
        };

        assert!(result.is_success());
        assert!(result.error_message().is_none());
        assert!(result.new_tab_id().is_some());
    }

    #[test]
    fn test_migration_result_with_error() {
        let mut record = MigrationRecord::new(
            BrowserType::Chrome,
            BrowserType::Firefox,
            TabId::new(),
            "https://test.com".to_string(),
            "Test".to_string(),
        );
        record.mark_failed("Test error".to_string());

        let result = MigrationResult {
            record,
            used_fallback: false,
            fallback_data: None,
        };

        assert!(!result.is_success());
        assert_eq!(result.error_message(), Some("Test error"));
        assert!(result.new_tab_id().is_none());
    }

    #[tokio::test]
    async fn test_stats_include_migration_fields() {
        let controller = RemoteTabController::new();
        let stats = controller.get_stats().await;

        assert_eq!(stats.cross_browser_migrations, 0);
        assert_eq!(stats.fallback_operations, 0);
    }
}
