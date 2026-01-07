use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;
use serde_json;
use crate::types::*;
use crate::errors::*;

/// FFI-safe result type
#[repr(C)]
pub struct FFIResult {
    pub success: bool,
    pub data: *mut c_char,
    pub error_message: *mut c_char,
}

impl FFIResult {
    pub fn success(data: String) -> Self {
        let data_cstring = CString::new(data).unwrap_or_default();
        Self {
            success: true,
            data: data_cstring.into_raw(),
            error_message: ptr::null_mut(),
        }
    }
    
    pub fn error(message: String) -> Self {
        let error_cstring = CString::new(message).unwrap_or_default();
        Self {
            success: false,
            data: ptr::null_mut(),
            error_message: error_cstring.into_raw(),
        }
    }
}

/// Free FFI result memory
#[no_mangle]
pub extern "C" fn free_ffi_result(result: FFIResult) {
    unsafe {
        if !result.data.is_null() {
            let _ = CString::from_raw(result.data);
        }
        if !result.error_message.is_null() {
            let _ = CString::from_raw(result.error_message);
        }
    }
}

/// Convert C string to Rust string
unsafe fn c_str_to_string(c_str: *const c_char) -> Result<String> {
    if c_str.is_null() {
        return Err(WebPageManagerError::System {
            source: SystemError::Configuration {
                details: "Null pointer passed to FFI".to_string(),
            },
        });
    }
    
    let c_str = CStr::from_ptr(c_str);
    c_str.to_str()
        .map(|s| s.to_string())
        .map_err(|_| WebPageManagerError::System {
            source: SystemError::Configuration {
                details: "Invalid UTF-8 in C string".to_string(),
            },
        })
}

/// Serialize data to JSON for FFI
fn serialize_to_json<T: serde::Serialize>(data: &T) -> Result<String> {
    serde_json::to_string(data).map_err(|e| WebPageManagerError::System {
        source: SystemError::Serialization { source: e },
    })
}

/// Deserialize JSON data from FFI
fn deserialize_from_json<T: for<'de> serde::Deserialize<'de>>(json: &str) -> Result<T> {
    serde_json::from_str(json).map_err(|e| WebPageManagerError::System {
        source: SystemError::Serialization { source: e },
    })
}

// Browser Connection FFI Functions

/// Initialize browser connector
#[no_mangle]
pub extern "C" fn init_browser_connector() -> FFIResult {
    match std::panic::catch_unwind(|| {
        // Initialize browser connector logic here
        "Browser connector initialized".to_string()
    }) {
        Ok(result) => FFIResult::success(result),
        Err(_) => FFIResult::error("Failed to initialize browser connector".to_string()),
    }
}

/// Detect available browsers
#[no_mangle]
pub extern "C" fn detect_browsers() -> FFIResult {
    match std::panic::catch_unwind(|| {
        // Mock browser detection for now
        let browsers = vec![
            BrowserInstance {
                browser_type: BrowserType::Chrome,
                version: "120.0.0.0".to_string(),
                process_id: 1234,
                debug_port: Some(9222),
                profile_path: Some("Default".to_string()),
            },
            BrowserInstance {
                browser_type: BrowserType::Firefox,
                version: "121.0.0".to_string(),
                process_id: 5678,
                debug_port: None,
                profile_path: Some("default-release".to_string()),
            },
        ];
        
        serialize_to_json(&browsers)
    }) {
        Ok(Ok(json)) => FFIResult::success(json),
        Ok(Err(e)) => FFIResult::error(format!("Serialization error: {}", e)),
        Err(_) => FFIResult::error("Failed to detect browsers".to_string()),
    }
}

/// Connect to a specific browser
#[no_mangle]
pub extern "C" fn connect_to_browser(browser_type_json: *const c_char) -> FFIResult {
    match std::panic::catch_unwind(|| {
        unsafe {
            let browser_type_str = c_str_to_string(browser_type_json)?;
            let browser_type: BrowserType = deserialize_from_json(&browser_type_str)?;
            
            // Mock connection for now
            let connection = BrowserConnection {
                browser_type,
                connection_id: uuid::Uuid::new_v4().to_string(),
                is_connected: true,
            };
            
            serialize_to_json(&connection)
        }
    }) {
        Ok(Ok(json)) => FFIResult::success(json),
        Ok(Err(e)) => FFIResult::error(format!("Connection error: {}", e)),
        Err(_) => FFIResult::error("Failed to connect to browser".to_string()),
    }
}

// AI Processing FFI Functions

/// Process page content with AI
#[no_mangle]
pub extern "C" fn process_page_content(content_json: *const c_char) -> FFIResult {
    match std::panic::catch_unwind(|| {
        unsafe {
            let content_str = c_str_to_string(content_json)?;
            let page_content: PageContent = deserialize_from_json(&content_str)?;
            
            // Mock AI processing for now
            let summary = ContentSummary {
                summary_text: format!("Summary of: {}", page_content.title),
                key_points: vec!["Key point 1".to_string(), "Key point 2".to_string()],
                content_type: ContentType::Article,
                language: "en".to_string(),
                reading_time_minutes: 5,
                confidence_score: 0.85,
                generated_at: chrono::Utc::now(),
            };
            
            serialize_to_json(&summary)
        }
    }) {
        Ok(Ok(json)) => FFIResult::success(json),
        Ok(Err(e)) => FFIResult::error(format!("AI processing error: {}", e)),
        Err(_) => FFIResult::error("Failed to process content".to_string()),
    }
}

// UI Manager FFI Functions

/// Initialize UI manager
#[no_mangle]
pub extern "C" fn init_ui_manager(framework: c_int) -> FFIResult {
    match std::panic::catch_unwind(|| {
        let ui_framework = match framework {
            0 => UIFramework::Flutter,
            1 => UIFramework::WinUI,
            2 => UIFramework::GTK,
            3 => UIFramework::Qt,
            _ => return Err(WebPageManagerError::UI {
                source: UIError::UnsupportedFramework {
                    framework: format!("Unknown framework ID: {}", framework),
                },
            }),
        };
        
        // Initialize UI manager logic here
        Ok(format!("UI manager initialized with {:?}", ui_framework))
    }) {
        Ok(Ok(result)) => FFIResult::success(result),
        Ok(Err(e)) => FFIResult::error(format!("UI initialization error: {}", e)),
        Err(_) => FFIResult::error("Failed to initialize UI manager".to_string()),
    }
}

/// Update UI data
#[no_mangle]
pub extern "C" fn update_ui_data(ui_data_json: *const c_char) -> FFIResult {
    match std::panic::catch_unwind(|| {
        unsafe {
            let ui_data_str = c_str_to_string(ui_data_json)?;
            let ui_data: UIData = deserialize_from_json(&ui_data_str)?;
            
            // Update UI logic here
            Ok(format!("UI updated with {} pages and {} groups", 
                   ui_data.pages.len(), ui_data.groups.len()))
        }
    }) {
        Ok(Ok(result)) => FFIResult::success(result),
        Ok(Err(e)) => FFIResult::error(format!("UI update error: {}", e)),
        Err(_) => FFIResult::error("Failed to update UI".to_string()),
    }
}