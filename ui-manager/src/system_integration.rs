//! Cross-platform system integration module
//! 
//! This module provides unified interfaces for:
//! - Global hotkey registration and handling
//! - System notifications
//! - System tray and quick access functionality
//! 
//! The implementation abstracts platform-specific details while providing
//! a consistent API across Windows, Linux, and macOS.

use web_page_manager_core::*;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

// Re-export chrono types from core
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Cross-platform hotkey manager
/// 
/// Provides a unified interface for registering and handling global hotkeys
/// across Windows, Linux, and macOS platforms.
pub struct CrossPlatformHotkeyManager {
    registered_hotkeys: Arc<RwLock<HashMap<String, HotkeyRegistration>>>,
    callbacks: Arc<RwLock<HashMap<String, Arc<dyn HotkeyCallback>>>>,
    initialized: std::sync::atomic::AtomicBool,
}

/// Hotkey registration information
#[derive(Debug, Clone)]
pub struct HotkeyRegistration {
    /// Unique identifier for the hotkey
    pub id: String,
    /// Key combination string (e.g., "Ctrl+Shift+F")
    pub key_combination: String,
    /// Action identifier
    pub action: String,
    /// Human-readable description
    pub description: String,
    /// Whether the hotkey is currently active
    pub is_active: bool,
    /// Platform-specific registration ID (if applicable)
    pub platform_id: Option<u32>,
}

impl From<&Hotkey> for HotkeyRegistration {
    fn from(hotkey: &Hotkey) -> Self {
        Self {
            id: hotkey.id.clone(),
            key_combination: hotkey.key_combination.clone(),
            action: hotkey.action.clone(),
            description: hotkey.description.clone(),
            is_active: false,
            platform_id: None,
        }
    }
}

/// Callback trait for hotkey events
pub trait HotkeyCallback: Send + Sync {
    /// Called when the hotkey is pressed
    fn on_hotkey_pressed(&self, hotkey_id: &str) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

/// Simple function-based hotkey callback
pub struct FnHotkeyCallback<F>
where
    F: Fn(&str) + Send + Sync,
{
    callback: F,
}

impl<F> FnHotkeyCallback<F>
where
    F: Fn(&str) + Send + Sync,
{
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

impl<F> HotkeyCallback for FnHotkeyCallback<F>
where
    F: Fn(&str) + Send + Sync,
{
    fn on_hotkey_pressed(&self, hotkey_id: &str) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        (self.callback)(hotkey_id);
        Box::pin(async {})
    }
}

impl CrossPlatformHotkeyManager {
    /// Create a new cross-platform hotkey manager
    pub fn new() -> Self {
        Self {
            registered_hotkeys: Arc::new(RwLock::new(HashMap::new())),
            callbacks: Arc::new(RwLock::new(HashMap::new())),
            initialized: std::sync::atomic::AtomicBool::new(false),
        }
    }
    
    /// Initialize the hotkey manager
    /// 
    /// This must be called before registering any hotkeys.
    pub async fn initialize(&self) -> Result<()> {
        tracing::info!("Initializing cross-platform hotkey manager");
        
        #[cfg(target_os = "windows")]
        {
            tracing::debug!("Initializing Windows hotkey support");
            // Windows uses RegisterHotKey API
        }
        
        #[cfg(target_os = "linux")]
        {
            tracing::debug!("Initializing Linux hotkey support (X11/Wayland)");
            // Linux uses X11 XGrabKey or libxkbcommon for Wayland
        }
        
        #[cfg(target_os = "macos")]
        {
            tracing::debug!("Initializing macOS hotkey support");
            // macOS uses Carbon Event Manager or CGEventTap
        }
        
        self.initialized.store(true, std::sync::atomic::Ordering::Relaxed);
        tracing::info!("Cross-platform hotkey manager initialized");
        Ok(())
    }
    
    /// Register a global hotkey
    /// 
    /// # Arguments
    /// 
    /// * `hotkey` - The hotkey definition
    /// * `callback` - Callback to invoke when the hotkey is pressed
    /// 
    /// # Returns
    /// 
    /// Returns Ok(()) if registration was successful, or an error if the
    /// hotkey could not be registered (e.g., already in use by another app).
    pub async fn register_hotkey(
        &self,
        hotkey: &Hotkey,
        callback: Arc<dyn HotkeyCallback>,
    ) -> Result<()> {
        if !self.initialized.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(WebPageManagerError::UI {
                source: UIError::NotInitialized,
            });
        }
        
        tracing::info!("Registering hotkey: {} -> {}", hotkey.key_combination, hotkey.action);
        
        let mut registration = HotkeyRegistration::from(hotkey);
        
        // Parse the key combination
        let parsed = Self::parse_key_combination(&hotkey.key_combination)?;
        tracing::debug!("Parsed key combination: {:?}", parsed);
        
        // Platform-specific registration
        #[cfg(target_os = "windows")]
        {
            registration.platform_id = Some(self.register_windows_hotkey(&parsed)?);
        }
        
        #[cfg(target_os = "linux")]
        {
            registration.platform_id = Some(self.register_linux_hotkey(&parsed)?);
        }
        
        #[cfg(target_os = "macos")]
        {
            registration.platform_id = Some(self.register_macos_hotkey(&parsed)?);
        }
        
        registration.is_active = true;
        
        // Store registration and callback
        {
            let mut hotkeys = self.registered_hotkeys.write().await;
            hotkeys.insert(hotkey.id.clone(), registration);
        }
        
        {
            let mut callbacks = self.callbacks.write().await;
            callbacks.insert(hotkey.id.clone(), callback);
        }
        
        tracing::info!("Hotkey registered successfully: {}", hotkey.id);
        Ok(())
    }
    
    /// Unregister a hotkey
    pub async fn unregister_hotkey(&self, hotkey_id: &str) -> Result<()> {
        tracing::info!("Unregistering hotkey: {}", hotkey_id);
        
        let registration = {
            let mut hotkeys = self.registered_hotkeys.write().await;
            hotkeys.remove(hotkey_id)
        };
        
        if let Some(reg) = registration {
            if let Some(platform_id) = reg.platform_id {
                #[cfg(target_os = "windows")]
                self.unregister_windows_hotkey(platform_id)?;
                
                #[cfg(target_os = "linux")]
                self.unregister_linux_hotkey(platform_id)?;
                
                #[cfg(target_os = "macos")]
                self.unregister_macos_hotkey(platform_id)?;
            }
        }
        
        {
            let mut callbacks = self.callbacks.write().await;
            callbacks.remove(hotkey_id);
        }
        
        Ok(())
    }
    
    /// Unregister all hotkeys
    pub async fn unregister_all(&self) -> Result<()> {
        tracing::info!("Unregistering all hotkeys");
        
        let hotkey_ids: Vec<String> = {
            let hotkeys = self.registered_hotkeys.read().await;
            hotkeys.keys().cloned().collect()
        };
        
        for id in hotkey_ids {
            self.unregister_hotkey(&id).await?;
        }
        
        Ok(())
    }
    
    /// Get list of registered hotkeys
    pub async fn get_registered_hotkeys(&self) -> Vec<HotkeyRegistration> {
        let hotkeys = self.registered_hotkeys.read().await;
        hotkeys.values().cloned().collect()
    }
    
    /// Check if a hotkey is registered
    pub async fn is_hotkey_registered(&self, hotkey_id: &str) -> bool {
        let hotkeys = self.registered_hotkeys.read().await;
        hotkeys.contains_key(hotkey_id)
    }
    
    /// Parse a key combination string into components
    fn parse_key_combination(combination: &str) -> Result<ParsedKeyCombination> {
        let parts: Vec<&str> = combination.split('+').map(|s| s.trim()).collect();
        
        if parts.is_empty() {
            return Err(WebPageManagerError::UI {
                source: UIError::OperationFailed {
                    operation: format!("Invalid key combination: {}", combination),
                },
            });
        }
        
        let mut modifiers = Vec::new();
        let mut key = None;
        
        for part in parts {
            match part.to_lowercase().as_str() {
                "ctrl" | "control" => modifiers.push(KeyModifier::Ctrl),
                "alt" => modifiers.push(KeyModifier::Alt),
                "shift" => modifiers.push(KeyModifier::Shift),
                "meta" | "win" | "cmd" | "super" => modifiers.push(KeyModifier::Meta),
                _ => {
                    if key.is_some() {
                        return Err(WebPageManagerError::UI {
                            source: UIError::OperationFailed {
                                operation: format!("Multiple keys in combination: {}", combination),
                            },
                        });
                    }
                    key = Some(part.to_string());
                }
            }
        }
        
        let key = key.ok_or_else(|| WebPageManagerError::UI {
            source: UIError::OperationFailed {
                operation: format!("No key specified in combination: {}", combination),
            },
        })?;
        
        Ok(ParsedKeyCombination { modifiers, key })
    }
    
    // Platform-specific registration methods
    
    #[cfg(target_os = "windows")]
    fn register_windows_hotkey(&self, _parsed: &ParsedKeyCombination) -> Result<u32> {
        // TODO: Implement Windows hotkey registration using RegisterHotKey
        // This would use the windows crate to call Win32 API
        tracing::debug!("Windows hotkey registration (stub)");
        Ok(0)
    }
    
    #[cfg(target_os = "windows")]
    fn unregister_windows_hotkey(&self, _platform_id: u32) -> Result<()> {
        // TODO: Implement Windows hotkey unregistration using UnregisterHotKey
        tracing::debug!("Windows hotkey unregistration (stub)");
        Ok(())
    }
    
    #[cfg(target_os = "linux")]
    fn register_linux_hotkey(&self, _parsed: &ParsedKeyCombination) -> Result<u32> {
        // TODO: Implement Linux hotkey registration
        // For X11: Use XGrabKey
        // For Wayland: Use libxkbcommon or portal API
        tracing::debug!("Linux hotkey registration (stub)");
        Ok(0)
    }
    
    #[cfg(target_os = "linux")]
    fn unregister_linux_hotkey(&self, _platform_id: u32) -> Result<()> {
        // TODO: Implement Linux hotkey unregistration
        tracing::debug!("Linux hotkey unregistration (stub)");
        Ok(())
    }
    
    #[cfg(target_os = "macos")]
    fn register_macos_hotkey(&self, _parsed: &ParsedKeyCombination) -> Result<u32> {
        // TODO: Implement macOS hotkey registration
        // Use Carbon Event Manager or CGEventTap
        tracing::debug!("macOS hotkey registration (stub)");
        Ok(0)
    }
    
    #[cfg(target_os = "macos")]
    fn unregister_macos_hotkey(&self, _platform_id: u32) -> Result<()> {
        // TODO: Implement macOS hotkey unregistration
        tracing::debug!("macOS hotkey unregistration (stub)");
        Ok(())
    }
    
    /// Shutdown the hotkey manager
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down cross-platform hotkey manager");
        self.unregister_all().await?;
        self.initialized.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}

impl Default for CrossPlatformHotkeyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Parsed key combination
#[derive(Debug, Clone)]
pub struct ParsedKeyCombination {
    pub modifiers: Vec<KeyModifier>,
    pub key: String,
}

/// Key modifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyModifier {
    Ctrl,
    Alt,
    Shift,
    Meta, // Windows key on Windows, Command on macOS, Super on Linux
}


// ============================================================================
// Cross-Platform Notification Manager
// ============================================================================

use crate::traits::{NotificationConfig, NotificationUrgency, NotificationAction};

/// Cross-platform notification manager
/// 
/// Provides a unified interface for showing system notifications
/// across Windows, Linux, and macOS platforms.
pub struct CrossPlatformNotificationManager {
    app_name: String,
    app_icon: Option<String>,
    initialized: std::sync::atomic::AtomicBool,
    notification_history: Arc<RwLock<Vec<NotificationRecord>>>,
}

/// Record of a sent notification
#[derive(Debug, Clone)]
pub struct NotificationRecord {
    pub id: String,
    pub config: NotificationConfig,
    pub sent_at: DateTime<Utc>,
    pub status: NotificationStatus,
}

/// Status of a notification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationStatus {
    Pending,
    Shown,
    Clicked,
    Dismissed,
    TimedOut,
    Failed,
}

/// Callback trait for notification events
pub trait NotificationCallback: Send + Sync {
    /// Called when a notification is clicked
    fn on_notification_clicked(&self, notification_id: &str) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
    
    /// Called when a notification action button is clicked
    fn on_action_clicked(&self, notification_id: &str, action_id: &str) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
    
    /// Called when a notification is dismissed
    fn on_notification_dismissed(&self, notification_id: &str) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

impl CrossPlatformNotificationManager {
    /// Create a new notification manager
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            app_icon: None,
            initialized: std::sync::atomic::AtomicBool::new(false),
            notification_history: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Create a new notification manager with an icon
    pub fn with_icon(app_name: impl Into<String>, icon_path: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            app_icon: Some(icon_path.into()),
            initialized: std::sync::atomic::AtomicBool::new(false),
            notification_history: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Initialize the notification manager
    pub async fn initialize(&self) -> Result<()> {
        tracing::info!("Initializing cross-platform notification manager for '{}'", self.app_name);
        
        #[cfg(target_os = "windows")]
        {
            tracing::debug!("Initializing Windows notification support (Toast notifications)");
            // Windows uses Windows.UI.Notifications or WinRT toast notifications
        }
        
        #[cfg(target_os = "linux")]
        {
            tracing::debug!("Initializing Linux notification support (libnotify/D-Bus)");
            // Linux uses libnotify or D-Bus org.freedesktop.Notifications
        }
        
        #[cfg(target_os = "macos")]
        {
            tracing::debug!("Initializing macOS notification support (NSUserNotification/UNUserNotificationCenter)");
            // macOS uses NSUserNotification (deprecated) or UNUserNotificationCenter
        }
        
        self.initialized.store(true, std::sync::atomic::Ordering::Relaxed);
        tracing::info!("Cross-platform notification manager initialized");
        Ok(())
    }
    
    /// Show a notification
    /// 
    /// # Arguments
    /// 
    /// * `config` - The notification configuration
    /// 
    /// # Returns
    /// 
    /// Returns the notification ID if successful.
    pub async fn show_notification(&self, config: &NotificationConfig) -> Result<String> {
        if !self.initialized.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(WebPageManagerError::UI {
                source: UIError::NotInitialized,
            });
        }
        
        let notification_id = Uuid::new_v4().to_string();
        
        tracing::info!("Showing notification: {} - {}", config.title, config.message);
        
        // Platform-specific notification display
        #[cfg(target_os = "windows")]
        self.show_windows_notification(&notification_id, config)?;
        
        #[cfg(target_os = "linux")]
        self.show_linux_notification(&notification_id, config)?;
        
        #[cfg(target_os = "macos")]
        self.show_macos_notification(&notification_id, config)?;
        
        // Record the notification
        let record = NotificationRecord {
            id: notification_id.clone(),
            config: config.clone(),
            sent_at: Utc::now(),
            status: NotificationStatus::Shown,
        };
        
        {
            let mut history = self.notification_history.write().await;
            history.push(record);
            
            // Keep only the last 100 notifications
            if history.len() > 100 {
                history.remove(0);
            }
        }
        
        Ok(notification_id)
    }
    
    /// Show a simple notification with just a message
    pub async fn show_simple(&self, message: impl Into<String>) -> Result<String> {
        let config = NotificationConfig::simple(message);
        self.show_notification(&config).await
    }
    
    /// Show a notification with title and message
    pub async fn show_with_title(
        &self,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Result<String> {
        let config = NotificationConfig::with_title(title, message);
        self.show_notification(&config).await
    }
    
    /// Show a notification for tab activity
    pub async fn show_tab_activity(&self, browser_name: &str, tab_count: usize) -> Result<String> {
        let config = NotificationConfig {
            title: "标签页活动".to_string(),
            message: format!("{} 中有 {} 个新标签页", browser_name, tab_count),
            icon: self.app_icon.clone(),
            urgency: NotificationUrgency::Normal,
            actions: Vec::new(),
            timeout_ms: Some(5000),
        };
        self.show_notification(&config).await
    }
    
    /// Show a notification for bookmark sync completion
    pub async fn show_bookmark_sync(&self, synced_count: usize) -> Result<String> {
        let config = NotificationConfig {
            title: "书签同步完成".to_string(),
            message: format!("已同步 {} 个书签", synced_count),
            icon: self.app_icon.clone(),
            urgency: NotificationUrgency::Low,
            actions: Vec::new(),
            timeout_ms: Some(3000),
        };
        self.show_notification(&config).await
    }
    
    /// Show a notification for content analysis completion
    pub async fn show_analysis_complete(&self, analyzed_count: usize) -> Result<String> {
        let config = NotificationConfig {
            title: "内容分析完成".to_string(),
            message: format!("已分析 {} 个页面", analyzed_count),
            icon: self.app_icon.clone(),
            urgency: NotificationUrgency::Low,
            actions: vec![
                NotificationAction {
                    id: "view_results".to_string(),
                    label: "查看结果".to_string(),
                },
            ],
            timeout_ms: Some(5000),
        };
        self.show_notification(&config).await
    }
    
    /// Show a notification for duplicate bookmarks found
    pub async fn show_duplicates_found(&self, duplicate_count: usize) -> Result<String> {
        let config = NotificationConfig {
            title: "发现重复书签".to_string(),
            message: format!("发现 {} 组重复书签，点击查看详情", duplicate_count),
            icon: self.app_icon.clone(),
            urgency: NotificationUrgency::Normal,
            actions: vec![
                NotificationAction {
                    id: "view_duplicates".to_string(),
                    label: "查看详情".to_string(),
                },
                NotificationAction {
                    id: "dismiss".to_string(),
                    label: "稍后处理".to_string(),
                },
            ],
            timeout_ms: None, // Persistent until user action
        };
        self.show_notification(&config).await
    }
    
    /// Get notification history
    pub async fn get_history(&self) -> Vec<NotificationRecord> {
        let history = self.notification_history.read().await;
        history.clone()
    }
    
    /// Clear notification history
    pub async fn clear_history(&self) {
        let mut history = self.notification_history.write().await;
        history.clear();
    }
    
    // Platform-specific notification methods
    
    #[cfg(target_os = "windows")]
    fn show_windows_notification(&self, _id: &str, config: &NotificationConfig) -> Result<()> {
        // TODO: Implement Windows toast notification
        // Use windows crate to call Windows.UI.Notifications API
        tracing::debug!("Windows notification (stub): {} - {}", config.title, config.message);
        Ok(())
    }
    
    #[cfg(target_os = "linux")]
    fn show_linux_notification(&self, _id: &str, config: &NotificationConfig) -> Result<()> {
        // TODO: Implement Linux notification using libnotify or D-Bus
        // org.freedesktop.Notifications interface
        tracing::debug!("Linux notification (stub): {} - {}", config.title, config.message);
        Ok(())
    }
    
    #[cfg(target_os = "macos")]
    fn show_macos_notification(&self, _id: &str, config: &NotificationConfig) -> Result<()> {
        // TODO: Implement macOS notification using UNUserNotificationCenter
        tracing::debug!("macOS notification (stub): {} - {}", config.title, config.message);
        Ok(())
    }
    
    /// Shutdown the notification manager
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down cross-platform notification manager");
        self.initialized.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}

impl Default for CrossPlatformNotificationManager {
    fn default() -> Self {
        Self::new("Web Page Manager")
    }
}


// ============================================================================
// Cross-Platform System Tray Manager
// ============================================================================

/// Cross-platform system tray manager
/// 
/// Provides a unified interface for system tray functionality
/// across Windows, Linux, and macOS platforms.
pub struct CrossPlatformTrayManager {
    app_name: String,
    tooltip: Arc<RwLock<String>>,
    icon_path: Option<String>,
    menu_items: Arc<RwLock<Vec<TrayMenuItem>>>,
    initialized: std::sync::atomic::AtomicBool,
    event_handler: Arc<RwLock<Option<Arc<dyn TrayEventHandler>>>>,
}

/// Tray menu item definition
#[derive(Debug, Clone)]
pub enum TrayMenuItem {
    /// A clickable menu item
    Item {
        id: String,
        label: String,
        enabled: bool,
        checked: Option<bool>,
    },
    /// A separator line
    Separator,
    /// A submenu
    Submenu {
        id: String,
        label: String,
        items: Vec<TrayMenuItem>,
    },
}

impl TrayMenuItem {
    /// Create a new menu item
    pub fn item(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::Item {
            id: id.into(),
            label: label.into(),
            enabled: true,
            checked: None,
        }
    }
    
    /// Create a disabled menu item
    pub fn disabled_item(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::Item {
            id: id.into(),
            label: label.into(),
            enabled: false,
            checked: None,
        }
    }
    
    /// Create a checkable menu item
    pub fn checkable_item(id: impl Into<String>, label: impl Into<String>, checked: bool) -> Self {
        Self::Item {
            id: id.into(),
            label: label.into(),
            enabled: true,
            checked: Some(checked),
        }
    }
    
    /// Create a separator
    pub fn separator() -> Self {
        Self::Separator
    }
    
    /// Create a submenu
    pub fn submenu(id: impl Into<String>, label: impl Into<String>, items: Vec<TrayMenuItem>) -> Self {
        Self::Submenu {
            id: id.into(),
            label: label.into(),
            items,
        }
    }
}

/// Tray event types
#[derive(Debug, Clone)]
pub enum TrayEvent {
    /// Tray icon was clicked (left click)
    IconClicked,
    /// Tray icon was double-clicked
    IconDoubleClicked,
    /// Tray icon was right-clicked (context menu requested)
    IconRightClicked,
    /// A menu item was selected
    MenuItemSelected { item_id: String },
}

/// Callback trait for tray events
pub trait TrayEventHandler: Send + Sync {
    /// Handle a tray event
    fn handle_event(&self, event: TrayEvent) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

/// Simple function-based tray event handler
pub struct FnTrayEventHandler<F>
where
    F: Fn(TrayEvent) + Send + Sync,
{
    callback: F,
}

impl<F> FnTrayEventHandler<F>
where
    F: Fn(TrayEvent) + Send + Sync,
{
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

impl<F> TrayEventHandler for FnTrayEventHandler<F>
where
    F: Fn(TrayEvent) + Send + Sync,
{
    fn handle_event(&self, event: TrayEvent) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        (self.callback)(event);
        Box::pin(async {})
    }
}

impl CrossPlatformTrayManager {
    /// Create a new system tray manager
    pub fn new(app_name: impl Into<String>) -> Self {
        let app_name = app_name.into();
        let tooltip = format!("{} - 点击打开", app_name);
        Self {
            app_name,
            tooltip: Arc::new(RwLock::new(tooltip)),
            icon_path: None,
            menu_items: Arc::new(RwLock::new(Vec::new())),
            initialized: std::sync::atomic::AtomicBool::new(false),
            event_handler: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Create a new system tray manager with an icon
    pub fn with_icon(app_name: impl Into<String>, icon_path: impl Into<String>) -> Self {
        let app_name = app_name.into();
        let tooltip = format!("{} - 点击打开", app_name);
        Self {
            app_name,
            tooltip: Arc::new(RwLock::new(tooltip)),
            icon_path: Some(icon_path.into()),
            menu_items: Arc::new(RwLock::new(Vec::new())),
            initialized: std::sync::atomic::AtomicBool::new(false),
            event_handler: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Initialize the system tray
    pub async fn initialize(&self) -> Result<()> {
        tracing::info!("Initializing cross-platform system tray for '{}'", self.app_name);
        
        #[cfg(target_os = "windows")]
        {
            tracing::debug!("Initializing Windows system tray (Shell_NotifyIcon)");
            // Windows uses Shell_NotifyIcon API
        }
        
        #[cfg(target_os = "linux")]
        {
            tracing::debug!("Initializing Linux system tray (StatusNotifierItem/AppIndicator)");
            // Linux uses StatusNotifierItem (modern) or XEmbed (legacy)
        }
        
        #[cfg(target_os = "macos")]
        {
            tracing::debug!("Initializing macOS system tray (NSStatusItem)");
            // macOS uses NSStatusItem
        }
        
        // Set up default menu
        self.set_default_menu().await?;
        
        self.initialized.store(true, std::sync::atomic::Ordering::Relaxed);
        tracing::info!("Cross-platform system tray initialized");
        Ok(())
    }
    
    /// Set the default context menu
    async fn set_default_menu(&self) -> Result<()> {
        let default_menu = vec![
            TrayMenuItem::item("show_window", "打开主窗口"),
            TrayMenuItem::Separator,
            TrayMenuItem::item("quick_search", "快速搜索"),
            TrayMenuItem::item("recent_closed", "最近关闭的标签页"),
            TrayMenuItem::Separator,
            TrayMenuItem::item("settings", "设置"),
            TrayMenuItem::Separator,
            TrayMenuItem::item("exit", "退出"),
        ];
        
        self.set_menu(default_menu).await
    }
    
    /// Set the context menu items
    pub async fn set_menu(&self, items: Vec<TrayMenuItem>) -> Result<()> {
        tracing::debug!("Setting tray menu with {} items", items.len());
        
        {
            let mut menu = self.menu_items.write().await;
            *menu = items;
        }
        
        // Platform-specific menu update
        #[cfg(target_os = "windows")]
        self.update_windows_menu()?;
        
        #[cfg(target_os = "linux")]
        self.update_linux_menu()?;
        
        #[cfg(target_os = "macos")]
        self.update_macos_menu()?;
        
        Ok(())
    }
    
    /// Update the tooltip text
    pub async fn set_tooltip(&self, tooltip: impl Into<String>) -> Result<()> {
        let tooltip = tooltip.into();
        tracing::debug!("Setting tray tooltip: {}", tooltip);
        
        {
            let mut current = self.tooltip.write().await;
            *current = tooltip.clone();
        }
        
        // Platform-specific tooltip update
        #[cfg(target_os = "windows")]
        self.update_windows_tooltip(&tooltip)?;
        
        #[cfg(target_os = "linux")]
        self.update_linux_tooltip(&tooltip)?;
        
        #[cfg(target_os = "macos")]
        self.update_macos_tooltip(&tooltip)?;
        
        Ok(())
    }
    
    /// Update tooltip with activity count
    pub async fn set_activity_badge(&self, count: usize) -> Result<()> {
        let tooltip = if count > 0 {
            format!("{} - {} 个新活动", self.app_name, count)
        } else {
            format!("{} - 点击打开", self.app_name)
        };
        self.set_tooltip(tooltip).await
    }
    
    /// Set the event handler
    pub async fn set_event_handler(&self, handler: Arc<dyn TrayEventHandler>) -> Result<()> {
        let mut current = self.event_handler.write().await;
        *current = Some(handler);
        Ok(())
    }
    
    /// Show the tray icon
    pub async fn show(&self) -> Result<()> {
        if !self.initialized.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(WebPageManagerError::UI {
                source: UIError::NotInitialized,
            });
        }
        
        tracing::info!("Showing system tray icon");
        
        #[cfg(target_os = "windows")]
        self.show_windows_tray()?;
        
        #[cfg(target_os = "linux")]
        self.show_linux_tray()?;
        
        #[cfg(target_os = "macos")]
        self.show_macos_tray()?;
        
        Ok(())
    }
    
    /// Hide the tray icon
    pub async fn hide(&self) -> Result<()> {
        tracing::info!("Hiding system tray icon");
        
        #[cfg(target_os = "windows")]
        self.hide_windows_tray()?;
        
        #[cfg(target_os = "linux")]
        self.hide_linux_tray()?;
        
        #[cfg(target_os = "macos")]
        self.hide_macos_tray()?;
        
        Ok(())
    }
    
    /// Get the current tooltip
    pub async fn get_tooltip(&self) -> String {
        let tooltip = self.tooltip.read().await;
        tooltip.clone()
    }
    
    /// Get the current menu items
    pub async fn get_menu(&self) -> Vec<TrayMenuItem> {
        let menu = self.menu_items.read().await;
        menu.clone()
    }
    
    // Platform-specific methods
    
    #[cfg(target_os = "windows")]
    fn update_windows_menu(&self) -> Result<()> {
        tracing::debug!("Updating Windows tray menu (stub)");
        Ok(())
    }
    
    #[cfg(target_os = "windows")]
    fn update_windows_tooltip(&self, _tooltip: &str) -> Result<()> {
        tracing::debug!("Updating Windows tray tooltip (stub)");
        Ok(())
    }
    
    #[cfg(target_os = "windows")]
    fn show_windows_tray(&self) -> Result<()> {
        tracing::debug!("Showing Windows tray icon (stub)");
        Ok(())
    }
    
    #[cfg(target_os = "windows")]
    fn hide_windows_tray(&self) -> Result<()> {
        tracing::debug!("Hiding Windows tray icon (stub)");
        Ok(())
    }
    
    #[cfg(target_os = "linux")]
    fn update_linux_menu(&self) -> Result<()> {
        tracing::debug!("Updating Linux tray menu (stub)");
        Ok(())
    }
    
    #[cfg(target_os = "linux")]
    fn update_linux_tooltip(&self, _tooltip: &str) -> Result<()> {
        tracing::debug!("Updating Linux tray tooltip (stub)");
        Ok(())
    }
    
    #[cfg(target_os = "linux")]
    fn show_linux_tray(&self) -> Result<()> {
        tracing::debug!("Showing Linux tray icon (stub)");
        Ok(())
    }
    
    #[cfg(target_os = "linux")]
    fn hide_linux_tray(&self) -> Result<()> {
        tracing::debug!("Hiding Linux tray icon (stub)");
        Ok(())
    }
    
    #[cfg(target_os = "macos")]
    fn update_macos_menu(&self) -> Result<()> {
        tracing::debug!("Updating macOS tray menu (stub)");
        Ok(())
    }
    
    #[cfg(target_os = "macos")]
    fn update_macos_tooltip(&self, _tooltip: &str) -> Result<()> {
        tracing::debug!("Updating macOS tray tooltip (stub)");
        Ok(())
    }
    
    #[cfg(target_os = "macos")]
    fn show_macos_tray(&self) -> Result<()> {
        tracing::debug!("Showing macOS tray icon (stub)");
        Ok(())
    }
    
    #[cfg(target_os = "macos")]
    fn hide_macos_tray(&self) -> Result<()> {
        tracing::debug!("Hiding macOS tray icon (stub)");
        Ok(())
    }
    
    /// Shutdown the system tray
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down cross-platform system tray");
        self.hide().await?;
        self.initialized.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}

impl Default for CrossPlatformTrayManager {
    fn default() -> Self {
        Self::new("Web Page Manager")
    }
}


// ============================================================================
// Unified System Integration Service
// ============================================================================

/// Unified system integration service
/// 
/// This service combines hotkey management, notifications, and system tray
/// functionality into a single, easy-to-use interface.
/// 
/// # Example
/// 
/// ```rust,ignore
/// use ui_manager::system_integration::SystemIntegrationService;
/// 
/// let service = SystemIntegrationService::new("Web Page Manager");
/// service.initialize().await?;
/// 
/// // Register hotkeys
/// service.register_default_hotkeys().await?;
/// 
/// // Show notification
/// service.show_notification("Hello", "World").await?;
/// 
/// // Show system tray
/// service.show_tray().await?;
/// ```
pub struct SystemIntegrationService {
    hotkey_manager: CrossPlatformHotkeyManager,
    notification_manager: CrossPlatformNotificationManager,
    tray_manager: CrossPlatformTrayManager,
    initialized: std::sync::atomic::AtomicBool,
}

impl SystemIntegrationService {
    /// Create a new system integration service
    pub fn new(app_name: impl Into<String>) -> Self {
        let app_name = app_name.into();
        Self {
            hotkey_manager: CrossPlatformHotkeyManager::new(),
            notification_manager: CrossPlatformNotificationManager::new(&app_name),
            tray_manager: CrossPlatformTrayManager::new(&app_name),
            initialized: std::sync::atomic::AtomicBool::new(false),
        }
    }
    
    /// Create a new system integration service with an icon
    pub fn with_icon(app_name: impl Into<String>, icon_path: impl Into<String>) -> Self {
        let app_name = app_name.into();
        let icon_path = icon_path.into();
        Self {
            hotkey_manager: CrossPlatformHotkeyManager::new(),
            notification_manager: CrossPlatformNotificationManager::with_icon(&app_name, &icon_path),
            tray_manager: CrossPlatformTrayManager::with_icon(&app_name, &icon_path),
            initialized: std::sync::atomic::AtomicBool::new(false),
        }
    }
    
    /// Initialize all system integration components
    pub async fn initialize(&self) -> Result<()> {
        tracing::info!("Initializing system integration service");
        
        self.hotkey_manager.initialize().await?;
        self.notification_manager.initialize().await?;
        self.tray_manager.initialize().await?;
        
        self.initialized.store(true, std::sync::atomic::Ordering::Relaxed);
        tracing::info!("System integration service initialized");
        Ok(())
    }
    
    /// Check if the service is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    // ========================================================================
    // Hotkey Methods
    // ========================================================================
    
    /// Register a hotkey
    pub async fn register_hotkey(
        &self,
        hotkey: &Hotkey,
        callback: Arc<dyn HotkeyCallback>,
    ) -> Result<()> {
        self.hotkey_manager.register_hotkey(hotkey, callback).await
    }
    
    /// Register default hotkeys for the application
    pub async fn register_default_hotkeys<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        let callback = Arc::new(FnHotkeyCallback::new(callback));
        
        // Quick search: Ctrl+Shift+F
        let quick_search = Hotkey {
            id: "quick_search".to_string(),
            key_combination: "Ctrl+Shift+F".to_string(),
            action: "quick_search".to_string(),
            description: "打开快速搜索".to_string(),
        };
        self.hotkey_manager.register_hotkey(&quick_search, callback.clone()).await?;
        
        // Show window: Ctrl+Shift+W
        let show_window = Hotkey {
            id: "show_window".to_string(),
            key_combination: "Ctrl+Shift+W".to_string(),
            action: "show_window".to_string(),
            description: "显示主窗口".to_string(),
        };
        self.hotkey_manager.register_hotkey(&show_window, callback.clone()).await?;
        
        // New tab: Ctrl+Shift+T
        let new_tab = Hotkey {
            id: "new_tab".to_string(),
            key_combination: "Ctrl+Shift+T".to_string(),
            action: "new_tab".to_string(),
            description: "在默认浏览器中打开新标签页".to_string(),
        };
        self.hotkey_manager.register_hotkey(&new_tab, callback).await?;
        
        tracing::info!("Default hotkeys registered");
        Ok(())
    }
    
    /// Unregister a hotkey
    pub async fn unregister_hotkey(&self, hotkey_id: &str) -> Result<()> {
        self.hotkey_manager.unregister_hotkey(hotkey_id).await
    }
    
    /// Unregister all hotkeys
    pub async fn unregister_all_hotkeys(&self) -> Result<()> {
        self.hotkey_manager.unregister_all().await
    }
    
    /// Get registered hotkeys
    pub async fn get_registered_hotkeys(&self) -> Vec<HotkeyRegistration> {
        self.hotkey_manager.get_registered_hotkeys().await
    }
    
    // ========================================================================
    // Notification Methods
    // ========================================================================
    
    /// Show a notification
    pub async fn show_notification(&self, config: &NotificationConfig) -> Result<String> {
        self.notification_manager.show_notification(config).await
    }
    
    /// Show a simple notification
    pub async fn show_simple_notification(&self, message: impl Into<String>) -> Result<String> {
        self.notification_manager.show_simple(message).await
    }
    
    /// Show a notification with title
    pub async fn show_notification_with_title(
        &self,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Result<String> {
        self.notification_manager.show_with_title(title, message).await
    }
    
    /// Show tab activity notification
    pub async fn notify_tab_activity(&self, browser_name: &str, tab_count: usize) -> Result<String> {
        self.notification_manager.show_tab_activity(browser_name, tab_count).await
    }
    
    /// Show bookmark sync notification
    pub async fn notify_bookmark_sync(&self, synced_count: usize) -> Result<String> {
        self.notification_manager.show_bookmark_sync(synced_count).await
    }
    
    /// Show analysis complete notification
    pub async fn notify_analysis_complete(&self, analyzed_count: usize) -> Result<String> {
        self.notification_manager.show_analysis_complete(analyzed_count).await
    }
    
    /// Show duplicates found notification
    pub async fn notify_duplicates_found(&self, duplicate_count: usize) -> Result<String> {
        self.notification_manager.show_duplicates_found(duplicate_count).await
    }
    
    // ========================================================================
    // System Tray Methods
    // ========================================================================
    
    /// Show the system tray icon
    pub async fn show_tray(&self) -> Result<()> {
        self.tray_manager.show().await
    }
    
    /// Hide the system tray icon
    pub async fn hide_tray(&self) -> Result<()> {
        self.tray_manager.hide().await
    }
    
    /// Set the tray tooltip
    pub async fn set_tray_tooltip(&self, tooltip: impl Into<String>) -> Result<()> {
        self.tray_manager.set_tooltip(tooltip).await
    }
    
    /// Set the tray activity badge
    pub async fn set_tray_badge(&self, count: usize) -> Result<()> {
        self.tray_manager.set_activity_badge(count).await
    }
    
    /// Set the tray menu
    pub async fn set_tray_menu(&self, items: Vec<TrayMenuItem>) -> Result<()> {
        self.tray_manager.set_menu(items).await
    }
    
    /// Set the tray event handler
    pub async fn set_tray_event_handler(&self, handler: Arc<dyn TrayEventHandler>) -> Result<()> {
        self.tray_manager.set_event_handler(handler).await
    }
    
    // ========================================================================
    // Lifecycle Methods
    // ========================================================================
    
    /// Shutdown all system integration components
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down system integration service");
        
        self.hotkey_manager.shutdown().await?;
        self.notification_manager.shutdown().await?;
        self.tray_manager.shutdown().await?;
        
        self.initialized.store(false, std::sync::atomic::Ordering::Relaxed);
        tracing::info!("System integration service shut down");
        Ok(())
    }
    
    /// Get the hotkey manager
    pub fn hotkey_manager(&self) -> &CrossPlatformHotkeyManager {
        &self.hotkey_manager
    }
    
    /// Get the notification manager
    pub fn notification_manager(&self) -> &CrossPlatformNotificationManager {
        &self.notification_manager
    }
    
    /// Get the tray manager
    pub fn tray_manager(&self) -> &CrossPlatformTrayManager {
        &self.tray_manager
    }
}

impl Default for SystemIntegrationService {
    fn default() -> Self {
        Self::new("Web Page Manager")
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_hotkey_manager_creation() {
        let manager = CrossPlatformHotkeyManager::new();
        assert!(!manager.initialized.load(std::sync::atomic::Ordering::Relaxed));
    }
    
    #[tokio::test]
    async fn test_hotkey_manager_initialize() {
        let manager = CrossPlatformHotkeyManager::new();
        let result = manager.initialize().await;
        assert!(result.is_ok());
        assert!(manager.initialized.load(std::sync::atomic::Ordering::Relaxed));
    }
    
    #[tokio::test]
    async fn test_parse_key_combination() {
        // Test valid combinations
        let result = CrossPlatformHotkeyManager::parse_key_combination("Ctrl+Shift+F");
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.modifiers.len(), 2);
        assert!(parsed.modifiers.contains(&KeyModifier::Ctrl));
        assert!(parsed.modifiers.contains(&KeyModifier::Shift));
        assert_eq!(parsed.key, "F");
        
        // Test with Alt
        let result = CrossPlatformHotkeyManager::parse_key_combination("Alt+Tab");
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(parsed.modifiers.contains(&KeyModifier::Alt));
        assert_eq!(parsed.key, "Tab");
        
        // Test with Meta/Win
        let result = CrossPlatformHotkeyManager::parse_key_combination("Win+E");
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(parsed.modifiers.contains(&KeyModifier::Meta));
        assert_eq!(parsed.key, "E");
    }
    
    #[tokio::test]
    async fn test_notification_manager_creation() {
        let manager = CrossPlatformNotificationManager::new("Test App");
        assert!(!manager.initialized.load(std::sync::atomic::Ordering::Relaxed));
    }
    
    #[tokio::test]
    async fn test_notification_manager_initialize() {
        let manager = CrossPlatformNotificationManager::new("Test App");
        let result = manager.initialize().await;
        assert!(result.is_ok());
        assert!(manager.initialized.load(std::sync::atomic::Ordering::Relaxed));
    }
    
    #[tokio::test]
    async fn test_notification_manager_show() {
        let manager = CrossPlatformNotificationManager::new("Test App");
        manager.initialize().await.unwrap();
        
        let result = manager.show_simple("Test message").await;
        assert!(result.is_ok());
        
        let history = manager.get_history().await;
        assert_eq!(history.len(), 1);
    }
    
    #[tokio::test]
    async fn test_tray_manager_creation() {
        let manager = CrossPlatformTrayManager::new("Test App");
        assert!(!manager.initialized.load(std::sync::atomic::Ordering::Relaxed));
    }
    
    #[tokio::test]
    async fn test_tray_manager_initialize() {
        let manager = CrossPlatformTrayManager::new("Test App");
        let result = manager.initialize().await;
        assert!(result.is_ok());
        assert!(manager.initialized.load(std::sync::atomic::Ordering::Relaxed));
    }
    
    #[tokio::test]
    async fn test_tray_menu_item_creation() {
        let item = TrayMenuItem::item("test", "Test Item");
        match item {
            TrayMenuItem::Item { id, label, enabled, checked } => {
                assert_eq!(id, "test");
                assert_eq!(label, "Test Item");
                assert!(enabled);
                assert!(checked.is_none());
            }
            _ => panic!("Expected Item variant"),
        }
        
        let separator = TrayMenuItem::separator();
        assert!(matches!(separator, TrayMenuItem::Separator));
        
        let checkable = TrayMenuItem::checkable_item("check", "Checkable", true);
        match checkable {
            TrayMenuItem::Item { checked, .. } => {
                assert_eq!(checked, Some(true));
            }
            _ => panic!("Expected Item variant"),
        }
    }
    
    #[tokio::test]
    async fn test_tray_manager_tooltip() {
        let manager = CrossPlatformTrayManager::new("Test App");
        manager.initialize().await.unwrap();
        
        manager.set_tooltip("New tooltip").await.unwrap();
        let tooltip = manager.get_tooltip().await;
        assert_eq!(tooltip, "New tooltip");
        
        manager.set_activity_badge(5).await.unwrap();
        let tooltip = manager.get_tooltip().await;
        assert!(tooltip.contains("5"));
    }
    
    #[tokio::test]
    async fn test_system_integration_service() {
        let service = SystemIntegrationService::new("Test App");
        assert!(!service.is_initialized());
        
        let result = service.initialize().await;
        assert!(result.is_ok());
        assert!(service.is_initialized());
        
        // Test notification
        let result = service.show_simple_notification("Test").await;
        assert!(result.is_ok());
        
        // Test shutdown
        let result = service.shutdown().await;
        assert!(result.is_ok());
        assert!(!service.is_initialized());
    }
    
    #[tokio::test]
    async fn test_hotkey_registration() {
        let manager = CrossPlatformHotkeyManager::new();
        manager.initialize().await.unwrap();
        
        let hotkey = Hotkey {
            id: "test_hotkey".to_string(),
            key_combination: "Ctrl+Shift+T".to_string(),
            action: "test_action".to_string(),
            description: "Test hotkey".to_string(),
        };
        
        let callback = Arc::new(FnHotkeyCallback::new(|_| {}));
        let result = manager.register_hotkey(&hotkey, callback).await;
        assert!(result.is_ok());
        
        assert!(manager.is_hotkey_registered("test_hotkey").await);
        
        let registered = manager.get_registered_hotkeys().await;
        assert_eq!(registered.len(), 1);
        assert_eq!(registered[0].id, "test_hotkey");
        
        // Unregister
        let result = manager.unregister_hotkey("test_hotkey").await;
        assert!(result.is_ok());
        assert!(!manager.is_hotkey_registered("test_hotkey").await);
    }
}
