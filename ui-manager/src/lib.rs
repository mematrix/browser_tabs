use web_page_manager_core::*;
use std::future::Future;
use std::pin::Pin;

pub mod traits;
pub mod flutter;
pub mod native;

pub use traits::*;

/// UI Manager factory for creating platform-specific implementations
pub struct UIManagerFactory;

impl UIManagerFactory {
    /// Create a UI manager based on compile-time feature selection
    pub fn create() -> Box<dyn UIManager> {
        #[cfg(feature = "flutter-ui")]
        {
            Box::new(flutter::FlutterUIManager::new())
        }
        
        #[cfg(all(feature = "winui-ui", target_os = "windows"))]
        {
            Box::new(native::winui::WinUIManager::new())
        }
        
        #[cfg(all(feature = "gtk-ui", target_os = "linux"))]
        {
            Box::new(native::gtk::GTKUIManager::new())
        }
        
        #[cfg(feature = "qt-ui")]
        {
            Box::new(native::qt::QtUIManager::new())
        }
        
        #[cfg(not(any(
            feature = "flutter-ui",
            all(feature = "winui-ui", target_os = "windows"),
            all(feature = "gtk-ui", target_os = "linux"),
            feature = "qt-ui"
        )))]
        {
            // Default to Flutter if no specific UI is selected
            Box::new(flutter::FlutterUIManager::new())
        }
    }
    
    /// Create a specific UI manager implementation
    pub fn create_specific(framework: UIFramework) -> Result<Box<dyn UIManager>> {
        match framework {
            UIFramework::Flutter => Ok(Box::new(flutter::FlutterUIManager::new())),
            
            #[cfg(target_os = "windows")]
            UIFramework::WinUI => Ok(Box::new(native::winui::WinUIManager::new())),
            
            #[cfg(target_os = "linux")]
            UIFramework::GTK => Ok(Box::new(native::gtk::GTKUIManager::new())),
            
            UIFramework::Qt => Ok(Box::new(native::qt::QtUIManager::new())),
            
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
}