use web_page_manager_core::*;
use std::future::Future;
use std::pin::Pin;

/// Unified UI Manager trait for different UI frameworks
pub trait UIManager: Send + Sync {
    /// Initialize the UI framework
    fn initialize(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// Show the main application window
    fn show_main_window(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// Show a system notification
    fn show_notification(&self, message: &str) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// Register global hotkeys
    fn register_global_hotkeys(&self, hotkeys: Vec<Hotkey>) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// Minimize application to system tray
    fn minimize_to_tray(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// Update UI with new data
    fn update_ui_data(&self, data: UIData) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// Shutdown the UI framework
    fn shutdown(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// Get the UI framework type
    fn framework_type(&self) -> UIFramework;
    
    /// Check if the UI framework is available on current platform
    fn is_available(&self) -> bool;
    
    /// Get platform-specific capabilities
    fn get_capabilities(&self) -> UICapabilities;
}

/// UI framework capabilities
#[derive(Debug, Clone)]
pub struct UICapabilities {
    pub supports_system_tray: bool,
    pub supports_global_hotkeys: bool,
    pub supports_native_notifications: bool,
    pub supports_jump_lists: bool,
    pub supports_live_tiles: bool,
    pub supports_dark_mode: bool,
    pub supports_transparency: bool,
    pub cross_platform: bool,
}

impl Default for UICapabilities {
    fn default() -> Self {
        Self {
            supports_system_tray: false,
            supports_global_hotkeys: false,
            supports_native_notifications: false,
            supports_jump_lists: false,
            supports_live_tiles: false,
            supports_dark_mode: false,
            supports_transparency: false,
            cross_platform: false,
        }
    }
}

/// UI event types that can be handled
#[derive(Debug, Clone)]
pub enum UIEvent {
    WindowClosed,
    WindowMinimized,
    WindowRestored,
    HotkeyPressed { hotkey_id: String },
    NotificationClicked { notification_id: String },
    TrayIconClicked,
    DataRefreshRequested,
}

/// UI event handler trait
pub trait UIEventHandler: Send + Sync {
    fn handle_event(&self, event: UIEvent) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
}