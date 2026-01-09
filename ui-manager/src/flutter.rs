use crate::traits::*;
use web_page_manager_core::*;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Flutter method channel names for Rust-Dart communication
pub mod channels {
    pub const MAIN_CHANNEL: &str = "web_page_manager/main";
    pub const DATA_CHANNEL: &str = "web_page_manager/data";
    pub const NOTIFICATION_CHANNEL: &str = "web_page_manager/notification";
    pub const HOTKEY_CHANNEL: &str = "web_page_manager/hotkey";
    pub const TRAY_CHANNEL: &str = "web_page_manager/tray";
}

/// Flutter UI configuration
#[derive(Debug, Clone)]
pub struct FlutterUIConfig {
    /// Enable system tray integration
    pub enable_system_tray: bool,
    /// Enable global hotkeys
    pub enable_hotkeys: bool,
    /// Enable native notifications
    pub enable_notifications: bool,
    /// Window title
    pub window_title: String,
    /// Initial window width
    pub window_width: u32,
    /// Initial window height
    pub window_height: u32,
    /// Minimum window width
    pub min_window_width: u32,
    /// Minimum window height
    pub min_window_height: u32,
}

impl Default for FlutterUIConfig {
    fn default() -> Self {
        Self {
            enable_system_tray: true,
            enable_hotkeys: true,
            enable_notifications: true,
            window_title: "Web Page Manager".to_string(),
            window_width: 1200,
            window_height: 800,
            min_window_width: 800,
            min_window_height: 600,
        }
    }
}

/// State for the Flutter UI
#[derive(Debug, Default)]
struct FlutterUIState {
    /// Current UI data
    current_data: Option<UIData>,
    /// Registered hotkeys
    registered_hotkeys: Vec<Hotkey>,
    /// Whether window is visible
    window_visible: bool,
    /// Whether minimized to tray
    minimized_to_tray: bool,
}

/// Flutter UI Manager implementation
/// 
/// This manager handles the Rust side of the Flutter UI integration.
/// It communicates with the Flutter/Dart side via method channels
/// and manages the application state.
pub struct FlutterUIManager {
    initialized: std::sync::atomic::AtomicBool,
    config: FlutterUIConfig,
    state: Arc<RwLock<FlutterUIState>>,
}

impl FlutterUIManager {
    /// Create a new Flutter UI Manager with default configuration
    pub fn new() -> Self {
        Self::with_config(FlutterUIConfig::default())
    }
    
    /// Create a new Flutter UI Manager with custom configuration
    pub fn with_config(config: FlutterUIConfig) -> Self {
        Self {
            initialized: std::sync::atomic::AtomicBool::new(false),
            config,
            state: Arc::new(RwLock::new(FlutterUIState::default())),
        }
    }
    
    /// Get the current configuration
    pub fn config(&self) -> &FlutterUIConfig {
        &self.config
    }
    
    /// Check if the UI is currently visible
    pub async fn is_window_visible(&self) -> bool {
        self.state.read().await.window_visible
    }
    
    /// Check if minimized to tray
    pub async fn is_minimized_to_tray(&self) -> bool {
        self.state.read().await.minimized_to_tray
    }
    
    /// Get the current UI data
    pub async fn get_current_data(&self) -> Option<UIData> {
        self.state.read().await.current_data.clone()
    }
    
    /// Get registered hotkeys
    pub async fn get_registered_hotkeys(&self) -> Vec<Hotkey> {
        self.state.read().await.registered_hotkeys.clone()
    }
}

impl Default for FlutterUIManager {
    fn default() -> Self {
        Self::new()
    }
}

impl UIManager for FlutterUIManager {
    fn initialize(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            tracing::info!("Initializing Flutter UI Manager");
            tracing::info!("Window config: {}x{} (min: {}x{})",
                self.config.window_width,
                self.config.window_height,
                self.config.min_window_width,
                self.config.min_window_height
            );
            
            // Initialize state
            {
                let mut state = self.state.write().await;
                state.window_visible = false;
                state.minimized_to_tray = false;
            }
            
            // The actual Flutter engine initialization happens on the Dart side.
            // This Rust side sets up the method channel handlers and state.
            
            // Method channels to set up:
            // 1. Main channel - window management, lifecycle events
            // 2. Data channel - page/group data updates
            // 3. Notification channel - system notifications
            // 4. Hotkey channel - global hotkey registration
            // 5. Tray channel - system tray management
            
            tracing::debug!("Method channels configured:");
            tracing::debug!("  - {}", channels::MAIN_CHANNEL);
            tracing::debug!("  - {}", channels::DATA_CHANNEL);
            tracing::debug!("  - {}", channels::NOTIFICATION_CHANNEL);
            tracing::debug!("  - {}", channels::HOTKEY_CHANNEL);
            tracing::debug!("  - {}", channels::TRAY_CHANNEL);
            
            self.initialized.store(true, std::sync::atomic::Ordering::Relaxed);
            tracing::info!("Flutter UI Manager initialized successfully");
            Ok(())
        })
    }
    
    fn show_main_window(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            if !self.initialized.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(WebPageManagerError::UI {
                    source: UIError::NotInitialized,
                });
            }
            
            tracing::info!("Showing Flutter main window: {}", self.config.window_title);
            
            // Update state
            {
                let mut state = self.state.write().await;
                state.window_visible = true;
                state.minimized_to_tray = false;
            }
            
            // The actual window show is handled by Flutter's window_manager plugin
            // This would send a message through the method channel to show the window
            
            Ok(())
        })
    }
    
    fn show_notification(&self, message: &str) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let message = message.to_string();
        let enabled = self.config.enable_notifications;
        Box::pin(async move {
            if !enabled {
                tracing::debug!("Notifications disabled, skipping: {}", message);
                return Ok(());
            }
            
            tracing::info!("Showing notification: {}", message);
            
            // The actual notification is shown via Flutter's local_notifier plugin
            // This would send a message through the notification channel
            
            Ok(())
        })
    }
    
    fn register_global_hotkeys(&self, hotkeys: Vec<Hotkey>) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let enabled = self.config.enable_hotkeys;
        let state = self.state.clone();
        Box::pin(async move {
            if !enabled {
                tracing::debug!("Hotkeys disabled, skipping registration of {} hotkeys", hotkeys.len());
                return Ok(());
            }
            
            tracing::info!("Registering {} global hotkeys", hotkeys.len());
            
            for hotkey in &hotkeys {
                tracing::debug!("  - {} -> {} ({})", 
                    hotkey.key_combination, 
                    hotkey.action,
                    hotkey.description
                );
            }
            
            // Store registered hotkeys
            {
                let mut s = state.write().await;
                s.registered_hotkeys = hotkeys;
            }
            
            // The actual hotkey registration is done via Flutter's hotkey_manager plugin
            // This would send a message through the hotkey channel
            
            Ok(())
        })
    }
    
    fn minimize_to_tray(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let enabled = self.config.enable_system_tray;
        let state = self.state.clone();
        Box::pin(async move {
            if !enabled {
                tracing::debug!("System tray disabled, cannot minimize to tray");
                return Ok(());
            }
            
            tracing::info!("Minimizing to system tray");
            
            // Update state
            {
                let mut s = state.write().await;
                s.window_visible = false;
                s.minimized_to_tray = true;
            }
            
            // The actual minimize is done via Flutter's system_tray and window_manager plugins
            // This would send a message through the tray channel
            
            Ok(())
        })
    }
    
    fn update_ui_data(&self, data: UIData) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let state = self.state.clone();
        Box::pin(async move {
            tracing::info!("Updating UI data: {} pages, {} groups, {} browsers", 
                data.pages.len(), 
                data.groups.len(),
                data.active_browser_count
            );
            
            // Store current data
            {
                let mut s = state.write().await;
                s.current_data = Some(data);
            }
            
            // The actual UI update is done by sending data through the data channel
            // Flutter's Provider/Riverpod will handle the state update and UI rebuild
            
            Ok(())
        })
    }
    
    fn shutdown(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let state = self.state.clone();
        Box::pin(async move {
            tracing::info!("Shutting down Flutter UI Manager");
            
            // Clear state
            {
                let mut s = state.write().await;
                s.current_data = None;
                s.registered_hotkeys.clear();
                s.window_visible = false;
                s.minimized_to_tray = false;
            }
            
            // The actual cleanup is done on the Flutter side
            // - Dispose method channels
            // - Unregister hotkeys
            // - Remove system tray
            // - Save UI state to preferences
            
            self.initialized.store(false, std::sync::atomic::Ordering::Relaxed);
            tracing::info!("Flutter UI Manager shut down successfully");
            Ok(())
        })
    }
    
    fn framework_type(&self) -> UIFramework {
        UIFramework::Flutter
    }
    
    fn is_available(&self) -> bool {
        // Flutter is available on Windows, Linux, and macOS
        cfg!(any(target_os = "windows", target_os = "linux", target_os = "macos"))
    }
    
    fn get_capabilities(&self) -> UICapabilities {
        UICapabilities {
            supports_system_tray: self.config.enable_system_tray,
            supports_global_hotkeys: self.config.enable_hotkeys,
            supports_native_notifications: self.config.enable_notifications,
            supports_jump_lists: false, // Not directly supported in Flutter
            supports_live_tiles: false, // Not directly supported in Flutter
            supports_dark_mode: true,
            supports_transparency: true,
            cross_platform: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_flutter_ui_manager_creation() {
        let manager = FlutterUIManager::new();
        assert!(!manager.initialized.load(std::sync::atomic::Ordering::Relaxed));
        assert_eq!(manager.framework_type(), UIFramework::Flutter);
    }
    
    #[tokio::test]
    async fn test_flutter_ui_manager_with_config() {
        let config = FlutterUIConfig {
            enable_system_tray: false,
            enable_hotkeys: false,
            enable_notifications: false,
            window_title: "Test Window".to_string(),
            window_width: 800,
            window_height: 600,
            min_window_width: 400,
            min_window_height: 300,
        };
        
        let manager = FlutterUIManager::with_config(config);
        assert_eq!(manager.config().window_title, "Test Window");
        assert_eq!(manager.config().window_width, 800);
        assert!(!manager.config().enable_system_tray);
    }
    
    #[tokio::test]
    async fn test_flutter_ui_manager_initialize() {
        let manager = FlutterUIManager::new();
        let result = manager.initialize().await;
        assert!(result.is_ok());
        assert!(manager.initialized.load(std::sync::atomic::Ordering::Relaxed));
    }
    
    #[tokio::test]
    async fn test_flutter_ui_manager_capabilities() {
        let manager = FlutterUIManager::new();
        let caps = manager.get_capabilities();
        
        assert!(caps.supports_system_tray);
        assert!(caps.supports_global_hotkeys);
        assert!(caps.supports_native_notifications);
        assert!(caps.supports_dark_mode);
        assert!(caps.cross_platform);
        assert!(!caps.supports_jump_lists);
        assert!(!caps.supports_live_tiles);
    }
    
    #[tokio::test]
    async fn test_flutter_ui_manager_state() {
        let manager = FlutterUIManager::new();
        manager.initialize().await.unwrap();
        
        // Initially not visible
        assert!(!manager.is_window_visible().await);
        assert!(!manager.is_minimized_to_tray().await);
        
        // Show window
        manager.show_main_window().await.unwrap();
        assert!(manager.is_window_visible().await);
        
        // Minimize to tray
        manager.minimize_to_tray().await.unwrap();
        assert!(!manager.is_window_visible().await);
        assert!(manager.is_minimized_to_tray().await);
    }
}