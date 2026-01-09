use web_page_manager_core::*;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Unified UI Manager trait for different UI frameworks
/// 
/// This trait defines the common interface that all UI framework implementations
/// must provide. It ensures functional consistency across Flutter, WinUI, GTK,
/// and Qt implementations while allowing each framework to leverage its
/// platform-specific capabilities.
/// 
/// # Design Principles
/// 
/// 1. **Compile-time Selection**: UI framework is selected at compile time via
///    Cargo features (flutter-ui, winui-ui, gtk-ui, qt-ui)
/// 2. **Functional Consistency**: All implementations provide the same core
///    functionality regardless of the underlying framework
/// 3. **Platform Adaptation**: Each implementation can leverage platform-specific
///    features while maintaining the common interface
/// 4. **Async-First**: All operations are async to support non-blocking UI updates
pub trait UIManager: Send + Sync {
    /// Initialize the UI framework
    /// 
    /// This must be called before any other UI operations. It sets up the
    /// framework-specific resources, event loops, and communication channels.
    fn initialize(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// Show the main application window
    /// 
    /// Creates and displays the main window if not already visible.
    /// If minimized to tray, this will restore the window.
    fn show_main_window(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// Hide the main application window
    /// 
    /// Hides the window without minimizing to tray. The window can be
    /// shown again with `show_main_window()`.
    fn hide_main_window(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// Show a system notification
    /// 
    /// Displays a native system notification with the given message.
    /// The notification style depends on the platform and framework.
    fn show_notification(&self, notification: &NotificationConfig) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// Register global hotkeys
    /// 
    /// Registers system-wide hotkeys that work even when the application
    /// is not focused. Previously registered hotkeys are replaced.
    fn register_global_hotkeys(&self, hotkeys: Vec<Hotkey>) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// Unregister all global hotkeys
    /// 
    /// Removes all previously registered global hotkeys.
    fn unregister_global_hotkeys(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// Minimize application to system tray
    /// 
    /// Hides the main window and shows a tray icon. The application
    /// continues running in the background.
    fn minimize_to_tray(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// Restore application from system tray
    /// 
    /// Shows the main window and optionally removes the tray icon.
    fn restore_from_tray(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// Update UI with new data
    /// 
    /// Sends new data to the UI for display. The UI framework handles
    /// the actual rendering and state management.
    fn update_ui_data(&self, data: UIData) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// Set the UI theme
    /// 
    /// Changes the application theme (light, dark, or system default).
    fn set_theme(&self, theme: UITheme) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// Get the current UI theme
    fn get_theme(&self) -> Pin<Box<dyn Future<Output = UITheme> + Send + '_>>;
    
    /// Register an event handler for UI events
    /// 
    /// The handler will be called when UI events occur (window closed,
    /// hotkey pressed, etc.).
    fn set_event_handler(&self, handler: Arc<dyn UIEventHandler>) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// Shutdown the UI framework
    /// 
    /// Cleans up all resources, closes windows, and prepares for
    /// application exit.
    fn shutdown(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// Get the UI framework type
    fn framework_type(&self) -> UIFramework;
    
    /// Check if the UI framework is available on current platform
    fn is_available(&self) -> bool;
    
    /// Get platform-specific capabilities
    fn get_capabilities(&self) -> UICapabilities;
    
    /// Get the current UI state
    fn get_state(&self) -> Pin<Box<dyn Future<Output = UIState> + Send + '_>>;
}

/// UI framework capabilities
/// 
/// Describes what features are supported by a specific UI framework
/// implementation. This allows the application to adapt its behavior
/// based on available capabilities.
#[derive(Debug, Clone)]
pub struct UICapabilities {
    /// Whether the framework supports system tray integration
    pub supports_system_tray: bool,
    /// Whether the framework supports global hotkeys
    pub supports_global_hotkeys: bool,
    /// Whether the framework supports native system notifications
    pub supports_native_notifications: bool,
    /// Whether the framework supports Windows Jump Lists
    pub supports_jump_lists: bool,
    /// Whether the framework supports Windows Live Tiles
    pub supports_live_tiles: bool,
    /// Whether the framework supports dark mode
    pub supports_dark_mode: bool,
    /// Whether the framework supports window transparency
    pub supports_transparency: bool,
    /// Whether the framework is cross-platform
    pub cross_platform: bool,
    /// Whether the framework supports custom window decorations
    pub supports_custom_decorations: bool,
    /// Whether the framework supports drag and drop
    pub supports_drag_drop: bool,
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
            supports_custom_decorations: false,
            supports_drag_drop: false,
        }
    }
}

/// UI theme options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UITheme {
    /// Light theme
    Light,
    /// Dark theme
    Dark,
    /// Follow system theme
    #[default]
    System,
}

/// Current UI state
#[derive(Debug, Clone, Default)]
pub struct UIState {
    /// Whether the UI is initialized
    pub initialized: bool,
    /// Whether the main window is visible
    pub window_visible: bool,
    /// Whether the app is minimized to tray
    pub minimized_to_tray: bool,
    /// Current theme
    pub current_theme: UITheme,
    /// Number of registered hotkeys
    pub registered_hotkey_count: usize,
    /// Whether event handler is set
    pub has_event_handler: bool,
}

/// Notification configuration
#[derive(Debug, Clone)]
pub struct NotificationConfig {
    /// Notification title
    pub title: String,
    /// Notification message body
    pub message: String,
    /// Optional icon path or identifier
    pub icon: Option<String>,
    /// Notification urgency level
    pub urgency: NotificationUrgency,
    /// Optional action buttons
    pub actions: Vec<NotificationAction>,
    /// Auto-dismiss timeout in milliseconds (None for persistent)
    pub timeout_ms: Option<u64>,
}

impl NotificationConfig {
    /// Create a simple notification with just a message
    pub fn simple(message: impl Into<String>) -> Self {
        Self {
            title: "Web Page Manager".to_string(),
            message: message.into(),
            icon: None,
            urgency: NotificationUrgency::Normal,
            actions: Vec::new(),
            timeout_ms: Some(5000),
        }
    }
    
    /// Create a notification with title and message
    pub fn with_title(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            icon: None,
            urgency: NotificationUrgency::Normal,
            actions: Vec::new(),
            timeout_ms: Some(5000),
        }
    }
}

/// Notification urgency level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NotificationUrgency {
    /// Low priority, may be hidden
    Low,
    /// Normal priority
    #[default]
    Normal,
    /// High priority, should be shown immediately
    High,
    /// Critical, requires user attention
    Critical,
}

/// Notification action button
#[derive(Debug, Clone)]
pub struct NotificationAction {
    /// Action identifier
    pub id: String,
    /// Button label
    pub label: String,
}

/// UI event types that can be handled
#[derive(Debug, Clone)]
pub enum UIEvent {
    /// Main window was closed
    WindowClosed,
    /// Main window was minimized
    WindowMinimized,
    /// Main window was restored from minimized state
    WindowRestored,
    /// Main window gained focus
    WindowFocused,
    /// Main window lost focus
    WindowBlurred,
    /// A global hotkey was pressed
    HotkeyPressed { hotkey_id: String },
    /// A notification was clicked
    NotificationClicked { notification_id: String },
    /// A notification action button was clicked
    NotificationActionClicked { notification_id: String, action_id: String },
    /// System tray icon was clicked
    TrayIconClicked,
    /// System tray icon was double-clicked
    TrayIconDoubleClicked,
    /// System tray context menu item was selected
    TrayMenuItemSelected { item_id: String },
    /// Data refresh was requested by the user
    DataRefreshRequested,
    /// Theme was changed (by system or user)
    ThemeChanged { new_theme: UITheme },
    /// Application is about to quit
    ApplicationQuitting,
}

/// UI event handler trait
/// 
/// Implement this trait to handle UI events from the framework.
pub trait UIEventHandler: Send + Sync {
    /// Handle a UI event
    /// 
    /// This method is called when a UI event occurs. The implementation
    /// should handle the event and return a result.
    fn handle_event(&self, event: UIEvent) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
}

/// A no-op event handler for testing or when events should be ignored
pub struct NoOpEventHandler;

impl UIEventHandler for NoOpEventHandler {
    fn handle_event(&self, _event: UIEvent) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
}

/// UI Manager adapter that provides a consistent interface across frameworks
/// 
/// This adapter wraps any UIManager implementation and provides additional
/// functionality like logging, error handling, and state tracking.
pub struct UIManagerAdapter<T: UIManager> {
    inner: T,
    name: String,
}

impl<T: UIManager> UIManagerAdapter<T> {
    /// Create a new adapter wrapping the given UI manager
    pub fn new(inner: T, name: impl Into<String>) -> Self {
        Self {
            inner,
            name: name.into(),
        }
    }
    
    /// Get a reference to the inner UI manager
    pub fn inner(&self) -> &T {
        &self.inner
    }
    
    /// Get the adapter name
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<T: UIManager> UIManager for UIManagerAdapter<T> {
    fn initialize(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let name = self.name.clone();
        Box::pin(async move {
            tracing::info!("[{}] Initializing UI manager", name);
            let result = self.inner.initialize().await;
            match &result {
                Ok(()) => tracing::info!("[{}] UI manager initialized successfully", name),
                Err(e) => tracing::error!("[{}] UI manager initialization failed: {}", name, e),
            }
            result
        })
    }
    
    fn show_main_window(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let name = self.name.clone();
        Box::pin(async move {
            tracing::debug!("[{}] Showing main window", name);
            self.inner.show_main_window().await
        })
    }
    
    fn hide_main_window(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let name = self.name.clone();
        Box::pin(async move {
            tracing::debug!("[{}] Hiding main window", name);
            self.inner.hide_main_window().await
        })
    }
    
    fn show_notification(&self, notification: &NotificationConfig) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let name = self.name.clone();
        let notification = notification.clone();
        Box::pin(async move {
            tracing::debug!("[{}] Showing notification: {}", name, notification.title);
            self.inner.show_notification(&notification).await
        })
    }
    
    fn register_global_hotkeys(&self, hotkeys: Vec<Hotkey>) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let name = self.name.clone();
        let count = hotkeys.len();
        Box::pin(async move {
            tracing::debug!("[{}] Registering {} global hotkeys", name, count);
            self.inner.register_global_hotkeys(hotkeys).await
        })
    }
    
    fn unregister_global_hotkeys(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let name = self.name.clone();
        Box::pin(async move {
            tracing::debug!("[{}] Unregistering global hotkeys", name);
            self.inner.unregister_global_hotkeys().await
        })
    }
    
    fn minimize_to_tray(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let name = self.name.clone();
        Box::pin(async move {
            tracing::debug!("[{}] Minimizing to tray", name);
            self.inner.minimize_to_tray().await
        })
    }
    
    fn restore_from_tray(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let name = self.name.clone();
        Box::pin(async move {
            tracing::debug!("[{}] Restoring from tray", name);
            self.inner.restore_from_tray().await
        })
    }
    
    fn update_ui_data(&self, data: UIData) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let name = self.name.clone();
        let page_count = data.pages.len();
        let group_count = data.groups.len();
        Box::pin(async move {
            tracing::debug!("[{}] Updating UI data: {} pages, {} groups", name, page_count, group_count);
            self.inner.update_ui_data(data).await
        })
    }
    
    fn set_theme(&self, theme: UITheme) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let name = self.name.clone();
        Box::pin(async move {
            tracing::debug!("[{}] Setting theme to {:?}", name, theme);
            self.inner.set_theme(theme).await
        })
    }
    
    fn get_theme(&self) -> Pin<Box<dyn Future<Output = UITheme> + Send + '_>> {
        Box::pin(async move {
            self.inner.get_theme().await
        })
    }
    
    fn set_event_handler(&self, handler: Arc<dyn UIEventHandler>) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let name = self.name.clone();
        Box::pin(async move {
            tracing::debug!("[{}] Setting event handler", name);
            self.inner.set_event_handler(handler).await
        })
    }
    
    fn shutdown(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let name = self.name.clone();
        Box::pin(async move {
            tracing::info!("[{}] Shutting down UI manager", name);
            let result = self.inner.shutdown().await;
            match &result {
                Ok(()) => tracing::info!("[{}] UI manager shut down successfully", name),
                Err(e) => tracing::error!("[{}] UI manager shutdown failed: {}", name, e),
            }
            result
        })
    }
    
    fn framework_type(&self) -> UIFramework {
        self.inner.framework_type()
    }
    
    fn is_available(&self) -> bool {
        self.inner.is_available()
    }
    
    fn get_capabilities(&self) -> UICapabilities {
        self.inner.get_capabilities()
    }
    
    fn get_state(&self) -> Pin<Box<dyn Future<Output = UIState> + Send + '_>> {
        Box::pin(async move {
            self.inner.get_state().await
        })
    }
}