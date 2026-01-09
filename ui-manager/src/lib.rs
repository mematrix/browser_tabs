use web_page_manager_core::*;

pub mod traits;
pub mod flutter;
pub mod native;
pub mod system_integration;
pub mod performance_monitor;

pub use system_integration::{
    CrossPlatformHotkeyManager,
    CrossPlatformNotificationManager,
    CrossPlatformTrayManager,
    SystemIntegrationService,
    HotkeyRegistration,
    HotkeyCallback,
    FnHotkeyCallback,
    NotificationRecord,
    NotificationStatus,
    NotificationCallback,
    TrayMenuItem,
    TrayEvent,
    TrayEventHandler,
    FnTrayEventHandler,
    ParsedKeyCombination,
    KeyModifier,
};

pub use performance_monitor::{
    PerformanceMonitor,
    PerformanceMetrics,
    PerformanceSummary,
    ResourceConfig,
    ResourceLevel,
    ProcessingPriority,
    AppSettings,
    SettingsManager,
    ThemeMode,
};

pub use traits::*;

/// UI Manager factory for creating platform-specific implementations
/// 
/// This factory provides compile-time UI framework selection based on
/// Cargo features. The available features are:
/// 
/// - `flutter-ui`: Cross-platform Flutter UI (default)
/// - `winui-ui`: Windows-native WinUI 3 (Windows only)
/// - `gtk-ui`: GTK 4 UI (primarily Linux)
/// - `qt-ui`: Qt UI (cross-platform)
/// 
/// # Compile-time Selection
/// 
/// The UI framework is selected at compile time. Only one framework
/// should be enabled at a time for optimal binary size.
/// 
/// ```toml
/// [features]
/// default = ["flutter-ui"]
/// flutter-ui = []
/// winui-ui = []
/// gtk-ui = []
/// qt-ui = []
/// ```
/// 
/// # Example
/// 
/// ```rust,ignore
/// use ui_manager::UIManagerFactory;
/// 
/// // Create UI manager based on compile-time feature
/// let ui = UIManagerFactory::create();
/// 
/// // Or create a specific implementation
/// let flutter_ui = UIManagerFactory::create_specific(UIFramework::Flutter)?;
/// ```
pub struct UIManagerFactory;

impl UIManagerFactory {
    /// Create a UI manager based on compile-time feature selection
    /// 
    /// This method returns the UI manager implementation that was selected
    /// at compile time via Cargo features. If no specific feature is enabled,
    /// it defaults to Flutter.
    /// 
    /// # Returns
    /// 
    /// A boxed UIManager implementation wrapped in an adapter for logging
    /// and state tracking.
    pub fn create() -> Box<dyn UIManager> {
        #[cfg(feature = "flutter-ui")]
        {
            return Box::new(UIManagerAdapter::new(
                flutter::FlutterUIManager::new(),
                "Flutter"
            ));
        }
        
        #[cfg(all(feature = "winui-ui", target_os = "windows"))]
        {
            return Box::new(UIManagerAdapter::new(
                native::winui::WinUIManager::new(),
                "WinUI"
            ));
        }
        
        #[cfg(all(feature = "gtk-ui", target_os = "linux"))]
        {
            return Box::new(UIManagerAdapter::new(
                native::gtk::GTKUIManager::new(),
                "GTK"
            ));
        }
        
        #[cfg(feature = "qt-ui")]
        {
            return Box::new(UIManagerAdapter::new(
                native::qt::QtUIManager::new(),
                "Qt"
            ));
        }
        
        #[cfg(not(any(
            feature = "flutter-ui",
            all(feature = "winui-ui", target_os = "windows"),
            all(feature = "gtk-ui", target_os = "linux"),
            feature = "qt-ui"
        )))]
        {
            // Default to Flutter if no specific UI is selected
            Box::new(UIManagerAdapter::new(
                flutter::FlutterUIManager::new(),
                "Flutter (default)"
            ))
        }
    }
    
    /// Create a specific UI manager implementation
    /// 
    /// This method allows runtime selection of a UI framework, though
    /// the framework must still be compiled in via features.
    /// 
    /// # Arguments
    /// 
    /// * `framework` - The UI framework to create
    /// 
    /// # Returns
    /// 
    /// A boxed UIManager implementation, or an error if the framework
    /// is not available on the current platform.
    /// 
    /// # Errors
    /// 
    /// Returns `UIError::PlatformNotSupported` if the requested framework
    /// is not available on the current platform.
    pub fn create_specific(framework: UIFramework) -> Result<Box<dyn UIManager>> {
        match framework {
            UIFramework::Flutter => Ok(Box::new(UIManagerAdapter::new(
                flutter::FlutterUIManager::new(),
                "Flutter"
            ))),
            
            #[cfg(target_os = "windows")]
            UIFramework::WinUI => Ok(Box::new(UIManagerAdapter::new(
                native::winui::WinUIManager::new(),
                "WinUI"
            ))),
            
            #[cfg(target_os = "linux")]
            UIFramework::GTK => Ok(Box::new(UIManagerAdapter::new(
                native::gtk::GTKUIManager::new(),
                "GTK"
            ))),
            
            UIFramework::Qt => Ok(Box::new(UIManagerAdapter::new(
                native::qt::QtUIManager::new(),
                "Qt"
            ))),
            
            #[cfg(not(target_os = "windows"))]
            UIFramework::WinUI => Err(WebPageManagerError::UI {
                source: UIError::PlatformNotSupported {
                    platform: "WinUI is only supported on Windows".to_string(),
                },
            }),
            
            #[cfg(not(target_os = "linux"))]
            UIFramework::GTK => Err(WebPageManagerError::UI {
                source: UIError::PlatformNotSupported {
                    platform: "GTK is primarily supported on Linux".to_string(),
                },
            }),
        }
    }
    
    /// Get a list of available UI frameworks on the current platform
    /// 
    /// This method returns all UI frameworks that are available on the
    /// current platform, regardless of which features are enabled.
    pub fn available_frameworks() -> Vec<UIFramework> {
        let mut frameworks = vec![UIFramework::Flutter, UIFramework::Qt];
        
        #[cfg(target_os = "windows")]
        frameworks.push(UIFramework::WinUI);
        
        #[cfg(target_os = "linux")]
        frameworks.push(UIFramework::GTK);
        
        frameworks
    }
    
    /// Get the default UI framework for the current platform
    /// 
    /// Returns the recommended UI framework based on the current platform:
    /// - Windows: WinUI (if available) or Flutter
    /// - Linux: GTK (if available) or Flutter
    /// - Other: Flutter
    pub fn default_framework() -> UIFramework {
        #[cfg(all(feature = "winui-ui", target_os = "windows"))]
        return UIFramework::WinUI;
        
        #[cfg(all(feature = "gtk-ui", target_os = "linux"))]
        return UIFramework::GTK;
        
        #[cfg(not(any(
            all(feature = "winui-ui", target_os = "windows"),
            all(feature = "gtk-ui", target_os = "linux")
        )))]
        UIFramework::Flutter
    }
    
    /// Check if a specific UI framework is available
    /// 
    /// Returns true if the framework can be used on the current platform.
    pub fn is_framework_available(framework: UIFramework) -> bool {
        match framework {
            UIFramework::Flutter => true,
            UIFramework::Qt => true,
            UIFramework::WinUI => cfg!(target_os = "windows"),
            UIFramework::GTK => cfg!(target_os = "linux") || cfg!(target_os = "macos"),
        }
    }
}

/// Get information about the current UI configuration
#[derive(Debug, Clone)]
pub struct UIConfiguration {
    /// The selected UI framework
    pub framework: UIFramework,
    /// Whether the framework is available on this platform
    pub is_available: bool,
    /// The framework's capabilities
    pub capabilities: UICapabilities,
    /// Platform name
    pub platform: String,
}

impl UIConfiguration {
    /// Get the current UI configuration
    pub fn current() -> Self {
        let ui = UIManagerFactory::create();
        Self {
            framework: ui.framework_type(),
            is_available: ui.is_available(),
            capabilities: ui.get_capabilities(),
            platform: Self::current_platform().to_string(),
        }
    }
    
    /// Get the current platform name
    pub fn current_platform() -> &'static str {
        #[cfg(target_os = "windows")]
        return "Windows";
        
        #[cfg(target_os = "linux")]
        return "Linux";
        
        #[cfg(target_os = "macos")]
        return "macOS";
        
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        "Unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_factory_create() {
        let ui = UIManagerFactory::create();
        assert!(ui.is_available());
    }
    
    #[test]
    fn test_factory_create_flutter() {
        let ui = UIManagerFactory::create_specific(UIFramework::Flutter);
        assert!(ui.is_ok());
        let ui = ui.unwrap();
        assert_eq!(ui.framework_type(), UIFramework::Flutter);
    }
    
    #[test]
    fn test_available_frameworks() {
        let frameworks = UIManagerFactory::available_frameworks();
        assert!(frameworks.contains(&UIFramework::Flutter));
        assert!(frameworks.contains(&UIFramework::Qt));
    }
    
    #[test]
    fn test_is_framework_available() {
        assert!(UIManagerFactory::is_framework_available(UIFramework::Flutter));
        assert!(UIManagerFactory::is_framework_available(UIFramework::Qt));
    }
    
    #[test]
    fn test_ui_configuration() {
        let config = UIConfiguration::current();
        assert!(config.is_available);
    }
}